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
//! address receives mail; once it arrives, the student can mark setup done without waiting for
//! VTOP's next OTP.

use std::fs;
use std::sync::{Arc, PoisonError};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{debug, info, warn};
use vtop_shared::mail_seal;

use vtop_core::spec;
use vtop_core::types::StudentProfile;

use crate::remote::backend::Kind;
use crate::{Core, CoreError, ForwardingOffKind, PAGES, Slot, traced, traced_sync};

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

/// How far mail forwarding's setup has progressed; owned by Rust so the UI,
/// OTP auto-fill and background sync agree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ForwardingReadiness {
    /// No address on this phone.
    Off,
    /// Address exists, waiting for Gmail's confirmation mail (or a v2 rejoin).
    NeedsConfirmation,
    /// Gmail's confirmation arrived; waiting for the filter (mark done or first OTP).
    NeedsFilter,
    /// Forwarding is active: an OTP came through or the student marked setup done.
    Ready,
}

/// This phone's forwarding address and how far its setup got, for the app.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum Forwarding {
    /// No address on this phone.
    Off {
        /// What the Gmail filter matches in From.
        sender: String,
        readiness: ForwardingReadiness,
    },
    /// An address, not finished setting up in Gmail yet (or kept from v2 and not rejoined).
    Waiting {
        address: String,
        sender: String,
        /// The latest Gmail confirmation that arrived.
        confirmation: Option<GmailConfirmation>,
        /// Unix seconds of the latest forwarded mail of either kind.
        last_mail_at: Option<i64>,
        readiness: ForwardingReadiness,
    },
    /// Forwarding is set up: an OTP came through, or the student marked setup done after
    /// Gmail's confirmation arrived.
    On {
        address: String,
        sender: String,
        /// Unix seconds of the latest forwarded OTP; None when marked done before one came.
        last_otp_at: Option<i64>,
        last_mail_at: Option<i64>,
        readiness: ForwardingReadiness,
    },
}

/// Where this phone's address lives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Mailbox {
    /// The server the address lives on.
    pub(crate) server: String,
    pub(crate) id: String,
    pub(crate) address: String,
}

/// How far an address's Gmail setup got.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Setup {
    /// Not known to work yet; Gmail's confirmation shows once it arrives.
    Waiting {
        confirmation: Option<GmailConfirmation>,
        last_mail_at: Option<i64>,
    },
    /// Forwarding works: an OTP came through (now or, as the server says, before), or the
    /// student marked setup done after Gmail's confirmation arrived. Gmail's confirmation is
    /// only for setting up, so none is kept here.
    Active {
        last_mail_at: Option<i64>,
        last_otp_at: Option<i64>,
    },
}

impl Setup {
    const NEW: Self = Self::Waiting {
        confirmation: None,
        last_mail_at: None,
    };

    /// An OTP sent at `at` reached the address.
    fn otp_at(self, at: i64) -> Self {
        let (last_mail_at, last_otp_at) = match self {
            Self::Waiting { last_mail_at, .. } => (last_mail_at, None),
            Self::Active {
                last_mail_at,
                last_otp_at,
            } => (last_mail_at, last_otp_at),
        };
        Self::Active {
            last_mail_at: last_mail_at.max(Some(at)),
            last_otp_at: last_otp_at.max(Some(at)),
        }
    }

    /// `payload` reached the address.
    fn mail(self, payload: &Payload) -> Self {
        let Payload::GmailConfirm {
            code,
            link,
            requester,
            sent_at,
        } = payload
        else {
            return self.otp_at(payload.sent_at());
        };
        match self {
            // Gmail's confirmation mail only shows the address was added; forwarding works once
            // Gmail forwards VTOP's mail or the student marks setup done.
            Self::Waiting { last_mail_at, .. } => Self::Waiting {
                confirmation: Some(GmailConfirmation {
                    code: code.clone(),
                    link: link.as_deref().and_then(gmail_link),
                    requester: requester.clone(),
                    sent_at: *sent_at,
                    requester_known: None,
                }),
                last_mail_at: last_mail_at.max(Some(*sent_at)),
            },
            // Working already: the server keeps the confirmation 24 h until an OTP arrives, and
            // it isn't shown again.
            Self::Active {
                last_mail_at,
                last_otp_at,
            } => Self::Active {
                last_mail_at: last_mail_at.max(Some(*sent_at)),
                last_otp_at,
            },
        }
    }

