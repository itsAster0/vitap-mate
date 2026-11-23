//! Mail forwarding (the Flutter app's `mail_forwarding_service.dart`): Gmail forwards VTOP's OTP
//! mail to `otp+<id>@<domain>`; the edge Worker reads it, seals the code to this install's
//! X25519 seal key (`device.json`, see [`crate::device`]; `vtop_shared::mail_seal`, the mailbox
//! id as associated data) and the server keeps the latest box for 5 minutes, which the app reads
//! from `mail:inbox`. Every call goes as the student this install proved to be
//! ([`Core::account_call`]); a mailbox belongs to their regno from the start and the server knows
//! its members by device id, so `forwarding.json` holds no secret. Codes and mail contents are
//! never logged.
//!
//! Only two kinds of mail are relayed: VTOP's OTP and Gmail's forwarding confirmation (sent once,
//! when the student adds the address in Gmail). That confirmation reaching the phone proves the
//! address works.

use std::fs;
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{debug, info, warn};
use vtop_shared::mail_seal;

use vtop_core::spec;
use vtop_core::types::StudentProfile;

use crate::remote::backend::Kind;
use crate::{Core, CoreError, PAGES, traced, traced_sync};

pub(crate) const FILE: &str = "forwarding.json";
/// Addresses this phone still has to leave on the edge, kept past sign-out (not a student
/// file): signed out, or with another student signed in, it must stop getting the last one's
/// OTPs. Left by [`Core::leave_old_forwarding`].
pub(crate) const LEAVE_FILE: &str = "forwarding_leave.json";
/// VTOP's OTP sender: the Gmail filter's From.
pub const VTOP_SENDER: &str = "noreply.sdc@vitap.ac.in";
/// The sign-in's time (VTOP's `Date:` just before it mailed the code) and the mail's `Date:`,
/// both VTOP's clock in whole seconds: an earlier sign-in's code is older than this.
const VTOP_SLACK_SECS: i64 = 3;
/// With no challenge to go by, the caller's time is the phone's clock, which may be off.
const CLOCK_SKEW_SECS: i64 = 60;

/// Serialises read-modify-write of [`FILE`].
pub(crate) static FILE_LOCK: Mutex<()> = Mutex::new(());
/// Serialises read-modify-write of [`LEAVE_FILE`].
static LEAVE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ForwardingState {
    /// No address on this phone.
    Off,
    /// An address, but no mail has reached it yet (Gmail not set up).
    Waiting,
    /// Mail reached the address: Gmail's confirmation or an OTP.
    On,
}

/// Gmail's "confirm forwarding" mail: the code to type in Gmail, or the link to open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct GmailConfirmation {
    pub code: Option<String>,
    pub link: Option<String>,
    /// The Gmail address asking to forward.
    pub requester: Option<String>,
    /// Unix seconds.
    pub sent_at: i64,
    /// Whether `requester` is an address on the student's saved VTOP profile; None without a
    /// requester or a saved profile. Anyone who knows the address can make Gmail send one.
    #[serde(skip)]
    pub requester_known: Option<bool>,
}

/// An address to leave: where it lives and its id.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Leave {
    server: String,
    id: String,
}

/// Gmail's confirm link, if it is one: https, a google.com host and `/mail/vf-`, so the button
/// opens nothing else whatever the mail held.
fn gmail_link(link: &str) -> Option<String> {
    let rest = link.strip_prefix("https://")?;
    let (host, path) = rest.split_at(rest.find('/')?);
    let host_ok = host
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        && (host == "google.com" || host.ends_with(".google.com"));
    // One path segment only: a `/` (or `\`, which browsers read as one) after `vf-` could walk
    // up with `..` to any page on the host.
    let token_ok = path.strip_prefix("/mail/vf-").is_some_and(|token| {
        !token.is_empty()
            && token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"%._~-".contains(&b))
    });
    (host_ok && token_ok).then(|| link.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Forwarding {
    pub state: ForwardingState,
    pub address: Option<String>,
    /// What the Gmail filter matches in From.
    pub sender: String,
    /// The latest Gmail confirmation that arrived, while setting up.
    pub confirmation: Option<GmailConfirmation>,
    /// Unix seconds of the latest forwarded mail of either kind.
    pub last_mail_at: Option<i64>,
    /// Unix seconds of the latest forwarded OTP.
    pub last_otp_at: Option<i64>,
}

/// This phone's address. A file from before signed requests (it held a mailbox secret and its
/// own key) does not parse, so it reads as no address.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    /// The edge the address lives on.
    server: String,
    id: String,
    address: String,
    /// From v2, where the student's first address claimed its owner hash. Unused now, and kept
    /// only so a file a v2 install wrote still reads (unknown fields are refused).
    #[serde(default)]
    owner_claimed: bool,
    /// This install is on the address on `server` as the backend v3 knows it. An address kept
    /// from the v2 edge isn't, until [`Core::keep_forwarding`] proves the student and rejoins it.
    #[serde(default)]
    joined: bool,
    /// Forwarding works: an OTP came through (now or, as the server says, before). Gmail's
    /// confirmation alone doesn't count.
    #[serde(default)]
    ready: bool,
    #[serde(default)]
    last_mail_at: Option<i64>,
    #[serde(default)]
    last_otp_at: Option<i64>,
    #[serde(default)]
    confirmation: Option<GmailConfirmation>,
}

