//! Server identity driver: JWT and refresh lifecycle over [`super::auth::AuthMachine`],
//! kept sign-outs (`server_last_student.json`) and background proofs.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{info, warn};

use super::Kind;
use super::auth::{self, Blocked, Effect, Grant, Stamp, Step, StepInput};
use super::rpc::edge_error;
use crate::device::EdgeError;
use crate::remote::account_token;
use crate::{Core, CoreError, unix_now};

/// A JWT this close to its end is renewed first.
pub(super) const RENEW_AHEAD_MS: i64 = 60_000;
/// How long a proof someone started may take, the ticket to the verdict: the verifier may queue
/// it for a minute before it starts (keeping the socket alive, sending nothing).
const PROOF_BUDGET: Duration = Duration::from_secs(150);

/// A refresh token set aside to sign out with ([`KeptSignOut`]). The name is from when only a
/// switch to another student kept one.
pub(crate) const KEPT_SIGN_OUT: &str = "server_last_student.json";

/// The refresh token `auth:token` handed this install. No `Debug`: it holds the token.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Refresh {
    pub(crate) server: String,
    pub(crate) token: String,
    pub(crate) regno: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Issued {
    jwt: String,
    jwt_expires_at: i64,
    refresh_token: String,
    regno: String,
}

/// [`KEPT_SIGN_OUT`] on disk: a refresh token set aside at a switch to another student, a server
/// sign-out sent offline or the registration number no longer shared, kept until the server
/// signed this install out with it ([`Core::send_kept_sign_out`]). Older versions wrote the
/// [`Refresh`] alone, which reads as `everywhere: false`; `kept_sign_out_format_roundtrips` pins
/// it. No `Debug`: it holds the token.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct KeptSignOut {
    #[serde(flatten)]
    pub(crate) refresh: Refresh,
    /// The other phones on the account come off it too (Server's "Sign out everywhere").
    #[serde(default)]
    pub(crate) everywhere: bool,
}

/// What [`Core::send_kept_sign_out`] left behind.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum KeptAfter {
    /// Nothing kept: sent, refused as signed out already, or for another server.
    Nothing,
    /// Still kept for the next call, for `regno`; `offline` when the server wasn't reached.
    Kept { regno: String, offline: bool },
}

/// Whether a sign-out failed because there is nothing left to sign out of.
pub(crate) fn signed_out_already(error: &EdgeError) -> bool {
    matches!(error, EdgeError::Status { code, .. } if code.is_signed_out_already())
}

impl Core {
    /// The signed-in student's registration number, upper case; None signed out.
    pub(crate) async fn signed_in_regno(&self) -> Option<String> {
        let slot = self.current().ok()?;
        let session = slot.read().await;
        // A restored or restarted session that hasn't loaded a page yet knows it from the saved
        // account (in memory or on disk).
        let regno = session.client.registration_number().ok();
        regno
            .or_else(|| session.account.registration_number.clone())
            .or_else(|| {
                self.read_account()
                    .filter(|saved| {
                        saved
                            .username
                            .eq_ignore_ascii_case(&session.account.username)
                    })
                    .and_then(|saved| saved.registration_number)
            })
            .or_else(|| crate::demo::is(&session.account).then(|| session.account.username.clone()))
            .map(|regno| regno.to_ascii_uppercase())
    }