    fn readiness(&self) -> ForwardingReadiness {
        match self {
            Self::Active { .. } => ForwardingReadiness::Ready,
            Self::Waiting {
                confirmation,
                last_mail_at,
            } if confirmation.is_some() || last_mail_at.is_some() => {
                ForwardingReadiness::NeedsFilter
            }
            Self::Waiting { .. } => ForwardingReadiness::NeedsConfirmation,
        }
    }

    /// The student says the Gmail filter is made: only once Gmail's mail has reached the address.
    fn mark_done(self) -> Self {
        match self {
            Self::Waiting { last_mail_at, .. }
                if self.readiness() == ForwardingReadiness::NeedsFilter =>
            {
                Self::Active {
                    last_mail_at,
                    last_otp_at: None,
                }
            }
            other => other,
        }
    }
}

/// This phone's address (`forwarding.json`, as [`Stored`]).
#[derive(Debug, Clone, PartialEq, Eq)]
enum MailboxState {
    Off,
    /// Kept from the v2 edge: this install isn't on the address as backend v3 knows it, so
    /// nothing is forwarded to it until [`Core::keep_forwarding`] proves the student and rejoins.
    /// `kept` is the setup it goes back to.
    NeedsRejoin {
        mailbox: Mailbox,
        kept: Setup,
    },
    Joined {
        mailbox: Mailbox,
        setup: Setup,
    },
}

impl MailboxState {
    fn mailbox(&self) -> Option<&Mailbox> {
        match self {
            Self::Off => None,
            Self::NeedsRejoin { mailbox, .. } | Self::Joined { mailbox, .. } => Some(mailbox),
        }
    }

    fn id(&self) -> Option<&str> {
        self.mailbox().map(|mailbox| mailbox.id.as_str())
    }

    fn readiness(&self) -> ForwardingReadiness {
        match self {
            Self::Off => ForwardingReadiness::Off,
            // Nothing reaches this install until it rejoins.
            Self::NeedsRejoin { .. } => ForwardingReadiness::NeedsConfirmation,
            Self::Joined { setup, .. } => setup.readiness(),
        }
    }

    /// `next` applied to the setup of the address `id`, if it is still this phone's.
    fn with_setup(self, id: &str, next: impl FnOnce(Setup) -> Setup) -> Self {
        match self {
            Self::NeedsRejoin { mailbox, kept } if mailbox.id == id => Self::NeedsRejoin {
                mailbox,
                kept: next(kept),
            },
            Self::Joined { mailbox, setup } if mailbox.id == id => Self::Joined {
                mailbox,
                setup: next(setup),
            },
            other => other,
        }
    }

    /// On `to` after the server rejoined this phone, which was on `from`; None when it isn't on
    /// `from` any more (turned off or signed out meanwhile).
    fn rejoined(&self, from: &str, to: Mailbox) -> Option<Self> {
        let (Self::NeedsRejoin {
            mailbox,
            kept: setup,
        }
        | Self::Joined { mailbox, setup }) = self
        else {
            return None;
        };
        if mailbox.id != from {
            return None;
        }
        // The student's account holds another address: mail goes there now, so its Gmail setup
        // has to be checked again (and the old one's times don't apply).
        let setup = if to.id == mailbox.id {
            setup.clone()
        } else {
            warn!("rejoined a different address");
            Setup::NEW
        };
        Some(Self::Joined { mailbox: to, setup })
    }

