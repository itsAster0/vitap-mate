//! Mail forwarding (the Flutter app's `mail_forwarding_service.dart`): Gmail forwards VTOP's OTP
//! mail to `otp+<id>@<domain>`; the edge Worker reads it, seals the code to this install's
//! X25519 seal key (`device.json`, see [`crate::device`]; `vtop_shared::mail_seal`, the mailbox
//! id as associated data) and keeps the latest box for 5 minutes, which the app pulls from
//! `/v1/mail-otp/inbox`. Every call is signed as this install ([`Core::edge_post`]); the edge
//! knows the mailbox's members by device id, so `forwarding.json` holds no secret. Codes and
//! mail contents are never logged.
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

use crate::{Core, CoreError, traced, traced_sync};

pub(crate) const FILE: &str = "forwarding.json";
/// VTOP's OTP sender: the Gmail filter's From.
pub const VTOP_SENDER: &str = "noreply.sdc@vitap.ac.in";
/// The sign-in's time (VTOP's `Date:` just before it mailed the code) and the mail's `Date:`,
/// both VTOP's clock in whole seconds: an earlier sign-in's code is older than this.
const VTOP_SLACK_SECS: i64 = 3;
/// With no challenge to go by, the caller's time is the phone's clock, which may be off.
const CLOCK_SKEW_SECS: i64 = 60;

/// Serialises read-modify-write of [`FILE`].
pub(crate) static FILE_LOCK: Mutex<()> = Mutex::new(());

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
    #[serde(default)]
    owner_claimed: bool,
    /// Mail arrived: Gmail's confirmation, or an OTP (now or, as the edge says, before).
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

/// Mail forwarding calls; each takes the caller's flow id first.
#[uniffi::export]
impl Core {
    /// This phone's forwarding address and how far its setup got. No network.
    pub fn forwarding_status(&self, flow: String) -> Forwarding {
        traced_sync("forwarding_status", &flow, || Ok(self.forwarding_view()))
            .unwrap_or_else(|_| view(None))
    }

    /// Gives this phone a forwarding address. A student who already has one (made on another
    /// phone; the edge knows it by their VTOP session) joins it, so their Gmail filter keeps
    /// working; otherwise a new address is made and tied to them. Does nothing when this phone
    /// already has one.
    pub async fn set_up_forwarding(&self, flow: String) -> Result<Forwarding, CoreError> {
        traced("set_up_forwarding", &flow, async {
            if self.read::<Saved>(FILE).is_some() {
                info!("already set up");
                return Ok(self.forwarding_view());
            }
            let server = self.forwarding_server();
            let action = "Set up mail forwarding";
            // An address made without the student's session (or account token) is never tied to
            // them: their next phone could not join it and the server could not sign in with it.
            let mut joined = None;
            let reclaim = self.account_post(
                &server,
                "/v1/mail-otp/reclaim",
                &flow,
                action,
                true,
                json!({}),
            );
            match reclaim.await? {
                Ok(body) => joined = Some(body),
                Err(error) if error.code() == GONE => info!("no address yet: making one"),
                Err(error) => return Err(error.into()),
            }
            let reclaimed = joined.is_some();
            let body = match joined {
                Some(body) => body,
                None => {
                    self.edge_post(&server, "/v1/mail-otp/register", &flow, &json!({}))
                        .await?
                }
            };
            let mut saved = Saved {
                server: server.clone(),
                id: string(&body, "id")?,
                address: string(&body, "address")?,
                owner_claimed: reclaimed,
                // Even a joined address may never have been confirmed in Gmail: the next pull
                // brings Gmail's confirmation or the latest OTP, which tells.
                ready: false,
                last_mail_at: None,
                last_otp_at: None,
                confirmation: None,
            };
            self.write(FILE, &saved);
            info!(reclaimed, "forwarding address set up");
            // Best effort: lets the student's next phone join it, and the server sign in with it.
            if !reclaimed {
                let owner = json!({ "id": saved.id });
                let path = "/v1/mail-otp/owner";
                match self
                    .account_post(&server, path, &flow, action, true, owner)
                    .await
                {
                    Ok(Ok(_)) => {
                        saved.owner_claimed = true;
                        self.write(FILE, &saved);
                    }
                    Ok(Err(error)) => warn!(code = error.code(), "owner not claimed"),
                    Err(error) => warn!(%error, "owner not claimed"),
                }
            }
            Ok(self.forwarding_view())
        })
        .await
    }

    /// Pulls the latest forwarded mail (the edge keeps it 5 minutes): Gmail's confirmation while
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
}

impl Core {
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
            .edge_post(&saved.server, "/v1/mail-otp/delete", flow, &body)
            .await
        {
            Ok(_) => info!(everywhere, "left the address"),
            Err(error) if error.code() == GONE => info!("address already gone"),
            Err(error) => return Err(error.into()),
        }
        self.forget_forwarding(&saved.id);
        Ok(())
    }

    fn forwarding_view(&self) -> Forwarding {
        view(self.read::<Saved>(FILE).as_ref())
    }

    /// The latest box in this phone's inbox, opened and recorded; None without an address, a
    /// box, or one that opens. An address the edge no longer knows is forgotten here.
    async fn pull(&self, flow: &str) -> Result<Option<Payload>, CoreError> {
        let Some(saved) = self.read::<Saved>(FILE) else {
            return Err(CoreError::Other {
                reason: "Mail forwarding is off.".into(),
            });
        };
        let body = json!({ "id": saved.id });
        let inbox = match self
            .edge_post(&saved.server, "/v1/mail-otp/inbox", flow, &body)
            .await
        {
            Ok(inbox) => inbox,
            Err(error) if error.code() == GONE => {
                self.forget_forwarding(&saved.id);
                return Err(CoreError::Other {
                    reason: "Your forwarding address no longer exists. Set it up again.".into(),
                });
            }
            Err(error) => return Err(error.into()),
        };
        let seal_secret = &self.device().seal_secret;
        // Kept by the edge since setup, for a phone that joined after Gmail sent it.
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
        saved.ready = true;
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
                    link: link.clone(),
                    requester: requester.clone(),
                    sent_at: *sent_at,
                });
            }
        }
        saved.drop_stale_confirmation();
        if saved != before {
            self.write(FILE, &saved);
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
