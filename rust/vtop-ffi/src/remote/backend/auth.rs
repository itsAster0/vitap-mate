//! Pure state machine for this install's server identity (`auth:token` JWT + refresh lifecycle),
//! with no I/O and no clock: the caller passes `now_ms` and runs the returned [`Effect`]s.
//!
//! Two monotonic counters keep async and socket callbacks from clobbering newer state:
//! - `generation`: bumped whenever the identity itself changes (new proof/v2/demo chain, device
//!   removed, signed out, account switched, or server changed). A `device_removed` error carries
//!   the `generation` it ran under; stale ones are ignored.
//! - `jwt_epoch`: bumped on every JWT issue or invalidation (including routine refresh). A
//!   rejected JWT (`unauthenticated` / `AuthError`) carries the `jwt_epoch` it ran under; stale
//!   ones are ignored so concurrent callers don't double-refresh.

use tracing::info;

use super::identity::{KeptSignOut, RENEW_AHEAD_MS, Refresh};
use crate::device::EdgeError;
use crate::remote::server_code::ServerCode;

/// Normalises a server URL so a trailing `/` never hides a kept refresh token.
pub(crate) fn normalize_server(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}

/// Identifies the identity (`generation`) and specific JWT (`jwt_epoch`) a call or watch ran
/// under, so late errors from an earlier one never wipe newer state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Stamp {
    pub(crate) generation: u64,
    pub(crate) jwt_epoch: u64,
}

/// The live JWT currently set on the server socket. No `Debug`: it holds the token.
pub(crate) struct Jwt {
    pub(crate) server: String,
    pub(crate) regno: String,
    pub(crate) token: String,
    /// Unix ms.
    pub(crate) expires_at_ms: i64,
}

/// Which grant `auth:token` is called with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GrantKind {
    Demo,
    Refresh,
    V2,
    Proof,
}

impl GrantKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Demo => "demo",
            Self::Refresh => "refresh",
            Self::V2 => "v2",
            Self::Proof => "proof",
        }
    }
}

/// A concrete grant chosen for `auth:token`. No `Debug`: it holds secrets.
pub(crate) enum Grant {
    Demo,
    Refresh(Refresh),
    V2(String),
    Proof,
}

impl Grant {
    pub(crate) fn kind(&self) -> GrantKind {
        match self {
            Self::Demo => GrantKind::Demo,
            Self::Refresh(_) => GrantKind::Refresh,
            Self::V2(_) => GrantKind::V2,
            Self::Proof => GrantKind::Proof,
        }
    }
}

/// Why [`AuthMachine::next_step`] cannot obtain a JWT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Blocked {
    NotSignedIn,
    ConsentDenied,
}

/// What the auth driver should do next.
pub(crate) enum Step {
    /// Already holds a live JWT for this `(server, regno)`.
    Ready(Stamp),
    /// Exchange `grant` via `auth:token`.
    FetchToken(Grant),
    /// Cannot proceed without a student sign-in or user-initiated proof.
    Blocked(Blocked),
}

/// Side effect emitted by an [`AuthMachine`] transition; the driver executes these in order
/// outside the state lock.
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum Effect {
    /// Set or clear the JWT on the Convex WebSocket (the only path that changes socket auth).
    SetSocketAuth(Option<String>),
    /// Clear UDP's cached "JWT already sent" hash so the next UDP fetch sends the new JWT (or none).
    ResetUdpJwt,
    /// Save or clear the refresh token in `device.json`.
    SaveRefresh(Option<Refresh>),
    /// Keep a refresh token aside to sign out with later (`server_last_student.json`).
    KeepSignOut(KeptSignOut),
    /// Remove the legacy v2 account token file (`account_token.json`).
    ForgetV2Token,
    /// Remove `forwarding.json` because the server removed this install.
    ForgetForwarding,
}