    /// The file for this state; None for [`MailboxState::Off`], which has none.
    fn stored(&self) -> Option<Stored> {
        let (mailbox, setup, joined) = match self {
            Self::Off => return None,
            Self::NeedsRejoin { mailbox, kept } => (mailbox, kept, false),
            Self::Joined { mailbox, setup } => (mailbox, setup, true),
        };
        let (ready, last_mail_at, last_otp_at, confirmation) = match setup {
            Setup::Waiting {
                confirmation,
                last_mail_at,
            } => (false, *last_mail_at, None, confirmation.clone()),
            Setup::Active {
                last_mail_at,
                last_otp_at,
            } => (true, *last_mail_at, *last_otp_at, None),
        };
        Some(Stored {
            server: mailbox.server.clone(),
            id: mailbox.id.clone(),
            address: mailbox.address.clone(),
            owner_claimed: false,
            joined,
            ready,
            last_mail_at,
            last_otp_at,
            confirmation,
        })
    }
}

/// `forwarding.json` as earlier versions wrote it, and still written so (an older app keeps
/// reading it). A file from before signed requests (it held a mailbox secret and its own key)
/// does not parse, so it reads as no address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    server: String,
    id: String,
    address: String,
    /// From v2, where the student's first address claimed its owner hash. Unused, and read only
    /// so a file a v2 install wrote still parses (unknown fields are refused).
    #[serde(default, skip_serializing)]
    owner_claimed: bool,
    #[serde(default)]
    joined: bool,
    #[serde(default)]
    ready: bool,
    #[serde(default)]
    last_mail_at: Option<i64>,
    #[serde(default)]
    last_otp_at: Option<i64>,
    #[serde(default)]
    confirmation: Option<GmailConfirmation>,
}

impl TryFrom<Stored> for MailboxState {
    type Error = &'static str;

    fn try_from(stored: Stored) -> Result<Self, Self::Error> {
        if stored.id.is_empty() || stored.address.is_empty() {
            return Err("forwarding.json has no address");
        }
        let mailbox = Mailbox {
            server: stored.server,
            id: stored.id,
            address: stored.address,
        };
        // An OTP that came through shows forwarding works, whatever `ready` says.
        let setup = if stored.ready || stored.last_otp_at.is_some() {
            Setup::Active {
                last_mail_at: stored.last_mail_at,
                last_otp_at: stored.last_otp_at,
            }
        } else {
            Setup::Waiting {
                confirmation: stored.confirmation,
                last_mail_at: stored.last_mail_at,
            }
        };
        Ok(if stored.joined {
            Self::Joined { mailbox, setup }
        } else {
            Self::NeedsRejoin {
                mailbox,
                kept: setup,
            }
        })
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
            .unwrap_or_else(|_| view(&MailboxState::Off))
    }