impl Saved {
    /// Gmail's confirmation is only for setting up: once an OTP sent after it came, forwarding
    /// works and it goes, as on the edge.
    fn drop_stale_confirmation(&mut self) {
        if self
            .confirmation
            .as_ref()
            .is_some_and(|c| self.last_otp_at >= Some(c.sent_at))
        {
            self.confirmation = None;
        }
    }
}

/// What a box holds (the edge's `Payload` in `edge/src/mail.ts`).
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Payload {
    Otp {
        code: String,
        #[serde(rename = "sentAt")]
        sent_at: i64,
    },
    GmailConfirm {
        code: Option<String>,
        link: Option<String>,
        requester: Option<String>,
        #[serde(rename = "sentAt")]
        sent_at: i64,
    },
}

impl Payload {
    fn sent_at(&self) -> i64 {
        match self {
            Self::Otp { sent_at, .. } | Self::GmailConfirm { sent_at, .. } => *sent_at,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::Otp { .. } => "otp",
            Self::GmailConfirm { .. } => "gmail_confirm",
        }
    }

    /// A six-digit OTP sent no earlier than `since` (minus `slack`).
    fn otp_since(&self, since: i64, slack: i64) -> Option<&str> {
        match self {
            Self::Otp { code, sent_at }
                if code.len() == 6
                    && code.bytes().all(|b| b.is_ascii_digit())
                    && *sent_at >= since - slack =>
            {
                Some(code)
            }
            _ => None,
        }
    }
}

/// Opens a box sealed to this install's seal key for mailbox `id`; the reason it couldn't is
/// logged, never the content.
fn open(seal_secret: &str, id: &str, sealed: &str) -> Option<Payload> {
    let plaintext = mail_seal::open(seal_secret, id, sealed)
        .inspect_err(|error| warn!(?error, "forwarded box not opened"))
        .ok()?;
    serde_json::from_slice(&plaintext)
        .inspect_err(|_| warn!("forwarded box holds an unknown payload"))
        .ok()
}

fn string(body: &Value, key: &str) -> Result<String, CoreError> {
    body[key]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| CoreError::Other {
            reason: "The server sent an unexpected answer.".into(),
        })
}

/// The edge no longer knows this address (deleted, or this install is no longer on it).
pub(crate) const GONE: &str = "mailbox_not_found";
/// This install was removed on the server, which took it off every address.
const REMOVED: &str = "device_removed";

/// Mail forwarding calls; each takes the caller's flow id first.
#[uniffi::export]
impl Core {
    /// This phone's forwarding address and how far its setup got. No network.
    pub fn forwarding_status(&self, flow: String) -> Forwarding {
        traced_sync("forwarding_status", &flow, || Ok(self.forwarding_view()))
            .unwrap_or_else(|_| view(None))
    }