impl std::fmt::Debug for Effect {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SetSocketAuth(Some(_)) => formatter.write_str("SetSocketAuth(Some(<redacted>))"),
            Self::SetSocketAuth(None) => formatter.write_str("SetSocketAuth(None)"),
            Self::ResetUdpJwt => formatter.write_str("ResetUdpJwt"),
            Self::SaveRefresh(Some(_)) => formatter.write_str("SaveRefresh(Some(<redacted>))"),
            Self::SaveRefresh(None) => formatter.write_str("SaveRefresh(None)"),
            Self::KeepSignOut(_) => formatter.write_str("KeepSignOut(<redacted>)"),
            Self::ForgetV2Token => formatter.write_str("ForgetV2Token"),
            Self::ForgetForwarding => formatter.write_str("ForgetForwarding"),
        }
    }
}

/// Inputs from disk and the signed-in session for [`AuthMachine::next_step`].
pub(crate) struct StepInput<'a> {
    pub(crate) server: &'a str,
    pub(crate) regno: Option<&'a str>,
    pub(crate) refresh: Option<&'a Refresh>,
    pub(crate) v2_token: Option<&'a str>,
    pub(crate) allow_proof: bool,
    pub(crate) now_ms: i64,
}

/// Single state of this install's server identity.
enum Phase {
    /// No live JWT on the socket.
    Unauthenticated,
    /// Live JWT set on the socket.
    Ready(Jwt),
}

impl Phase {
    fn name(&self) -> &'static str {
        match self {
            Self::Unauthenticated => "unauthenticated",
            Self::Ready(_) => "ready",
        }
    }
}

/// Deterministic state machine for the install's server identity.
pub(crate) struct AuthMachine {
    phase: Phase,
    generation: u64,
    jwt_epoch: u64,
}

impl Default for AuthMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthMachine {
    pub(crate) const fn new() -> Self {
        Self {
            phase: Phase::Unauthenticated,
            generation: 1,
            jwt_epoch: 0,
        }
    }

    pub(crate) fn stamp(&self) -> Stamp {
        Stamp {
            generation: self.generation,
            jwt_epoch: self.jwt_epoch,
        }
    }

    /// The JWT currently set on the socket, if any (for restoring socket auth after a
    /// temporary switch such as `send_kept_sign_out`).
    pub(crate) fn socket_jwt(&self) -> Option<String> {
        match &self.phase {
            Phase::Ready(jwt) => Some(jwt.token.clone()),
            Phase::Unauthenticated => None,
        }
    }

    /// The live JWT if it still has more than [`RENEW_AHEAD_MS`] left (for the UDP gateway).
    pub(crate) fn live_jwt(&self, now_ms: i64) -> Option<String> {
        match &self.phase {
            Phase::Ready(jwt) if jwt.expires_at_ms - now_ms > RENEW_AHEAD_MS => {
                Some(jwt.token.clone())
            }
            _ => None,
        }
    }

    /// Unix ms when the current JWT should be proactively renewed; None when not `Ready`.
    pub(crate) fn renew_due_ms(&self) -> Option<i64> {
        match &self.phase {
            Phase::Ready(jwt) => Some(jwt.expires_at_ms - RENEW_AHEAD_MS),
            Phase::Unauthenticated => None,
        }
    }

    /// Whether the install holds a server identity it may use unasked for `(server, regno)`
    /// (demo, matching refresh token, matching v2 token, or a ready JWT for the student).
    pub(crate) fn has_usable_identity(
        &self,
        server: &str,
        regno: Option<&str>,
        refresh: Option<&Refresh>,
        has_v2: bool,
    ) -> bool {
        let server = normalize_server(server);
        let demo = regno.is_some_and(|r| r.eq_ignore_ascii_case(crate::demo::USERNAME));
        if demo || matching_refresh(&server, regno, refresh).is_some() || has_v2 {
            return true;
        }
        matches!(
            &self.phase,
            Phase::Ready(jwt)
                if jwt.server == server
                    && regno.is_none_or(|r| r.eq_ignore_ascii_case(&jwt.regno))
        )
    }

