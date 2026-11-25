//! Connected apps (Settings → Account): the access keys an MCP client or the browser extension
//! use, and this phone's answers to their session requests (`keys:*`, `sessions:answer`). Every
//! call goes as the student this install proved to be ([`Core::account_call`]); proving links
//! the phone to the account. A new key is shown once and never logged; nor are cookies or
//! pairing codes.

use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

use crate::device::EdgeError;
use crate::remote::backend::Kind;
use crate::remote::consent;
use crate::remote::handover::{AskerIp, HandoverAsk, HandoverAsker, HandoverStatus};
use crate::remote::server::{PhoneWire, ServerStorage, unexpected};
use crate::remote::server_code::ServerCode;
use crate::{Core, CoreError, dev, traced};

/// The label of a key made by pairing the browser extension; [`KeyKind`] is read from it.
pub const BROWSER_LABEL: &str = "Browser";
/// The label of a key made for an MCP client.
const MCP_LABEL: &str = "MCP";
/// Crockford base32, the alphabet the extension draws pairing codes from (`backend/convex/lib/pairing.ts`).
const PAIR_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const PAIR_LENGTH: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum KeyKind {
    Mcp,
    Browser,
}

/// A connected app's request for this phone's VTOP session, waiting for an answer.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SessionRequest {
    pub request_id: String,
    /// Unix ms; None from a server before it said.
    pub asked_at: Option<i64>,
    /// Unix ms.
    pub expires_at: i64,
    /// Whether the app also asks for the saved password.
    pub want_credentials: bool,
    /// Who asked; None for a request the server kept no asker for.
    pub asker: Option<HandoverAsker>,
}