    /// Gives this phone a forwarding address. A student who already has one (made on another
    /// phone) joins it, so their Gmail filter keeps working; otherwise a new address is made for
    /// them. Does nothing when this phone already has one.
    pub async fn set_up_forwarding(&self, flow: String) -> Result<Forwarding, CoreError> {
        traced("set_up_forwarding", &flow, async {
            // A signed-out student's address first, as it may be the one this sign-in joins.
            self.leave_queued(&flow).await;
            if self.read::<Saved>(FILE).is_some() {
                info!("already set up");
                return Ok(self.forwarding_view());
            }
            let server = self.forwarding_server();
            let action = "Set up mail forwarding";
            let body = self
                .account_call(
                    &flow,
                    Kind::Mutation,
                    "mail:register",
                    action,
                    true,
                    json!({}),
                )
                .await??;
            let saved = Saved {
                server,
                id: string(&body, "id")?,
                address: string(&body, "address")?,
                owner_claimed: true,
                joined: true,
                // Even a joined address may never have been confirmed in Gmail: the next pull
                // brings Gmail's confirmation or the latest OTP, which tells.
                ready: false,
                last_mail_at: None,
                last_otp_at: None,
                confirmation: None,
            };
            self.write(FILE, &saved);
            info!("forwarding address set up");
            Ok(self.forwarding_view())
        })
        .await
    }

    /// Pulls the latest forwarded mail (the server keeps it 5 minutes): Gmail's confirmation while
    /// setting up, or an OTP. Either one proves the address works.
    pub async fn check_forwarding(&self, flow: String) -> Result<Forwarding, CoreError> {
        traced("check_forwarding", &flow, async {
            self.pull(&flow).await?;
            Ok(self.forwarding_view())
        })
        .await
    }

    /// The forwarded OTP for the pending challenge (sent after VTOP's time for it), else one
    /// sent since `since` (unix seconds, the phone's clock), if one is in the inbox; for the OTP
    /// prompt, which calls it every few seconds. Never logs the code.
    pub async fn fetch_forwarded_otp(
        &self,
        flow: String,
        since: i64,
    ) -> Result<Option<String>, CoreError> {
        traced("fetch_forwarded_otp", &flow, async {
            let pending = match self.current() {
                Ok(slot) => slot.read().await.client.otp_issued_at(),
                Err(_) => None,
            };
            let (since, slack) = match pending {
                Some(issued_at) => (issued_at as i64, VTOP_SLACK_SECS),
                None => (since, CLOCK_SKEW_SECS),
            };
            let payload = self.pull(&flow).await?;
            let code = payload.as_ref().and_then(|p| p.otp_since(since, slack));
            info!(found = code.is_some(), "forwarded otp");
            Ok(code.map(str::to_string))
        })
        .await
    }

    /// Takes this phone off its address and forgets it. The edge deletes the address when no
    /// other phone uses it; mail forwarded to it is rejected from then on.
    pub async fn turn_off_forwarding(&self, flow: String) -> Result<(), CoreError> {
        traced(
            "turn_off_forwarding",
            &flow,
            self.leave_forwarding(&flow, false),
        )
        .await
    }

    /// Rejoins an address kept from the v2 edge, proving who the student is once (the proof sends
    /// no cookie): the backfill kept the address, but mail is sealed only to the installs on it,
    /// so without this the OTPs stop after the update. Does nothing once joined, without an
    /// address, or for one on another server. Call it with a live session: after a sign-in, or
    /// when the app opens signed in.
    pub async fn keep_forwarding(&self, flow: String) -> Result<(), CoreError> {
        traced("keep_forwarding", &flow, async {
            let Some(saved) = self.read::<Saved>(FILE) else {
                return Ok(());
            };
            if saved.joined || saved.server != self.forwarding_server() {
                return Ok(());
            }
            self.rejoin(&flow, &saved, true).await.map(drop)
        })
        .await
    }

    /// Takes this phone off the addresses a sign-out (or another student signing in) left
    /// behind. The edge keeps a student's address with no phone on it, for their next sign-in to
    /// join. Best effort: an address not left yet stays queued for the next call.
    pub async fn leave_old_forwarding(&self, flow: String) -> Result<(), CoreError> {
        traced("leave_old_forwarding", &flow, async {
            self.sign_out_last_student(&flow).await;
            self.leave_with_identity(&flow).await;
            Ok(())
        })
        .await
    }
}

/// How [`Core::rejoin`] went.
enum Rejoined {
    Joined,
    /// No identity to ask with, or forwarding changed meanwhile.
    Later,
    /// The student has no address.
    NoAddress,
}