    /// Decides what the caller should do next to hold a live JWT for `(server, regno)`.
    ///
    /// If a previously `Ready` JWT has expired or belongs to another server/student, transitions
    /// to `Unauthenticated` and emits `SetSocketAuth(None)` so `auth:token` never runs behind an
    /// expired JWT on the WebSocket.
    pub(crate) fn next_step(&mut self, input: &StepInput<'_>) -> (Step, Vec<Effect>) {
        let server = normalize_server(input.server);
        let mut effects = Vec::new();
        if let Phase::Ready(jwt) = &self.phase {
            let server_matches = jwt.server == server;
            let regno_matches = input
                .regno
                .is_none_or(|r| r.eq_ignore_ascii_case(&jwt.regno));
            let fresh = jwt.expires_at_ms - RENEW_AHEAD_MS > input.now_ms;
            if server_matches && regno_matches && fresh {
                return (Step::Ready(self.stamp()), effects);
            }
            let reason = if !server_matches {
                "server_changed"
            } else if !regno_matches {
                "student_changed"
            } else {
                "jwt_expired"
            };
            let new_generation = !server_matches || !regno_matches;
            self.drop_jwt("next_step", reason, new_generation, &mut effects);
        }

        let demo = input
            .regno
            .is_some_and(|r| r.eq_ignore_ascii_case(crate::demo::USERNAME));
        let step = if demo {
            Step::FetchToken(Grant::Demo)
        } else if let Some(refresh) = matching_refresh(&server, input.regno, input.refresh) {
            Step::FetchToken(Grant::Refresh(refresh.clone()))
        } else if let Some(v2) = input.v2_token.filter(|_| input.regno.is_some()) {
            Step::FetchToken(Grant::V2(v2.to_string()))
        } else if input.allow_proof && input.regno.is_some() {
            Step::FetchToken(Grant::Proof)
        } else {
            Step::Blocked(if input.allow_proof {
                Blocked::NotSignedIn
            } else {
                Blocked::ConsentDenied
            })
        };
        (step, effects)
    }

    /// Records a newly issued JWT + rotated refresh token from `auth:token`.
    pub(crate) fn on_issued(
        &mut self,
        source: GrantKind,
        server: &str,
        regno: &str,
        token: String,
        expires_at_ms: i64,
        refresh_token: String,
    ) -> (Stamp, Vec<Effect>) {
        let from = self.phase.name();
        let server = normalize_server(server);
        let regno = regno.to_ascii_uppercase();
        if source != GrantKind::Refresh {
            self.generation += 1;
        }
        self.jwt_epoch += 1;
        self.phase = Phase::Ready(Jwt {
            server: server.clone(),
            regno: regno.clone(),
            token: token.clone(),
            expires_at_ms,
        });
        let stamp = self.stamp();
        info!(
            from,
            to = self.phase.name(),
            event = "issued",
            source = source.as_str(),
            gen = stamp.generation,
            jwt_epoch = stamp.jwt_epoch,
            "auth transition"
        );
        let mut effects = Vec::with_capacity(4);
        if source == GrantKind::V2 {
            effects.push(Effect::ForgetV2Token);
        }
        effects.push(Effect::SaveRefresh(Some(Refresh {
            server,
            token: refresh_token,
            regno,
        })));
        effects.push(Effect::SetSocketAuth(Some(token)));
        effects.push(Effect::ResetUdpJwt);
        (stamp, effects)
    }

    /// Handles a failed `auth:token`. Returns `(should_retry, effects)`.
    /// Only a refused token ([`ServerCode::is_token_refused`], on non-proof grants) drops the
    /// rejected grant and retries; transient errors (offline, 5xx) keep the refresh token.
    pub(crate) fn on_token_refused(
        &mut self,
        source: GrantKind,
        error: &EdgeError,
    ) -> (bool, Vec<Effect>) {
        let code = match error {
            EdgeError::Status { code, .. } if code.is_token_refused() => code,
            _ => return (false, Vec::new()),
        };
        if source == GrantKind::Proof {
            return (false, Vec::new());
        }
        let from = self.phase.name();
        let mut effects = Vec::new();
        let removed = *code == ServerCode::DeviceRemoved;
        let code = code.as_str();
        match (source, removed) {
            (GrantKind::V2, _) => {
                effects.push(Effect::ForgetV2Token);
                info!(
                    from,
                    to = self.phase.name(),
                    event = "token_refused",
                    source = source.as_str(),
                    code,
                    gen = self.generation,
                    jwt_epoch = self.jwt_epoch,
                    "auth transition"
                );
            }
            (_, true) => {
                self.clear_identity("token_refused", code, true, &mut effects);
            }
            _ => {
                self.clear_identity("token_refused", code, false, &mut effects);
            }
        }
        (true, effects)
    }