    /// Runs the side effects emitted by [`auth::AuthMachine`] outside the state lock.
    pub(super) fn run_effects(&self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::SetSocketAuth(jwt) => {
                    let backend = if jwt.is_some() {
                        self.backend().ok()
                    } else {
                        self.shared.backend_state().backend.clone()
                    };
                    if let Some(backend) = backend {
                        backend.set_auth(jwt);
                    }
                }
                Effect::ResetUdpJwt => crate::remote::udp::forget_jwt_sent(&self.shared),
                Effect::SaveRefresh(refresh) => self.keep_refresh(refresh),
                Effect::KeepSignOut(kept) => self.write(KEPT_SIGN_OUT, &kept),
                Effect::ForgetV2Token => self.forget_account_token(),
                Effect::ForgetForwarding => self.forget_any_forwarding(),
            }
        }
        self.shared.identity_may_have_changed();
    }

    /// Makes sure the client goes with a live JWT for the signed-in student, returning the
    /// [`Stamp`] it ran under so callers can ignore stale errors from an earlier identity.
    pub(super) async fn ensure_identity(
        &self,
        flow: &str,
        action: &str,
        ask: bool,
    ) -> Result<Stamp, CoreError> {
        self.identity_within(flow, action, ask.then_some(PROOF_BUDGET), false)
            .await
            .map(|(stamp, _)| stamp)
    }

    /// Makes sure the client goes with a live JWT for the signed-in student, getting one from
    /// the first source that works: the demo account, the kept refresh token, a v2 account
    /// token (traded in once), then (only when `ask`) a proof. One at a time, so a refresh token
    /// is never used twice. A new one first leaves the addresses queued to be left: a leave
    /// queued by a sign-in over another student waits for this phone's next identity.
    pub(crate) async fn identity(
        &self,
        flow: &str,
        action: &str,
        ask: bool,
    ) -> Result<(), CoreError> {
        self.ensure_identity(flow, action, ask).await.map(drop)
    }

    /// [`Self::identity`], proving within `prove` when it must (None: never); returns
    /// `(stamp, got_new_jwt)`. With `background_only`, returns `Ok((stamp, false))` without
    /// network traffic whenever a usable identity or grant already exists.
    pub(super) async fn identity_within(
        &self,
        flow: &str,
        action: &str,
        prove: Option<Duration>,
        background_only: bool,
    ) -> Result<(Stamp, bool), CoreError> {
        let (stamp, taken) = self
            .take_identity(flow, action, prove, background_only)
            .await?;
        if taken {
            self.leave_queued(flow).await;
            self.semester_to_server(flow).await;
        }
        Ok((stamp, taken))
    }

    /// The kept v2 account token for `regno`, if any.
    pub(super) fn kept_v2(&self, regno: Option<&str>) -> Option<account_token::Saved> {
        self.read::<account_token::Saved>(account_token::FILE)
            .filter(|saved| {
                regno.is_some_and(|regno| regno.eq_ignore_ascii_case(&saved.registration_number))
            })
    }

    /// Unix ms on the server's clock, as the JWT's expiry is: a phone set minutes off would
    /// otherwise renew on every call, or hold on to an expired JWT.
    pub(super) fn server_now_ms(&self) -> i64 {
        unix_now()
            .saturating_add(self.device().clock_offset)
            .saturating_mul(1000)
    }

    async fn take_identity(
        &self,
        flow: &str,
        action: &str,
        prove: Option<Duration>,
        background_only: bool,
    ) -> Result<(Stamp, bool), CoreError> {
        // Before the turn: the sheet may wait on the student, and nothing names them until
        // they said Share. A background run or a call nobody started never asks.
        let may_ask = prove.is_some() && !background_only;
        let asked_for = self.signed_in_regno().await;
        self.regno_allowed(flow, asked_for.as_deref(), may_ask)
            .await?;
        let _turn = self.shared.identity_turn.lock().await;
        // Another student may have signed in while this one waited.
        let regno = self.signed_in_regno().await;
        if regno != asked_for {
            self.regno_allowed(flow, regno.as_deref(), false).await?;
        }
        let server = self.forwarding_server();
        if background_only {
            let Some(regno_str) = regno.as_deref() else {
                return Ok((self.shared.backend_state().auth.stamp(), false));
            };
            let refresh = self.device().refresh;
            let has_v2 = self.kept_v2(Some(regno_str)).is_some();
            let state = self.shared.backend_state();
            if regno_str.eq_ignore_ascii_case(crate::demo::USERNAME)
                || state.auth.has_usable_identity(
                    &server,
                    Some(regno_str),
                    refresh.as_ref(),
                    has_v2,
                )
            {
                return Ok((state.auth.stamp(), false));
            }
        }
        for _ in 0..3 {
            let now_ms = self.server_now_ms();
            let refresh = self.device().refresh;
            let v2 = self.kept_v2(regno.as_deref());
            let (step, effects) = {
                let mut state = self.shared.backend_state();
                state.auth.next_step(&StepInput {
                    server: &server,
                    regno: regno.as_deref(),
                    refresh: refresh.as_ref(),
                    v2_token: v2.as_ref().map(|saved| saved.token.as_str()),
                    allow_proof: prove.is_some(),
                    now_ms,
                })
            };
            self.run_effects(effects);
            let grant = match step {
                Step::Ready(stamp) => return Ok((stamp, false)),
                Step::Blocked(Blocked::NotSignedIn) => {
                    info!(action, signed_in = false, "no identity: not proving");
                    return Err(CoreError::NotSignedIn);
                }
                Step::Blocked(Blocked::ConsentDenied) => {
                    info!(
                        action,
                        signed_in = regno.is_some(),
                        "no identity: not proving"
                    );
                    return Err(CoreError::ConsentDenied);
                }
                Step::FetchToken(grant) => grant,
            };
            // A sign-out kept aside goes first: sent after a new identity of the same student,
            // it would take that identity's refresh tokens with it (the server signs the whole
            // install out). While the server can't be reached, no new identity either; any
            // other failure doesn't hold the student up.
            if let KeptAfter::Kept {
                regno: kept,
                offline: true,
            } = self.send_kept_sign_out_in_turn(flow).await
                && regno.as_deref() == Some(kept.as_str())
            {
                info!(action, "a kept sign-out waits for the server: not proving");
                return Err(CoreError::Offline);
            }
            let source = grant.kind();
            let payload = match grant {
                Grant::Demo => json!({ "demo": true, "device": self.device_info() }),
                Grant::Refresh(refresh) => json!({ "refreshToken": refresh.token }),
                Grant::V2(token) => {
                    json!({ "v2AccountToken": token, "device": self.device_info() })
                }
                Grant::Proof => {
                    let Some(budget) = prove else {
                        return Err(CoreError::ConsentDenied);
                    };
                    info!(action, "no identity: proving");
                    let ticket = self.prove_account(flow, budget).await?;
                    json!({ "ticket": ticket, "device": self.device_info() })
                }
            };
            let asked = Instant::now();
            let issued = self
                .signed_call(flow, Kind::Action, "auth:token", &payload)
                .await;
            let token_ms = asked.elapsed().as_millis() as u64;
            let issued = match issued {
                Ok(issued) => issued,
                Err(error) => {
                    let (retry, effects) = self
                        .shared
                        .backend_state()
                        .auth
                        .on_token_refused(source, &error);
                    if retry {
                        warn!(
                            source = source.as_str(),
                            code = error.code(),
                            "token refused"
                        );
                        self.run_effects(effects);
                        continue;
                    }
                    return Err(error.into());
                }
            };
            let issued: Issued = serde_json::from_value(issued).map_err(|_| CoreError::Other {
                reason: "The server sent an unexpected answer.".into(),
            })?;
            // ms; seconds from a server that counts them.
            let expires_at = match issued.jwt_expires_at {
                at if at < 10_000_000_000 => at.saturating_mul(1000),
                at => at,
            };
            let (stamp, effects) = self.shared.backend_state().auth.on_issued(
                source,
                &server,
                &issued.regno,
                issued.jwt,
                expires_at,
                issued.refresh_token,
            );
            self.run_effects(effects);
            // Signed out (leaving an old address), the lines still name the student.
            if regno.is_none() {
                crate::obs::set_regno(Some(issued.regno.to_ascii_uppercase()));
            }
            info!(
                source = source.as_str(),
                token_ms,
                expires_in_s = (expires_at - now_ms) / 1000,
                "server identity"
            );
            return Ok((stamp, true));
        }
        Err(CoreError::Other {
            reason: "The server turned down this phone's sign-in.".into(),
        })
    }

    /// Marks the current JWT stale (if `jwt_epoch` matches) and clears socket auth.
    pub(crate) fn on_jwt_rejected(&self, jwt_epoch: Option<u64>) {
        let (_, effects) = self.shared.backend_state().auth.on_jwt_rejected(jwt_epoch);
        self.run_effects(effects);
    }

    /// Handles a `device_removed` response from `generation`; returns true when it applied.
    pub(crate) fn on_device_removed(&self, generation: u64) -> bool {
        let (applied, effects) = self
            .shared
            .backend_state()
            .auth
            .on_device_removed(generation);
        self.run_effects(effects);
        applied
    }

    /// Developer changed the server URL: drops the old client and active JWT while leaving the
    /// single renewal coordinator intact.
    pub(crate) fn on_server_changed(&self) {
        let (old, effects) = {
            let mut state = self.shared.backend_state();
            let old = state.backend.take();
            state.renewal.poke();
            (old, state.auth.on_server_changed())
        };
        if let Some(old) = old {
            old.close();
        }
        self.run_effects(effects);
    }

    /// Another student signed in over the last one: the last one's refresh token is kept aside
    /// for [`Core::send_kept_sign_out`], and this install has no identity until the new
    /// student's.
    pub(crate) fn set_aside_identity(&self) {
        self.set_aside_sign_out(false);
    }

    /// Drops the identity here at once and keeps its refresh token aside, so
    /// [`Core::send_kept_sign_out`] signs this install out of the server with it (of every phone
    /// on the account with `everywhere`), now or on a later call while the server can't be
    /// reached.
    pub(crate) fn set_aside_sign_out(&self, everywhere: bool) {
        let current = self.device().refresh;
        let effects = self
            .shared
            .backend_state()
            .auth
            .on_set_aside(current, everywhere);
        self.run_effects(effects);
    }

    /// Signs this install out of the server with the refresh token kept aside
    /// ([`KEPT_SIGN_OUT`]), so it stops getting that student's OTPs and session requests and
    /// the server drops its refresh tokens. Tried again on the next call while the server can't
    /// be reached.
    pub(crate) async fn send_kept_sign_out(&self, flow: &str) -> KeptAfter {
        let _turn = self.shared.identity_turn.lock().await;
        self.send_kept_sign_out_in_turn(flow).await
    }

    /// [`Self::send_kept_sign_out`] with the identity's turn held. The socket goes with the kept
    /// token's JWT for this one call: no other identity can be taken meanwhile, and the current
    /// student's JWT (if any) is restored afterwards.
    async fn send_kept_sign_out_in_turn(&self, flow: &str) -> KeptAfter {
        let Some(kept) = self.read::<KeptSignOut>(KEPT_SIGN_OUT) else {
            return KeptAfter::Nothing;
        };
        let KeptSignOut {
            refresh: last,
            everywhere,
        } = kept;
        if auth::normalize_server(&last.server) != auth::normalize_server(&self.forwarding_server())
        {
            self.remove_file(KEPT_SIGN_OUT);
            return KeptAfter::Nothing;
        }
        let kept = |offline: bool| KeptAfter::Kept {
            regno: last.regno.to_ascii_uppercase(),
            offline,
        };
        let body = json!({ "refreshToken": last.token });
        let issued = match self
            .signed_call(flow, Kind::Action, "auth:token", &body)
            .await
        {
            Ok(issued) => issued,
            Err(error) if signed_out_already(&error) => {
                info!(code = error.code(), "kept sign-out: signed out already");
                self.remove_file(KEPT_SIGN_OUT);
                return KeptAfter::Nothing;
            }
            Err(error) => {
                info!(code = error.code(), "kept sign-out not sent yet");
                return kept(matches!(error, EdgeError::Offline));
            }
        };
        // Persist the rotated refresh token before calling `auth:signOut`, so an offline failure
        // retries with the newest token instead of reusing the replaced one.
        if let Some(rotated) = issued.get("refreshToken").and_then(Value::as_str) {
            self.write(
                KEPT_SIGN_OUT,
                &KeptSignOut {
                    refresh: Refresh {
                        server: last.server.clone(),
                        token: rotated.to_owned(),
                        regno: last.regno.clone(),
                    },
                    everywhere,
                },
            );
        }
        let (Some(jwt), Ok(backend)) = (issued.get("jwt").and_then(Value::as_str), self.backend())
        else {
            return kept(false);
        };
        backend.set_auth(Some(jwt.to_owned()));
        let signed_out = self
            .run(
                flow,
                Kind::Mutation,
                "auth:signOut",
                json!({ "everywhere": everywhere }),
            )
            .await;
        let restore = self.shared.backend_state().auth.socket_jwt();
        backend.set_auth(restore);
        match signed_out {
            Ok(_) => {
                info!(everywhere, "kept sign-out sent");
                self.remove_file(KEPT_SIGN_OUT);
                KeptAfter::Nothing
            }
            Err(error) => {
                let error = edge_error("auth:signOut", error);
                info!(code = error.code(), "kept sign-out not sent yet");
                kept(matches!(error, EdgeError::Offline))
            }
        }
    }

    /// The JWT the client goes with, while it has a minute left: the UDP gateway signs in with
    /// it when the session it holds expires.
    pub(crate) fn live_jwt(&self) -> Option<String> {
        let now = self.server_now_ms();
        self.shared.backend_state().auth.live_jwt(now)
    }

    /// Drops the JWT and the refresh token (signed out of the server; another student; this
    /// install removed there). The v2 token goes with the student's files.
    pub(crate) fn forget_identity(&self) {
        let effects = self.shared.backend_state().auth.on_forget("explicit");
        self.run_effects(effects);
    }
}

#[cfg(not(feature = "tlsn"))]
impl Core {
    async fn prove_account(&self, _: &str, _: Duration) -> Result<String, CoreError> {
        Err(CoreError::Other {
            reason: "This build cannot prove who you are to the server.".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kept_sign_out_format_roundtrips() {
        // An older version's file: the refresh token alone.
        let old = json!({ "server": "https://s", "token": "vrt_1", "regno": "22BCE0001" });
        let kept: KeptSignOut = serde_json::from_value(old).unwrap();
        assert_eq!(kept.refresh.token, "vrt_1");
        assert!(!kept.everywhere);
        let text = json!({
            "server": "https://s", "token": "vrt_1", "regno": "22BCE0001", "everywhere": true,
        });
        let kept: KeptSignOut = serde_json::from_value(text.clone()).unwrap();
        assert!(kept.everywhere);
        assert_eq!(serde_json::to_value(&kept).unwrap(), text);
    }
}