/// `sessions:pending`'s item as sent.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionRequestWire {
    request_id: String,
    asked_at: Option<i64>,
    expires_at: i64,
    want_credentials: bool,
    asker: Option<AskerWire>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AskerWire {
    key_label: String,
    via: ViaWire,
    ip: Option<AskerIpWire>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ViaWire {
    Extension,
    Mcp,
}

#[derive(Deserialize)]
struct AskerIpWire {
    address: String,
    seen: bool,
}

impl From<SessionRequestWire> for SessionRequest {
    fn from(wire: SessionRequestWire) -> Self {
        Self {
            request_id: wire.request_id,
            asked_at: wire.asked_at,
            expires_at: wire.expires_at,
            want_credentials: wire.want_credentials,
            asker: wire.asker.map(|asker| HandoverAsker {
                key_label: asker.key_label,
                via: match asker.via {
                    ViaWire::Extension => KeyKind::Browser,
                    ViaWire::Mcp => KeyKind::Mcp,
                },
                ip: (asker.ip).map(|ip| AskerIp {
                    address: ip.address,
                    seen: ip.seen,
                }),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct KeyUse {
    /// Unix seconds.
    pub at: i64,
    pub via: KeyKind,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AccessKey {
    /// The short id the server revokes it by.
    pub id: String,
    pub label: String,
    pub kind: KeyKind,
    /// Whether it may apply for and cancel outings.
    pub outings: bool,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    /// Newest first, at most 10.
    pub uses: Vec<KeyUse>,
}

/// Keys another phone answers for: they can be moved to this one ([`Core::move_keys`]).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct KeyPhone {
    pub owner: KeyOwner,
    pub keys: u32,
}

/// Who answers for a [`KeyPhone`]'s keys.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum KeyOwner {
    /// A linked phone: its short id (which [`Core::move_keys`] takes) and when it was seen
    /// (unix seconds).
    Phone {
        id: String,
        label: Option<String>,
        seen_at: i64,
    },
    /// Phones no longer linked, as one entry: nothing answers for their keys.
    Unlinked,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConnectedApps {
    /// This phone is one of the account's phones (it made a key, or linked before).
    pub linked: bool,
    pub keys: Vec<AccessKey>,
    /// The other phones with keys, most keys first; the unlinked ones as one entry.
    pub elsewhere: Vec<KeyPhone>,
    /// What the server stores for these apps (Session handover).
    pub storage: ServerStorage,
    /// The semester they read; None is the newest.
    pub semester_id: Option<String>,
    /// Whether the server can wake this phone for a session request.
    pub push: ServerPush,
}

/// Whether the server holds a push token for this phone, so a connected app's request reaches
/// it while the app is closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ServerPush {
    Token,
    NoToken,
    /// The server couldn't say (an older server, or the read failed): no warning.
    Unknown,
}

/// The other phones that answer for keys, from the account view's `phones` and `keys`.
fn elsewhere(phones: &[PhoneWire], keys: &[KeyView]) -> Vec<KeyPhone> {
    let count = |phone: Option<&str>| {
        keys.iter()
            .filter(|key| match phone {
                Some(id) => key.phone_id.as_deref() == Some(id),
                None => !phones
                    .iter()
                    .any(|p| key.phone_id.as_deref() == Some(p.id.as_str())),
            })
            .count() as u32
    };
    let mut found: Vec<KeyPhone> = phones
        .iter()
        .filter(|phone| !phone.this_phone)
        .map(|phone| KeyPhone {
            owner: KeyOwner::Phone {
                id: phone.id.clone(),
                label: phone.label.clone(),
                seen_at: phone.seen_at,
            },
            keys: count(Some(&phone.id)),
        })
        .collect();
    found.push(KeyPhone {
        owner: KeyOwner::Unlinked,
        keys: count(None),
    });
    found.retain(|phone| phone.keys > 0);
    found.sort_by_key(|phone| std::cmp::Reverse(phone.keys));
    found
}

/// The MCP client (Claude and others) whose sign-in page shows a pairing code: asked before the
/// student approves, so the sheet can name it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PairApp {
    pub name: String,
    /// Where the client goes back to (`claude.ai`), from its redirect URI.
    pub host: String,
}

/// `pair:info`'s answer: an MCP client's sign-in page, or the extension's code.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum PairInfo {
    Mcp {
        name: String,
        host: String,
    },
    Browser,
    /// A kind from a newer server: no client to name.
    #[serde(other)]
    Other,
}

/// `keys:move`'s answer.
#[derive(Deserialize)]
struct Moved {
    moved: u32,
}

/// A key just made (`keys:create`'s answer): [`NewKey::key`] is shown once.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct NewKey {
    pub id: String,
    pub key: String,
    pub mcp_url: String,
}

/// The edge's view of a key (`AccountRecord.view`).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyView {
    id: String,
    label: String,
    created_at: i64,
    last_used_at: Option<i64>,
    /// The phone that answers for it.
    #[serde(default)]
    phone_id: Option<String>,
    #[serde(default)]
    outings: bool,
    #[serde(default)]
    uses: Vec<UseView>,
}

#[derive(Deserialize)]
struct UseView {
    at: i64,
    via: String,
}

impl From<KeyView> for AccessKey {
    fn from(view: KeyView) -> Self {
        let kind = |text: &str| match text {
            "extension" => KeyKind::Browser,
            _ => KeyKind::Mcp,
        };
        Self {
            kind: if view.label == BROWSER_LABEL {
                KeyKind::Browser
            } else {
                KeyKind::Mcp
            },
            uses: view
                .uses
                .iter()
                .map(|u| KeyUse {
                    at: u.at,
                    via: kind(&u.via),
                })
                .collect(),
            id: view.id,
            label: view.label,
            outings: view.outings,
            created_at: view.created_at,
            last_used_at: view.last_used_at,
        }
    }
}

/// A pairing code as typed or scanned (`ABCDE-12345`, or a link ending in `#ABCDE-12345`), the
/// way the edge reads it: case, dashes and spaces ignored, `O` as `0`, `I`/`L` as `1`. None when
/// it is not one.
#[uniffi::export]
pub fn pair_code(text: String) -> Option<String> {
    let text = text.trim();
    let text = text.rsplit_once('#').map_or(text, |(_, code)| code);
    let code: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            c => c,
        })
        .collect();
    (code.chars().count() == PAIR_LENGTH && code.chars().all(|c| PAIR_ALPHABET.contains(c)))
        .then_some(code)
}

/// `name=value; name=value` as the answer's cookie list.
fn cookie_list(header: &str) -> Vec<Value> {
    header
        .split(';')
        .filter_map(|pair| pair.split_once('='))
        .map(|(name, value)| (name.trim(), value.trim()))
        .filter(|(name, value)| !name.is_empty() && !value.is_empty())
        .map(|(name, value)| json!({ "name": name, "value": value, "domain": dev::COOKIE_DOMAIN }))
        .collect()
}