    /// The server rejected the JWT (`unauthenticated` or socket `AuthError`).
    /// When `jwt_epoch` is `Some(e)` and doesn't match the current epoch, the rejection is stale
    /// (from an earlier JWT) and is ignored.
    pub(crate) fn on_jwt_rejected(&mut self, jwt_epoch: Option<u64>) -> (bool, Vec<Effect>) {
        if let Some(epoch) = jwt_epoch
            && epoch != self.jwt_epoch
        {
            return (false, Vec::new());
        }
        let mut effects = Vec::new();
        let changed = matches!(self.phase, Phase::Ready(_));
        if changed {
            self.drop_jwt("jwt_rejected", "unauthenticated", false, &mut effects);
        }
        (changed, effects)
    }

    /// The server answered `device_removed` for a call or watch started under `generation`.
    /// Stale errors from an older generation are ignored so they never wipe a newer refresh token.
    pub(crate) fn on_device_removed(&mut self, generation: u64) -> (bool, Vec<Effect>) {
        if generation != self.generation {
            info!(
                stale_gen = generation,
                current_gen = self.generation,
                "ignoring stale device_removed"
            );
            return (false, Vec::new());
        }
        let mut effects = Vec::new();
        self.clear_identity("device_removed", "device_removed", true, &mut effects);
        (true, effects)
    }

    /// Explicit identity forget (server sign-out, account deleted, device deleted).
    pub(crate) fn on_forget(&mut self, reason: &str) -> Vec<Effect> {
        let mut effects = Vec::new();
        self.clear_identity("forget", reason, false, &mut effects);
        effects
    }

    /// Sets the current refresh token aside for deferred sign-out (`server_last_student.json`,
    /// of every phone on the account with `everywhere`) and clears this install's active
    /// identity.
    pub(crate) fn on_set_aside(
        &mut self,
        current_refresh: Option<Refresh>,
        everywhere: bool,
    ) -> Vec<Effect> {
        let mut effects = Vec::new();
        if let Some(refresh) = current_refresh {
            effects.push(Effect::KeepSignOut(KeptSignOut {
                refresh,
                everywhere,
            }));
        }
        self.clear_identity(
            "set_aside",
            "student_switched_or_signed_out",
            false,
            &mut effects,
        );
        effects
    }

    /// The Developer server URL changed: drop the active JWT and bump generation so calls on the
    /// old client can't touch the new server's state.
    pub(crate) fn on_server_changed(&mut self) -> Vec<Effect> {
        let from = self.phase.name();
        self.generation += 1;
        self.jwt_epoch += 1;
        self.phase = Phase::Unauthenticated;
        info!(
            from,
            to = self.phase.name(),
            event = "server_changed",
            gen = self.generation,
            jwt_epoch = self.jwt_epoch,
            "auth transition"
        );
        vec![Effect::ResetUdpJwt]
    }

    fn drop_jwt(
        &mut self,
        event: &str,
        reason: &str,
        bump_generation: bool,
        effects: &mut Vec<Effect>,
    ) {
        let from = self.phase.name();
        if bump_generation {
            self.generation += 1;
        }
        self.jwt_epoch += 1;
        self.phase = Phase::Unauthenticated;
        info!(
            from,
            to = self.phase.name(),
            event,
            reason,
            gen = self.generation,
            jwt_epoch = self.jwt_epoch,
            "auth transition"
        );
        effects.push(Effect::SetSocketAuth(None));
        effects.push(Effect::ResetUdpJwt);
    }

