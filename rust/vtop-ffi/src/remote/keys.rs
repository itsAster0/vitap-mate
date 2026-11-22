//! Connected apps (Settings → Account): the access keys an MCP client or the browser extension
//! use, and this phone's answers to their session requests. Every call is signed as this install
//! ([`Core::edge_post`]) and sends the live VTOP session, so the edge can tell whose account it is
//! (`edge/API.md`, "Accounts — phones"). Keys are made on demand: the first key links
//! this phone to the account. A new key is shown once and never logged; nor are cookies or
//! pairing codes.

use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{info, warn};

use crate::device::EdgeError;
use crate::remote::consent::CookieKind;
use crate::{Core, CoreError, dev, traced};

/// The label of a key made by pairing the browser extension; [`KeyKind`] is read from it.
pub const BROWSER_LABEL: &str = "Browser";
/// The label of a key made for an MCP client.
const MCP_LABEL: &str = "MCP";
/// Crockford base32, the alphabet the extension draws pairing codes from (`edge/src/pair.ts`).
const PAIR_ALPHABET: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const PAIR_LENGTH: usize = 10;
/// The path every session request's `callbackUrl` points at.
const ANSWER_PATH: &str = "/v1/session/answer?";

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum KeyKind {
    Mcp,
    Browser,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct KeyUse {
    /// Unix seconds.
    pub at: i64,
    pub via: KeyKind,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AccessKey {
    /// The short id the edge revokes it by.
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

/// The edge's refusals in the student's words.
pub(crate) fn refused(error: EdgeError) -> CoreError {
    let reason = match error.code() {
        "key_limit" => "Revoke a key first: this account has the most keys allowed.",
        "pair_expired" => "That code has expired. Start connecting again in the app.",
        "bad_request" => "Check the code: it's the 10 characters the extension shows.",
        "key_not_found" => "That key is already gone.",
        "phone_not_found" => "That phone is already gone.",
        "session_invalid" => "VTOP didn't accept this phone's session. Try again.",
        "vtop_unreachable" => "The server couldn't reach VTOP. Try again in a minute.",
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
                .account_post(
                    &self.forwarding_server(),
                    "/v1/account",
                    &flow,
                    action,
                    true,
                    json!({}),
                )
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
            self.link_with(&self.forwarding_server(), &flow, action)
                .await
        })
        .await
    }

    /// Makes a key: for the extension that shows `pair_code` (it collects the key from the edge),
    /// else for an MCP client. Links this phone first when it is not yet.
    pub async fn create_key(
        &self,
        flow: String,
        outings: bool,
        pair_code: Option<String>,
    ) -> Result<NewKey, CoreError> {
        traced("create_key", &flow, async {
            self.create_key_with(
                &self.forwarding_server(),
                &flow,
                "Make an access key",
                outings,
                pair_code,
            )
            .await
        })
        .await
    }

    /// The MCP client behind a pairing code from its sign-in page; None for the extension's codes.
    pub async fn pair_app(&self, flow: String, code: String) -> Result<Option<PairApp>, CoreError> {
        traced("pair_app", &flow, async {
            self.pair_app_with(&self.forwarding_server(), &flow, &code)
                .await
        })
        .await
    }

    pub async fn revoke_key(&self, flow: String, id: String) -> Result<(), CoreError> {
        traced("revoke_key", &flow, async {
            let (path, action) = ("/v1/keys/revoke", "Revoke an access key");
            let body = json!({ "id": id });
            self.account_post(&self.forwarding_server(), path, &flow, action, true, body)
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
            let (path, action) = ("/v1/keys/move", "Move keys to this phone");
            let mut body = json!({});
            if let Some(from) = from {
                crate::json_set(&mut body, "from", json!(from));
            }
            let answer = self
                .account_post(&self.forwarding_server(), path, &flow, action, true, body)
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
            let (path, action) = ("/v1/keys/outings", "Change a key's outing access");
            let body = json!({ "id": id, "outings": outings });
            self.account_post(&self.forwarding_server(), path, &flow, action, true, body)
                .await?
                .map_err(refused)?;
            info!(outings, "key outings");
            Ok(())
        })
        .await
    }

    /// Answers a `vtop_cookie_request` (push or event log): this phone's live VTOP session,
    /// signing in again when it had expired, or why it can't. With `error` (the caller gave up,
    /// e.g. timed out) only that is sent. A failure here is sent too, then returned. Only with
    /// the student on screen saying yes: from the background it is always the error.
    pub async fn answer_session_request(
        &self,
        flow: String,
        request_id: String,
        callback_url: String,
        error: Option<String>,
    ) -> Result<(), CoreError> {
        traced("answer_session_request", &flow, async {
            let server = self.forwarding_server();
            answer_path(&server, &callback_url)?; // Before the dialog: never ask for elsewhere.
            let session = match error {
                Some(error) => Err(CoreError::Other { reason: error }),
                None => {
                    let action = "Give a connected app your VTOP session";
                    self.session_cookies(&flow, CookieKind::Handover, action, true)
                        .await
                        .and_then(|cookies| cookies.ok_or(CoreError::NotSignedIn))
                }
            };
            self.answer_with(&server, &flow, &request_id, &callback_url, session)
                .await
        })
        .await
    }
}

impl Core {
    async fn pair_app_with(
        &self,
        server: &str,
        flow: &str,
        code: &str,
    ) -> Result<Option<PairApp>, CoreError> {
        let answer = self
            .edge_post(server, "/v1/pair/info", flow, &json!({ "code": code }))
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

    /// Adds this phone to the account's phones, with the semester picked here.
    async fn link_with(&self, server: &str, flow: &str, action: &str) -> Result<(), CoreError> {
        let mut link = json!({ "label": crate::remote::server::label() });
        if let Some(semester) = self.read_kept::<String>(crate::SELECTED) {
            crate::json_set(&mut link, "semesterId", json!(semester));
        }
        self.account_post(server, "/v1/link", flow, action, true, link)
            .await?
            .map_err(refused)?;
        info!("phone linked");
        Ok(())
    }

    async fn create_key_with(
        &self,
        server: &str,
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
        let mut linked_now = false;
        let answer = loop {
            let made = self.account_post(server, "/v1/keys", flow, action, true, body.clone());
            match made.await? {
                Err(error) if error.code() == "not_linked" && !linked_now => {
                    self.link_with(server, flow, action).await?;
                    linked_now = true;
                }
                answer => break answer.map_err(refused)?,
            }
        };
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
        info!(paired, outings, linked_now, "key made");
        Ok(key)
    }

    /// Sends `session` (cookies, or why there are none) to the request's callback on `server`;
    /// a callback anywhere else is refused unsent.
    async fn answer_with(
        &self,
        server: &str,
        flow: &str,
        request_id: &str,
        callback_url: &str,
        session: Result<String, CoreError>,
    ) -> Result<(), CoreError> {
        let path = answer_path(server, callback_url)?;
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
        let answer = self.edge_post(server, path, flow, &body).await;
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

/// The answer path of `callback_url` on `server`; a callback anywhere else is refused.
fn answer_path<'a>(server: &str, callback_url: &'a str) -> Result<&'a str, CoreError> {
    callback_url
        .strip_prefix(server.trim_end_matches('/'))
        .filter(|path| path.starts_with(ANSWER_PATH))
        .ok_or_else(|| {
            warn!("session request for another server");
            CoreError::Other {
                reason: "The session request names another server.".into(),
            }
        })
}

#[cfg(test)]
mod tests;