    /// Gives this phone a forwarding address. A student who already has one (made on another
    /// phone) joins it, so their Gmail filter keeps working; otherwise a new address is made for
    /// them. Does nothing when this phone already has one.
    pub async fn set_up_forwarding(&self, flow: String) -> Result<Forwarding, CoreError> {
        traced("set_up_forwarding", &flow, async {
            let slot = self.current()?;
            // A signed-out student's address first, as it may be the one this sign-in joins.
            self.leave_queued(&flow).await;
            if self.mailbox_state() != MailboxState::Off {
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
            let mailbox = Mailbox {
                server,
                id: string(&body, "id")?,
                address: string(&body, "address")?,
            };
            self.save_new_mailbox(&slot, mailbox)?;
            Ok(self.forwarding_view())
        })
        .await
    }

    /// Pulls the latest forwarded mail (the server keeps it 5 minutes): Gmail's confirmation while
    /// setting up, or an OTP.
    pub async fn check_forwarding(&self, flow: String) -> Result<Forwarding, CoreError> {
        traced("check_forwarding", &flow, async {
            self.pull(&flow).await?;
            Ok(self.forwarding_view())
        })
        .await
    }

    /// Marks forwarding set up once Gmail's confirmation has reached the address: VTOP doesn't
    /// send an OTP on every sign-in, so the app can't verify the Gmail filter right away. Does
    /// nothing before confirmation (or without an address). No network.
    pub fn mark_forwarding_done(&self, flow: String) -> Forwarding {
        traced_sync("mark_forwarding_done", &flow, || {
            self.update_mailbox(|state| match state {
                MailboxState::Joined { mailbox, setup } => {
                    let setup = setup.mark_done();
                    if matches!(setup, Setup::Active { .. }) {
                        info!("forwarding marked done");
                    }
                    MailboxState::Joined { mailbox, setup }
                }
                // Not rejoined yet: nothing reaches this install to mark done over.
                other => other,
            });
            Ok(self.forwarding_view())
        })
        .unwrap_or_else(|_| view(&MailboxState::Off))
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
            let MailboxState::NeedsRejoin { mailbox, .. } = self.mailbox_state() else {
                return Ok(());
            };
            if mailbox.server != self.forwarding_server() {
                return Ok(());
            }
            // At an open nobody asked for forwarding: no sheet, only an answer given already.
            let prove = self.identity_shared().await;
            self.rejoin(&flow, &mailbox.id, true, prove).await.map(drop)
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
    /// The saved address; Off without one (or with a file that doesn't read).
    fn mailbox_state(&self) -> MailboxState {
        let Some(stored) = self.read::<Stored>(FILE) else {
            return MailboxState::Off;
        };
        MailboxState::try_from(stored).unwrap_or_else(|error| {
            warn!(error, "forwarding.json not used");
            MailboxState::Off
        })
    }

    /// Saves `state`; Off removes the file. The caller holds the forwarding lock (`Locks::forwarding`).
    fn store_mailbox(&self, state: &MailboxState) {
        match state.stored() {
            Some(stored) => self.write(FILE, &stored),
            None => self.remove_file(FILE),
        }
    }

    /// The saved state through `next`, under the forwarding lock; written only when it changed.
    fn update_mailbox(&self, next: impl FnOnce(MailboxState) -> MailboxState) -> MailboxState {
        let _lock = self
            .shared
            .locks
            .forwarding
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let before = self.mailbox_state();
        let after = next(before.clone());
        if after != before {
            self.store_mailbox(&after);
        }
        after
    }

    /// Saves the address `mail:register` gave while `slot` was signed in, unless this phone got
    /// one meanwhile.
    fn save_new_mailbox(&self, slot: &Arc<Slot>, mailbox: Mailbox) -> Result<(), CoreError> {
        let _lock = self
            .shared
            .locks
            .forwarding
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if !self.is_current(slot) {
            // Signed out (or another student signed in) during the call: this install is on the
            // address now, and the next student mustn't get its OTPs.
            info!("signed out while setting up: address queued to leave");
            self.queue_leave_of(&mailbox);
            return Err(CoreError::NotSignedIn);
        }
        if self.mailbox_state() != MailboxState::Off {
            info!("set up meanwhile");
            return Ok(());
        }
        // Even a joined address may never have been confirmed in Gmail: the next pull brings
        // Gmail's confirmation or the latest OTP, which tells.
        self.store_mailbox(&MailboxState::Joined {
            mailbox,
            setup: Setup::NEW,
        });
        info!("forwarding address set up");
        Ok(())
    }

    /// Joins the student's address from this install, which was on `from`, with the identity it
    /// has (a v2 account token is traded in). With `prove`, an install with none proves who the
    /// student is first (the proof sends no cookie, so nothing is asked); without, it waits for
    /// an identity.
    async fn rejoin(
        &self,
        flow: &str,
        from: &str,
        make: bool,
        prove: bool,
    ) -> Result<Rejoined, CoreError> {
        let path = if make { "mail:register" } else { "mail:join" };
        let body = match self
            .account_call(
                flow,
                Kind::Mutation,
                path,
                "Keep mail forwarding",
                prove,
                json!({}),
            )
            .await
        {
            Ok(answer) => answer?,
            Err(CoreError::ConsentDenied | CoreError::NotSignedIn | CoreError::RegnoNotShared) => {
                info!("no identity yet: rejoined later");
                return Ok(Rejoined::Later);
            }
            Err(error) => return Err(error),
        };
        if body.is_null() {
            info!("the student has no address any more");
            return Ok(Rejoined::NoAddress);
        }
        let to = Mailbox {
            server: self.forwarding_server(),
            id: string(&body, "id")?,
            address: string(&body, "address")?,
        };
        let mut joined = false;
        self.update_mailbox(|state| match state.rejoined(from, to) {
            Some(next) => {
                joined = true;
                next
            }
            None => state,
        });
        if joined {
            info!("forwarding address rejoined");
            Ok(Rejoined::Joined)
        } else {
            info!("forwarding changed meanwhile, not rejoined");
            Ok(Rejoined::Later)
        }
    }

    /// Queues this phone's address to be left; called before the student's files are forgotten.
    pub(crate) fn queue_leave(&self) {
        // Not while a setup saves the address: it would be saved after this and then forgotten.
        let _lock = self
            .shared
            .locks
            .forwarding
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(mailbox) = self.mailbox_state().mailbox() {
            self.queue_leave_of(mailbox);
        }
    }

    /// Queues `mailbox` to be left.
    fn queue_leave_of(&self, mailbox: &Mailbox) {
        let _lock = self
            .shared
            .locks
            .forwarding_leave
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let mut queue = self.read::<Vec<Leave>>(LEAVE_FILE).unwrap_or_default();
        if !queue.iter().any(|leave| leave.id == mailbox.id) {
            queue.push(Leave {
                server: mailbox.server.clone(),
                id: mailbox.id.clone(),
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
        let mut done = Vec::new();
        for leave in queue {
            // Joined again since: this phone uses it. Read for each, as a sign-in may rejoin one
            // while earlier ones are left.
            if self.mailbox_state().id() == Some(leave.id.as_str()) {
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
        let _lock = self
            .shared
            .locks
            .forwarding_leave
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
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
        self.mailbox_state().mailbox().map(|mailbox| {
            (
                mailbox.server.clone(),
                mailbox.id.clone(),
                mailbox.address.clone(),
            )
        })
    }

    /// Takes this phone off its address (deleting it for every phone with `everywhere`) and
    /// forgets it here.
    pub(crate) async fn leave_forwarding(
        &self,
        flow: &str,
        everywhere: bool,
    ) -> Result<(), CoreError> {
        let Some(id) = self.mailbox_state().id().map(str::to_string) else {
            return Ok(());
        };
        let body = json!({ "id": id, "everywhere": everywhere });
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
        self.forget_forwarding(&id);
        Ok(())
    }

    fn forwarding_view(&self) -> Forwarding {
        let mut forwarding = view(&self.mailbox_state());
        if let Forwarding::Waiting {
            confirmation: Some(confirmation),
            ..
        } = &mut forwarding
        {
            confirmation.requester_known = self.requester_known(confirmation.requester.as_deref());
        }
        forwarding
    }

    /// The latest box in this phone's inbox, opened and recorded; None without an address, a
    /// box, or one that opens. An address the edge no longer knows is forgotten here.
    async fn pull(&self, flow: &str) -> Result<Option<Payload>, CoreError> {
        let mut rejoined = false;
        loop {
            let state = self.mailbox_state();
            let joined = matches!(state, MailboxState::Joined { .. });
            let Some(id) = state.id().map(str::to_string) else {
                return Err(CoreError::ForwardingOff {
                    kind: ForwardingOffKind::NotSetUp,
                    reason: "Mail forwarding is off.".into(),
                });
            };
            let body = json!({ "id": id });
            // While the inbox is watched (an OTP wait) the client answers this from that
            // subscription, with no trip to the server.
            let error = match self
                .device_call(flow, Kind::Query, "mail:inbox", body)
                .await
            {
                Ok(inbox) => return Ok(self.open_inbox(&id, &inbox)),
                Err(error) => error,
            };
            return Err(match error.code() {
                // Not on this address (any more): kept from v2 and not rejoined yet, or the
                // server moved the student's phones to their old address. Joined where the
                // student's address is, then read there; forgotten only when they have none
                // (deleted) or it can't be asked.
                GONE if !rejoined => match Box::pin(self.rejoin(flow, &id, !joined, false)).await {
                    Ok(Rejoined::Joined) => {
                        rejoined = true;
                        continue;
                    }
                    Ok(Rejoined::Later) if !joined => CoreError::ForwardingOff {
                        kind: ForwardingOffKind::NeedsSignIn,
                        reason: "Mail forwarding reconnects once you're signed in.".into(),
                    },
                    Ok(Rejoined::Later | Rejoined::NoAddress) => {
                        self.forget_forwarding(&id);
                        CoreError::ForwardingOff {
                            kind: ForwardingOffKind::AddressDeleted,
                            reason: "Your forwarding address no longer exists. Set it up again."
                                .into(),
                        }
                    }
                    Err(error) => error,
                },
                // Rejoined, and the server still doesn't know this install on the address.
                GONE => CoreError::Other {
                    reason: "Mail forwarding reconnected. Try again.".into(),
                },
                // No identity on this install yet (updated from v2 without a live account
                // token): the app proves and rejoins once signed in (`keep_forwarding`).
                "unauthenticated" => CoreError::ForwardingOff {
                    kind: ForwardingOffKind::NeedsSignIn,
                    reason: "Mail forwarding reconnects once you're signed in.".into(),
                },
                REMOVED => {
                    self.forget_forwarding(&id);
                    CoreError::ForwardingOff {
                        kind: ForwardingOffKind::DeviceRemoved,
                        reason: "This phone was removed from your account. Sign in again.".into(),
                    }
                }
                _ => error.into(),
            });
        }
    }

    /// Opens and records `mail:inbox`'s answer for the address `id`: the latest box, if one
    /// opens.
    fn open_inbox(&self, id: &str, inbox: &Value) -> Option<Payload> {
        let seal_secret = &self.device().seal_secret;
        // Kept by the server since setup, for a phone that joined after Gmail sent it.
        if let Some(sealed) = inbox
            .get("confirmation")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            && let Some(confirmation) = open(seal_secret, id, sealed)
        {
            debug!("gmail confirmation kept on the edge");
            self.record(id, &confirmation);
        }
        if let Some(at) = inbox.get("lastOtpAt").and_then(Value::as_i64) {
            self.update_mailbox(|state| state.with_setup(id, |setup| setup.otp_at(at)));
        }
        let Some(sealed) = inbox
            .get("sealed")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        else {
            debug!("inbox empty");
            return None;
        };
        let payload = open(seal_secret, id, sealed)?;
        info!(
            kind = payload.kind(),
            sent_at = payload.sent_at(),
            "forwarded mail"
        );
        self.record(id, &payload);
        Some(payload)
    }

    /// Notes that `payload` reached the address `id` (if it is still this phone's).
    fn record(&self, id: &str, payload: &Payload) {
        self.update_mailbox(|state| state.with_setup(id, |setup| setup.mail(payload)));
    }

    /// Forgets this phone's address, whichever it is: the server deleted it with the account.
    pub(crate) fn forget_any_forwarding(&self) {
        if let Some(id) = self.mailbox_state().id() {
            self.forget_forwarding(id);
        }
    }

    pub(crate) fn forget_forwarding(&self, id: &str) {
        self.update_mailbox(|state| {
            if state.id() == Some(id) {
                info!("forwarding forgotten");
                MailboxState::Off
            } else {
                state
            }
        });
    }
}

fn view(state: &MailboxState) -> Forwarding {
    let sender = VTOP_SENDER.to_string();
    let readiness = state.readiness();
    match state {
        MailboxState::Off => Forwarding::Off { sender, readiness },
        // Nothing is forwarded to this install until it rejoins, so it isn't on yet.
        MailboxState::NeedsRejoin { mailbox, .. } => Forwarding::Waiting {
            address: mailbox.address.clone(),
            sender,
            confirmation: None,
            last_mail_at: None,
            readiness,
        },
        MailboxState::Joined {
            mailbox,
            setup:
                Setup::Waiting {
                    confirmation,
                    last_mail_at,
                },
        } => Forwarding::Waiting {
            address: mailbox.address.clone(),
            sender,
            confirmation: confirmation.clone(),
            last_mail_at: *last_mail_at,
            readiness,
        },
        MailboxState::Joined {
            mailbox,
            setup:
                Setup::Active {
                    last_mail_at,
                    last_otp_at,
                },
        } => Forwarding::On {
            address: mailbox.address.clone(),
            sender,
            last_otp_at: *last_otp_at,
            last_mail_at: *last_mail_at,
            readiness,
        },
    }
}

#[cfg(test)]
mod tests;