impl Core {
    /// Joins `saved`'s address from this install, with the identity it has (a v2 account token
    /// is traded in) but no proof: it runs unasked, on every open until it works, and the next
    /// proof the student starts brings it about. True once joined.
    async fn rejoin(&self, flow: &str, saved: &Saved, make: bool) -> Result<Rejoined, CoreError> {
        let path = if make { "mail:register" } else { "mail:join" };
        let body = match self
            .account_call(
                flow,
                Kind::Mutation,
                path,
                "Keep mail forwarding",
                false,
                json!({}),
            )
            .await
        {
            Ok(answer) => answer?,
            Err(CoreError::ConsentDenied) => {
                info!("no identity yet: rejoined later");
                return Ok(Rejoined::Later);
            }
            Err(error) => return Err(error),
        };
        if body.is_null() {
            info!("the student has no address any more");
            return Ok(Rejoined::NoAddress);
        }
        let id = string(&body, "id")?;
        let address = string(&body, "address")?;
        let _lock = FILE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        // Turned off or signed out meanwhile: not brought back.
        let Some(mut kept) = self.read::<Saved>(FILE).filter(|now| now.id == saved.id) else {
            info!("forwarding changed meanwhile, not rejoined");
            return Ok(Rejoined::Later);
        };
        if id != kept.id {
            // The student's account holds another address: mail goes there now, so its
            // Gmail setup has to be checked again.
            warn!("rejoined a different address");
            kept.address = address;
            kept.id = id;
            kept.ready = false;
            kept.last_mail_at = None;
            kept.last_otp_at = None;
            kept.confirmation = None;
        }
        kept.joined = true;
        kept.server = self.forwarding_server();
        self.write(FILE, &kept);
        info!("forwarding address rejoined");
        Ok(Rejoined::Joined)
    }

    /// Queues this phone's address to be left; called before the student's files are forgotten.
    pub(crate) fn queue_leave(&self) {
        let Some(saved) = self.read::<Saved>(FILE) else {
            return;
        };
        let _lock = LEAVE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let mut queue = self.read::<Vec<Leave>>(LEAVE_FILE).unwrap_or_default();
        if !queue.iter().any(|leave| leave.id == saved.id) {
            queue.push(Leave {
                server: saved.server,
                id: saved.id,
            });
            self.write(LEAVE_FILE, &queue);
            info!("address queued to leave");
        }
    }

    /// [`Core::leave_queued`] as the student this install proved to be, never proving.
    pub(crate) async fn leave_with_identity(&self, flow: &str) {
        match self
            .identity(flow, "Leave an old forwarding address", false)
            .await
        {
            // A new identity already left them.
            Ok(()) => self.leave_queued(flow).await,
            Err(error) => info!(%error, "old addresses wait for an identity"),
        }
    }