    fn clear_identity(
        &mut self,
        event: &str,
        reason: &str,
        forget_forwarding: bool,
        effects: &mut Vec<Effect>,
    ) {
        let from = self.phase.name();
        self.generation += 1;
        self.jwt_epoch += 1;
        self.phase = Phase::Unauthenticated;
        info!(
            from,
            to = self.phase.name(),
            event,
            reason,
            gen = self.generation,
            jwt_epoch = self.jwt_epoch,
            "auth transition"
        );
        effects.push(Effect::ResetUdpJwt);
        effects.push(Effect::SetSocketAuth(None));
        effects.push(Effect::SaveRefresh(None));
        if forget_forwarding {
            effects.push(Effect::ForgetForwarding);
        }
    }
}

/// Returns `refresh` only if it belongs to `server` (normalised) and `regno` (case-insensitive).
pub(crate) fn matching_refresh<'a>(
    server: &str,
    regno: Option<&str>,
    refresh: Option<&'a Refresh>,
) -> Option<&'a Refresh> {
    let server = normalize_server(server);
    refresh.filter(|kept| {
        normalize_server(&kept.server) == server
            && regno.is_none_or(|r| r.eq_ignore_ascii_case(&kept.regno))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SERVER: &str = "https://vmate.aster0.dev";
    const REGNO: &str = "22BCE0001";

    fn kept(token: &str) -> Refresh {
        Refresh {
            server: SERVER.into(),
            token: token.into(),
            regno: REGNO.into(),
        }
    }

    fn input<'a>(refresh: Option<&'a Refresh>, allow_proof: bool, now_ms: i64) -> StepInput<'a> {
        StepInput {
            server: SERVER,
            regno: Some(REGNO),
            refresh,
            v2_token: None,
            allow_proof,
            now_ms,
        }
    }

    #[test]
    fn ready_jwt_is_reused_until_renew_ahead_then_cleared_before_refresh() {
        let mut machine = AuthMachine::new();
        let refresh = kept("vrt_1");
        let (stamp, effects) = machine.on_issued(
            GrantKind::Refresh,
            SERVER,
            REGNO,
            "jwt-1".into(),
            1_000_000,
            "vrt_2".into(),
        );
        assert_eq!(
            stamp,
            Stamp {
                generation: 1,
                jwt_epoch: 1
            }
        );
        assert!(effects.contains(&Effect::SetSocketAuth(Some("jwt-1".into()))));

        // Still fresh: reused with no effects.
        let (step, effects) = machine.next_step(&input(Some(&refresh), true, 900_000));
        assert!(matches!(step, Step::Ready(s) if s == stamp));
        assert!(effects.is_empty());

        // Within RENEW_AHEAD_MS: clears expired socket auth before fetching a new token via refresh.
        let (step, effects) =
            machine.next_step(&input(Some(&refresh), true, 1_000_000 - RENEW_AHEAD_MS));
        assert!(matches!(step, Step::FetchToken(Grant::Refresh(r)) if r.token == "vrt_1"));
        assert_eq!(
            effects,
            vec![Effect::SetSocketAuth(None), Effect::ResetUdpJwt]
        );
        assert_eq!(machine.socket_jwt(), None);
    }

    #[test]
    fn trailing_slash_in_server_url_still_matches_refresh_token() {
        let mut machine = AuthMachine::new();
        let refresh = Refresh {
            server: format!("{SERVER}/"),
            token: "vrt_1".into(),
            regno: REGNO.into(),
        };
        let step_in = StepInput {
            server: SERVER,
            regno: Some(REGNO),
            refresh: Some(&refresh),
            v2_token: None,
            allow_proof: true,
            now_ms: 0,
        };
        let (step, _) = machine.next_step(&step_in);
        assert!(matches!(step, Step::FetchToken(Grant::Refresh(r)) if r.token == "vrt_1"));
    }

    #[test]
    fn stale_device_removed_never_wipes_a_newer_generation() {
        let mut machine = AuthMachine::new();
        let (old_stamp, _) = machine.on_issued(
            GrantKind::Proof,
            SERVER,
            REGNO,
            "jwt-old".into(),
            1_000_000,
            "vrt_old".into(),
        );
        // A new proof replaces the identity, bumping generation.
        let (new_stamp, _) = machine.on_issued(
            GrantKind::Proof,
            SERVER,
            REGNO,
            "jwt-new".into(),
            2_000_000,
            "vrt_new".into(),
        );
        assert!(new_stamp.generation > old_stamp.generation);

        // Late device_removed from old generation is ignored.
        let (applied, effects) = machine.on_device_removed(old_stamp.generation);
        assert!(!applied);
        assert!(effects.is_empty());
        assert_eq!(machine.socket_jwt().as_deref(), Some("jwt-new"));

        // Routine refresh keeps the same generation, so a real device_removed still applies.
        let (refreshed, _) = machine.on_issued(
            GrantKind::Refresh,
            SERVER,
            REGNO,
            "jwt-new-2".into(),
            3_000_000,
            "vrt_new_2".into(),
        );
        assert_eq!(refreshed.generation, new_stamp.generation);
        let (applied, effects) = machine.on_device_removed(new_stamp.generation);
        assert!(applied);
        assert!(effects.contains(&Effect::SaveRefresh(None)));
        assert!(effects.contains(&Effect::ForgetForwarding));
        assert!(effects.contains(&Effect::SetSocketAuth(None)));
    }

    #[test]
    fn stale_jwt_rejected_does_not_invalidate_a_newer_jwt() {
        let mut machine = AuthMachine::new();
        let (first, _) = machine.on_issued(
            GrantKind::Refresh,
            SERVER,
            REGNO,
            "jwt-1".into(),
            1_000_000,
            "vrt_1".into(),
        );
        let (second, _) = machine.on_issued(
            GrantKind::Refresh,
            SERVER,
            REGNO,
            "jwt-2".into(),
            2_000_000,
            "vrt_2".into(),
        );
        let (changed, effects) = machine.on_jwt_rejected(Some(first.jwt_epoch));
        assert!(!changed);
        assert!(effects.is_empty());
        assert_eq!(machine.socket_jwt().as_deref(), Some("jwt-2"));

        let (changed, effects) = machine.on_jwt_rejected(Some(second.jwt_epoch));
        assert!(changed);
        assert_eq!(
            effects,
            vec![Effect::SetSocketAuth(None), Effect::ResetUdpJwt]
        );
        assert_eq!(machine.socket_jwt(), None);
    }

    #[test]
    fn transient_token_error_keeps_refresh_while_refused_drops_it() {
        let mut machine = AuthMachine::new();
        let refused = |code| EdgeError::Status {
            code,
            message: String::new(),
        };
        let (retry, effects) =
            machine.on_token_refused(GrantKind::Refresh, &refused(ServerCode::ServerError));
        assert!(!retry);
        assert!(effects.is_empty());

        let (retry, effects) =
            machine.on_token_refused(GrantKind::Refresh, &refused(ServerCode::RefreshReused));
        assert!(retry);
        assert!(effects.contains(&Effect::SaveRefresh(None)));
        assert!(!effects.contains(&Effect::ForgetForwarding));

        let (retry, effects) =
            machine.on_token_refused(GrantKind::Refresh, &refused(ServerCode::DeviceRemoved));
        assert!(retry);
        assert!(effects.contains(&Effect::SaveRefresh(None)));
        assert!(effects.contains(&Effect::ForgetForwarding));

        // Proof refusal never retries in a loop.
        let (retry, effects) =
            machine.on_token_refused(GrantKind::Proof, &refused(ServerCode::DeviceRemoved));
        assert!(!retry);
        assert!(effects.is_empty());
        let (retry, effects) = machine.on_token_refused(GrantKind::Refresh, &EdgeError::Offline);
        assert!(!retry);
        assert!(effects.is_empty());
    }

    #[test]
    fn unasked_call_without_grant_is_blocked_with_consent_denied() {
        let mut machine = AuthMachine::new();
        let (step, effects) = machine.next_step(&input(None, false, 0));
        assert!(matches!(step, Step::Blocked(Blocked::ConsentDenied)));
        assert!(effects.is_empty());

        let (step, _) = machine.next_step(&StepInput {
            server: SERVER,
            regno: None,
            refresh: None,
            v2_token: None,
            allow_proof: true,
            now_ms: 0,
        });
        assert!(matches!(step, Step::Blocked(Blocked::NotSignedIn)));
    }
}
