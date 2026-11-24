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
use crate::remote::consent::CookieKind;
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
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct SessionRequest {
    pub request_id: String,
    /// Unix ms.
    pub expires_at: i64,
    /// Whether the app also asks for the saved password.
    pub want_credentials: bool,
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
    /// The phone's short id; None for phones no longer linked (nothing answers for their keys).
    pub id: Option<String>,
    pub label: Option<String>,
    /// Unix seconds; None for phones no longer linked.
    pub seen_at: Option<i64>,
    pub keys: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConnectedApps {
    /// This phone is one of the account's phones (it made a key, or linked before).
    pub linked: bool,
    pub keys: Vec<AccessKey>,
    /// The other phones with keys, most keys first; the unlinked ones as one entry.
    pub elsewhere: Vec<KeyPhone>,
}

/// The other phones that answer for keys, from the account view's `phones` and `keys`.
fn elsewhere(view: &Value) -> Vec<KeyPhone> {
    let list = |key: &str| view[key].as_array().cloned().unwrap_or_default();
    let phones = list("phones");
    let count = |phone: Option<&str>| {
        list("keys")
            .iter()
            .filter(|key| match phone {
                Some(id) => key["phoneId"] == id,
                None => !phones.iter().any(|p| p["id"] == key["phoneId"]),
            })
            .count() as u32
    };
    let mut found: Vec<KeyPhone> = phones
        .iter()
        .filter(|phone| phone["thisPhone"] != true)
        .filter_map(|phone| {
            let id = phone["id"].as_str()?;
            Some(KeyPhone {
                id: Some(id.into()),
                label: phone["label"].as_str().map(str::to_string),
                seen_at: phone["seenAt"].as_i64(),
                keys: count(Some(id)),
            })
        })
        .collect();
    found.push(KeyPhone {
        id: None,
        label: None,
        seen_at: None,
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

/// A key just made: [`NewKey::key`] is shown once.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
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
    /// The account's access keys and whether this phone is linked to it.
    pub async fn connected_apps(&self, flow: String) -> Result<ConnectedApps, CoreError> {
        traced("connected_apps", &flow, async {
            let action = "Load your connected apps";
            let view = self
                .account_call(&flow, Kind::Query, "account:view", action, true, json!({}))
                .await?
                .map_err(refused)?;
            let keys: Vec<AccessKey> = serde_json::from_value::<Vec<KeyView>>(
                view.get("keys").cloned().unwrap_or_default(),
            )
            .unwrap_or_default()
            .into_iter()
            .map(AccessKey::from)
            .collect();
            let linked = view
                .get("thisPhone")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let elsewhere = if linked { elsewhere(&view) } else { vec![] };
            info!(
                linked,
                keys = keys.len(),
                elsewhere = elsewhere.len(),
                "connected apps"
            );
            Ok(ConnectedApps {
                linked,
                keys,
                elsewhere,
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
            let moved = answer.get("moved").and_then(Value::as_u64).unwrap_or(0) as u32;
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
                answer => serde_json::from_value(answer.map_err(refused)?).map_err(|_| {
                    CoreError::Other {
                        reason: "The server sent an unexpected answer.".into(),
                    }
                })?,
            };
            info!(count = requests.len(), "session requests");
            Ok(requests)
        })
        .await
    }

    /// Answers a `vtop_cookie_request` (push or `sessions:pending`): this phone's live VTOP session,
    /// signing in again when it had expired, or why it can't. With `error` (the caller gave up,
    /// e.g. timed out) only that is sent. A failure here is sent too, then returned. Only with
    /// the student on screen saying yes: from the background it is always the error.
    pub async fn answer_session_request(
        &self,
        flow: String,
        request_id: String,
        error: Option<String>,
    ) -> Result<(), CoreError> {
        traced("answer_session_request", &flow, async {
            let session = match error {
                Some(error) => Err(CoreError::Other { reason: error }),
                None => {
                    let action = "Give a connected app your VTOP session";
                    self.session_cookies(&flow, CookieKind::Handover, action, true)
                        .await
                        .and_then(|cookies| cookies.ok_or(CoreError::NotSignedIn))
                }
            };
            self.answer_with(&flow, &request_id, session).await
        })
        .await
    }
}

impl Core {
    async fn pair_app_with(&self, flow: &str, code: &str) -> Result<Option<PairApp>, CoreError> {
        let answer = self
            .device_call(flow, Kind::Query, "pair:info", json!({ "code": code }))
            .await
            .map_err(refused)?;
        let text = |key: &str| answer.get(key)?.as_str().map(str::to_string);
        let app = match (
            answer.get("kind").and_then(Value::as_str),
            text("name"),
            text("host"),
        ) {
            (Some("mcp"), Some(name), Some(host)) => Some(PairApp { name, host }),
            _ => None,
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
        let text = |key: &str| {
            answer
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| CoreError::Other {
                    reason: "The server sent an unexpected answer.".into(),
                })
        };
        let key = NewKey {
            id: text("id")?,
            key: text("key")?,
            mcp_url: text("mcpUrl")?,
        };
        info!(paired, outings, "key made");
        Ok(key)
    }

    /// Sends `session` (cookies, or why there are none) as the answer to `request_id`.
    async fn answer_with(
        &self,
        flow: &str,
        request_id: &str,
        session: Result<String, CoreError>,
    ) -> Result<(), CoreError> {
        let (body, failed) = match session {
            Ok(cookies) => (
                json!({ "requestId": request_id, "cookies": cookie_list(&cookies) }),
                None,
            ),
            Err(error) => {
                let reason = match &error {
                    CoreError::NotSignedIn => "The phone is signed out of VTOP.".to_string(),
                    CoreError::Offline => "The phone is offline.".to_string(),
                    CoreError::OtpRequired { .. } => "The phone needs an OTP to sign in.".into(),
                    CoreError::InvalidCredentials => "VTOP rejected the phone's password.".into(),
                    CoreError::ConsentDenied => {
                        "The student hasn't allowed it on the phone.".into()
                    }
                    error => format!("The phone couldn't get a session: {error}"),
                };
                (
                    json!({ "requestId": request_id, "error": reason }),
                    Some(error),
                )
            }
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
        info!(
            sent = failed.is_none(),
            late = answer.as_ref().ok().and_then(|a| a["late"].as_bool()),
            "session request answered"
        );
        match (failed, answer) {
            (Some(error), _) => Err(error),
            (None, answer) => answer.map(|_| ()).map_err(refused),
        }
    }
}

#[cfg(test)]
mod tests;