/// The server's refusals in the student's words.
pub(crate) fn refused(error: EdgeError) -> CoreError {
    let EdgeError::Status { code, .. } = &error else {
        return error.into();
    };
    let reason = match code {
        ServerCode::KeyLimit => "Revoke a key first: this account has the most keys allowed.",
        ServerCode::PairExpired => "That code has expired. Start connecting again in the app.",
        ServerCode::BadRequest => "Check the code: it's the 10 characters the extension shows.",
        ServerCode::KeyNotFound => "That key is already gone.",
        ServerCode::PhoneNotFound => "That phone is already gone.",
        ServerCode::SessionInvalid => "VTOP didn't accept this phone's session. Try again.",
        ServerCode::VtopUnreachable => "The server couldn't reach VTOP. Try again in a minute.",
        _ => return error.into(),
    };
    CoreError::Other {
        reason: reason.into(),
    }
}

/// Connected-apps calls; each takes the caller's flow id first.
#[uniffi::export]
impl Core {
    /// The account's access keys, whether this phone is linked to it, what the server stores
    /// for connected apps and whether it can wake this phone.
    pub async fn connected_apps(&self, flow: String) -> Result<ConnectedApps, CoreError> {
        traced("connected_apps", &flow, async {
            let view = self.account_view(&flow, "Load your connected apps").await?;
            let linked = view.this_phone;
            let elsewhere = if linked {
                elsewhere(&view.phones, &view.keys)
            } else {
                vec![]
            };
            let storage = view.storage();
            // After the account view, so an identity it proved for is there; never proves.
            let push = match self
                .device_call(&flow, Kind::Query, "devices:info", json!({}))
                .await
            {
                Ok(info) => match info.get("pushToken").and_then(Value::as_bool) {
                    Some(true) => ServerPush::Token,
                    Some(false) => ServerPush::NoToken,
                    None => ServerPush::Unknown,
                },
                Err(error) => {
                    info!(code = error.code(), "push token not read");
                    ServerPush::Unknown
                }
            };
            let keys: Vec<AccessKey> = view.keys.into_iter().map(AccessKey::from).collect();
            info!(
                linked,
                keys = keys.len(),
                elsewhere = elsewhere.len(),
                ?push,
                "connected apps"
            );
            Ok(ConnectedApps {
                linked,
                keys,
                elsewhere,
                storage,
                semester_id: view.semester_id,
                push,
            })
        })
        .await
    }

    /// Links this phone to the account without making a key (after a reinstall, so the keys
    /// made before show again).
    pub async fn link_phone(&self, flow: String) -> Result<(), CoreError> {
        traced("link_phone", &flow, async {
            let action = "Link this phone to your account";
            self.link_with(&flow, action).await
        })
        .await
    }

    /// Makes a key: for the extension that shows `pair_code` (it collects the key from the server),
    /// else for an MCP client.
    pub async fn create_key(
        &self,
        flow: String,
        outings: bool,
        pair_code: Option<String>,
    ) -> Result<NewKey, CoreError> {
        traced("create_key", &flow, async {
            self.create_key_with(&flow, "Make an access key", outings, pair_code)
                .await
        })
        .await
    }

    /// The MCP client behind a pairing code from its sign-in page; None for the extension's codes.
    pub async fn pair_app(&self, flow: String, code: String) -> Result<Option<PairApp>, CoreError> {
        traced("pair_app", &flow, async {
            self.pair_app_with(&flow, &code).await
        })
        .await
    }

    pub async fn revoke_key(&self, flow: String, id: String) -> Result<(), CoreError> {
        traced("revoke_key", &flow, async {
            let (path, action) = ("keys:revoke", "Revoke an access key");
            let body = json!({ "id": id });
            self.account_call(&flow, Kind::Mutation, path, action, true, body)
                .await?
                .map_err(refused)?;
            info!("key revoked");
            Ok(())
        })
        .await
    }