    /// Leaves the queued addresses with the JWT already set; one that fails stays queued.
    pub(crate) async fn leave_queued(&self, flow: &str) {
        let queue = self.read::<Vec<Leave>>(LEAVE_FILE).unwrap_or_default();
        let current = self.read::<Saved>(FILE).map(|saved| saved.id);
        let mut done = Vec::new();
        for leave in queue {
            // Joined again since: this phone uses it.
            if current.as_ref() == Some(&leave.id) {
                done.push(leave);
                continue;
            }
            let body = json!({ "id": leave.id });
            // One on another server (the v2 edge) can't be reached from here any more.
            if leave.server != self.forwarding_server() {
                info!("old address on another server dropped");
                done.push(leave);
                continue;
            }
            match self
                .jwt_call(flow, Kind::Mutation, "mail:leave", body)
                .await
            {
                Ok(_) => info!("left an old address"),
                Err(error) if error.code() == GONE => info!("old address already gone"),
                // Offline, or no identity yet: tried again later.
                Err(error) => {
                    warn!(code = error.code(), "old address not left yet");
                    continue;
                }
            }
            done.push(leave);
        }
        if done.is_empty() {
            return;
        }
        let _lock = LEAVE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let mut queue = self.read::<Vec<Leave>>(LEAVE_FILE).unwrap_or_default();
        queue.retain(|leave| !done.contains(leave));
        if queue.is_empty() {
            #[expect(
                clippy::let_underscore_must_use,
                reason = "a file already gone is the goal"
            )]
            let _ = fs::remove_file(self.at(LEAVE_FILE));
        } else {
            self.write(LEAVE_FILE, &queue);
        }
    }

    /// Whether `requester` is an address on the student's saved profile (any field, as VTOP
    /// mails the OTP to the address the student gave it); None without one or the other.
    fn requester_known(&self, requester: Option<&str>) -> Option<bool> {
        let requester = requester?.trim();
        let profile =
            self.read_kept::<StudentProfile>(&format!("{PAGES}/{}", spec::PROFILE.file))?;
        let fields = profile
            .sections
            .iter()
            .flat_map(|section| &section.groups)
            .flat_map(|group| &group.fields)
            .map(|field| field.value.as_str());
        Some(
            std::iter::once(profile.vit_email.as_str())
                .chain(fields)
                .flat_map(|value| value.split(|c: char| c.is_whitespace() || c == ',' || c == ';'))
                .any(|word| word.eq_ignore_ascii_case(requester)),
        )
    }

    /// This phone's address as `(server, mailbox id, address)`, if it has one.
    pub(crate) fn saved_mailbox(&self) -> Option<(String, String, String)> {
        self.read::<Saved>(FILE)
            .map(|saved| (saved.server, saved.id, saved.address))
    }

    /// Takes this phone off its address (deleting it for every phone with `everywhere`) and
    /// forgets it here.
    pub(crate) async fn leave_forwarding(
        &self,
        flow: &str,
        everywhere: bool,
    ) -> Result<(), CoreError> {
        let Some(saved) = self.read::<Saved>(FILE) else {
            return Ok(());
        };
        let body = json!({ "id": saved.id, "everywhere": everywhere });
        match self
            .device_call(flow, Kind::Mutation, "mail:remove", body)
            .await
        {
            Ok(_) => info!(everywhere, "left the address"),
            // Removed on another phone: the server took it off every address already.
            Err(error) if matches!(error.code(), GONE | REMOVED) => {
                info!(code = error.code(), "address already gone");
            }
            Err(error) => return Err(error.into()),
        }
        self.forget_forwarding(&saved.id);
        Ok(())
    }

    fn forwarding_view(&self) -> Forwarding {
        let mut forwarding = view(self.read::<Saved>(FILE).as_ref());
        if let Some(confirmation) = &mut forwarding.confirmation {
            confirmation.requester_known = self.requester_known(confirmation.requester.as_deref());
        }
        forwarding
    }

    /// The latest box in this phone's inbox, opened and recorded; None without an address, a
    /// box, or one that opens. An address the edge no longer knows is forgotten here.
    async fn pull(&self, flow: &str) -> Result<Option<Payload>, CoreError> {
        let Some(saved) = self.read::<Saved>(FILE) else {
            return Err(CoreError::ForwardingOff {
                reason: "Mail forwarding is off.".into(),
            });
        };
        let body = json!({ "id": saved.id });
        // While the inbox is watched (an OTP wait) the client answers this from that
        // subscription, with no trip to the server.
        let inbox = match self
            .device_call(flow, Kind::Query, "mail:inbox", body)
            .await
        {
            Ok(inbox) => inbox,
            // Not on this address (any more): kept from v2 and not rejoined yet, or the server
            // moved the student's phones to their old address. Joined where the student's
            // address is; forgotten only when they have none (deleted) or it can't be asked.
            Err(error) if error.code() == GONE => {
                let rejoined = Box::pin(self.rejoin(flow, &saved, !saved.joined)).await;
                return Err(match rejoined {
                    Ok(Rejoined::Joined) => CoreError::Other {
                        reason: "Mail forwarding reconnected. Try again.".into(),
                    },
                    Ok(Rejoined::Later) if !saved.joined => CoreError::ForwardingOff {
                        reason: "Mail forwarding reconnects once you're signed in.".into(),
                    },
                    Ok(Rejoined::Later | Rejoined::NoAddress) => {
                        self.forget_forwarding(&saved.id);
                        CoreError::ForwardingOff {
                            reason: "Your forwarding address no longer exists. Set it up again."
                                .into(),
                        }
                    }
                    Err(error) => error,
                });
            }
            Err(error) if error.code() == REMOVED => {
                self.forget_forwarding(&saved.id);
                return Err(CoreError::ForwardingOff {
                    reason: "This phone was removed from your account. Sign in again.".into(),
                });
            }
            Err(error) => return Err(error.into()),
        };
        let seal_secret = &self.device().seal_secret;
        // Kept by the server since setup, for a phone that joined after Gmail sent it.
        if let Some(sealed) = inbox
            .get("confirmation")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            && let Some(confirmation) = open(seal_secret, &saved.id, sealed)
        {
            debug!("gmail confirmation kept on the edge");
            self.record(&saved.id, &confirmation);
        }
        if let Some(at) = inbox.get("lastOtpAt").and_then(Value::as_i64) {
            self.record_otp_at(&saved.id, at);
        }
        let Some(sealed) = inbox
            .get("sealed")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        else {
            debug!("inbox empty");
            return Ok(None);
        };
        let Some(payload) = open(seal_secret, &saved.id, sealed) else {
            return Ok(None);
        };
        info!(
            kind = payload.kind(),
            sent_at = payload.sent_at(),
            "forwarded mail"
        );
        self.record(&saved.id, &payload);
        Ok(Some(payload))
    }

    /// Notes that an OTP sent at `at` reached the address `id` (if it is still this phone's).
    fn record_otp_at(&self, id: &str, at: i64) {
        let _lock = FILE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(mut saved) = self.read::<Saved>(FILE).filter(|saved| saved.id == id) else {
            return;
        };
        let before = saved.clone();
        saved.ready = true;
        saved.last_otp_at = saved.last_otp_at.max(Some(at));
        saved.last_mail_at = saved.last_mail_at.max(Some(at));
        saved.drop_stale_confirmation();
        if saved != before {
            self.write(FILE, &saved);
        }
    }

    /// Notes that `payload` reached the address `id` (if it is still this phone's).
    fn record(&self, id: &str, payload: &Payload) {
        let _lock = FILE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(mut saved) = self.read::<Saved>(FILE).filter(|saved| saved.id == id) else {
            return;
        };
        let before = saved.clone();
        let sent_at = payload.sent_at();
        // Gmail's confirmation mail only shows the address was added; forwarding works once
        // Gmail forwards VTOP's mail, after the student confirmed and made the filter.
        saved.ready |= matches!(payload, Payload::Otp { .. });
        saved.last_mail_at = saved.last_mail_at.max(Some(sent_at));
        match payload {
            Payload::Otp { .. } => saved.last_otp_at = saved.last_otp_at.max(Some(sent_at)),
            // Older than an OTP that already came: forwarding works, so it isn't shown again.
            Payload::GmailConfirm { sent_at, .. } if saved.last_otp_at >= Some(*sent_at) => {}
            Payload::GmailConfirm {
                code,
                link,
                requester,
                sent_at,
            } => {
                saved.confirmation = Some(GmailConfirmation {
                    code: code.clone(),
                    link: link.as_deref().and_then(gmail_link),
                    requester: requester.clone(),
                    sent_at: *sent_at,
                    requester_known: None,
                });
            }
        }
        saved.drop_stale_confirmation();
        if saved != before {
            self.write(FILE, &saved);
        }
    }

    /// Forgets this phone's address, whichever it is: the server deleted it with the account.
    pub(crate) fn forget_any_forwarding(&self) {
        if let Some(saved) = self.read::<Saved>(FILE) {
            self.forget_forwarding(&saved.id);
        }
    }

    pub(crate) fn forget_forwarding(&self, id: &str) {
        let _lock = FILE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        if self.read::<Saved>(FILE).is_some_and(|saved| saved.id == id) {
            #[expect(
                clippy::let_underscore_must_use,
                reason = "a file already gone is the goal"
            )]
            let _ = fs::remove_file(self.at(FILE));
            info!("forwarding forgotten");
        }
    }
}

fn view(saved: Option<&Saved>) -> Forwarding {
    Forwarding {
        state: match saved {
            None => ForwardingState::Off,
            Some(saved) if saved.ready => ForwardingState::On,
            Some(_) => ForwardingState::Waiting,
        },
        address: saved.map(|saved| saved.address.clone()),
        sender: VTOP_SENDER.to_string(),
        confirmation: saved.and_then(|saved| saved.confirmation.clone()),
        last_mail_at: saved.and_then(|saved| saved.last_mail_at),
        last_otp_at: saved.and_then(|saved| saved.last_otp_at),
    }
}

#[cfg(test)]
mod tests;