    /// Makes this phone answer for the keys of the phone `from` (which is unlinked) and of
    /// phones no longer linked; without `from`, only the latter. The number of keys moved.
    pub async fn move_keys(&self, flow: String, from: Option<String>) -> Result<u32, CoreError> {
        traced("move_keys", &flow, async {
            let (path, action) = ("keys:move", "Move keys to this phone");
            let mut body = json!({});
            if let Some(from) = from {
                crate::json_set(&mut body, "from", json!(from));
            }
            let answer = self
                .account_call(&flow, Kind::Mutation, path, action, true, body)
                .await?
                .map_err(refused)?;
            let Moved { moved } = serde_json::from_value(answer).map_err(unexpected)?;
            info!(moved, "keys moved");
            Ok(moved)
        })
        .await
    }

    pub async fn set_key_outings(
        &self,
        flow: String,
        id: String,
        outings: bool,
    ) -> Result<(), CoreError> {
        traced("set_key_outings", &flow, async {
            let (path, action) = ("keys:setOutings", "Change a key's outing access");
            let body = json!({ "id": id, "outings": outings });
            self.account_call(&flow, Kind::Mutation, path, action, true, body)
                .await?
                .map_err(refused)?;
            info!(outings, "key outings");
            Ok(())
        })
        .await
    }

    /// The connected apps' session requests waiting on this phone (`sessions:pending`, which
    /// [`LiveTopic::SessionRequests`](crate::remote::backend::LiveTopic) follows); none without
    /// an identity.
    pub async fn pending_session_requests(
        &self,
        flow: String,
    ) -> Result<Vec<SessionRequest>, CoreError> {
        traced("pending_session_requests", &flow, async {
            let answer = self
                .device_call(&flow, Kind::Query, "sessions:pending", json!({}))
                .await;
            let requests = match answer {
                Err(error) if error.is(&ServerCode::Unauthenticated) => Vec::new(),
                answer => {
                    serde_json::from_value::<Vec<SessionRequestWire>>(answer.map_err(refused)?)
                        .map_err(|_| CoreError::Other {
                            reason: "The server sent an unexpected answer.".into(),
                        })?
                        .into_iter()
                        .map(SessionRequest::from)
                        .collect()
                }
            };
            info!(count = requests.len(), "session requests");
            Ok(requests)
        })
        .await
    }
}

impl Core {
    /// The answer to a `vtop_cookie_request` ([`Core::answer_handover`]): this phone's live VTOP
    /// session, signing in again when it had expired, or why it can't. Only with the student on
    /// screen approving it on the sheet (or a remembered yes): from the background it is always
    /// the error. Once approved, the app hears how it goes ([`HandoverStatus`]). A failure is
    /// sent too, then returned.
    pub(crate) async fn answer_session_request(
        &self,
        flow: &str,
        request_id: &str,
    ) -> Result<(), CoreError> {
        let mut approved = false;
        let session = match self.current() {
            Err(error) => Err(error),
            Ok(_) => {
                let ask = self.handover_ask(flow, request_id).await;
                let session = (self.handover_cookies(flow, ask).await)
                    .and_then(|cookies| cookies.ok_or(CoreError::NotSignedIn));
                approved = session != Err(CoreError::ConsentDenied);
                session
            }
        };
        let delivered = self.answer_with(flow, request_id, session).await;
        if approved {
            let status = match &delivered {
                Ok(Delivered::Taken) => HandoverStatus::Sent,
                Ok(Delivered::Late) => HandoverStatus::Failed {
                    reason: "The app had stopped waiting.".into(),
                },
                Err(error) => HandoverStatus::Failed {
                    reason: why_not(error),
                },
            };
            consent::handover_status(&self.shared, flow, request_id, status);
        }
        delivered.map(|_| ())
    }

    /// What the approve sheet shows for `request_id`: who asked and when, as the server lists
    /// it. A request it no longer lists (or a failed read) is asked about without them.
    async fn handover_ask(&self, flow: &str, request_id: &str) -> HandoverAsk {
        let listed = match self.pending_session_requests(flow.to_string()).await {
            Ok(requests) => requests.into_iter().find(|r| r.request_id == request_id),
            Err(error) => {
                info!(%error, "session request details unavailable");
                None
            }
        };
        HandoverAsk {
            request_id: request_id.into(),
            asker: listed.as_ref().and_then(|r| r.asker.clone()),
            asked_at: listed.as_ref().and_then(|r| r.asked_at),
            expires_at: listed.as_ref().map(|r| r.expires_at),
            server: consent::host(&self.forwarding_server()),
        }
    }

    async fn pair_app_with(&self, flow: &str, code: &str) -> Result<Option<PairApp>, CoreError> {
        let answer = self
            .device_call(flow, Kind::Query, "pair:info", json!({ "code": code }))
            .await
            .map_err(refused)?;
        let app = match serde_json::from_value(answer).map_err(unexpected)? {
            PairInfo::Mcp { name, host } => Some(PairApp { name, host }),
            PairInfo::Browser | PairInfo::Other => None,
        };
        info!(mcp = app.is_some(), "pair code checked");
        Ok(app)
    }

    /// Adds this phone to the account's phones (`auth:token` links it), with the semester
    /// picked here.
    async fn link_with(&self, flow: &str, action: &str) -> Result<(), CoreError> {
        // A new identity sends the picked semester too (`semester_to_server`); a later pick goes
        // with `send_server_semester`.
        self.identity(flow, action, true).await?;
        info!("phone linked");
        Ok(())
    }

    async fn create_key_with(
        &self,
        flow: &str,
        action: &str,
        outings: bool,
        pair_code: Option<String>,
    ) -> Result<NewKey, CoreError> {
        let paired = pair_code.is_some();
        let mut body = json!({
            "label": if paired { BROWSER_LABEL } else { MCP_LABEL },
            "outings": outings,
        });
        if let Some(code) = pair_code {
            crate::json_set(&mut body, "pairCode", json!(code));
        }
        // The JWT links this phone to the account, so the key is always this phone's.
        let answer = self
            .account_call(flow, Kind::Action, "keys:create", action, true, body)
            .await?
            .map_err(refused)?;
        let key: NewKey = serde_json::from_value(answer).map_err(unexpected)?;
        info!(paired, outings, "key made");
        Ok(key)
    }

    /// Sends `session` (cookies, or why there are none) as the answer to `request_id`.
    pub(crate) async fn answer_with(
        &self,
        flow: &str,
        request_id: &str,
        session: Result<String, CoreError>,
    ) -> Result<Delivered, CoreError> {
        let (body, failed) = match session {
            Ok(cookies) => (
                json!({ "requestId": request_id, "cookies": cookie_list(&cookies) }),
                None,
            ),
            Err(error) => (
                json!({ "requestId": request_id, "error": why_not(&error) }),
                Some(error),
            ),
        };
        let path = "sessions:answer";
        // With a session to send, an install with no identity yet (updated from v2 without a
        // live account token) proves first: the student already said yes, and the proof needs
        // the live session the answer carries anyway.
        let answer = if failed.is_none() {
            self.account_call(
                flow,
                Kind::Action,
                path,
                "Answer a connected app",
                true,
                body,
            )
            .await?
        } else {
            self.device_call(flow, Kind::Action, path, body).await
        };
        let late = answer.as_ref().ok().and_then(|a| a["late"].as_bool());
        info!(sent = failed.is_none(), late, "session request answered");
        match (failed, answer) {
            (Some(error), _) => Err(error),
            (None, answer) => answer.map_err(refused).map(|_| {
                if late == Some(true) {
                    Delivered::Late
                } else {
                    Delivered::Taken
                }
            }),
        }
    }
}

/// Whether the server took a session answer in time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Delivered {
    Taken,
    /// The request had ended (answered, timed out or expired) before it arrived.
    Late,
}

/// Why a session request got no session, for the server and the app's notification.
fn why_not(error: &CoreError) -> String {
    match error {
        CoreError::NotSignedIn => "The phone is signed out of VTOP.".to_string(),
        CoreError::Offline => "The phone is offline.".to_string(),
        CoreError::OtpRequired { .. } => "The phone needs an OTP to sign in.".into(),
        CoreError::InvalidCredentials => "VTOP rejected the phone's password.".into(),
        CoreError::ConsentDenied | CoreError::RegnoNotShared => {
            "The student hasn't allowed it on the phone.".into()
        }
        error => format!("The phone couldn't get a session: {error}"),
    }
}

#[cfg(test)]
mod tests;
