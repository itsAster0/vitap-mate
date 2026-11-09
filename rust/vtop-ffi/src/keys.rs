//! Connected apps (Settings → Account): the access keys an MCP client or the browser extension
//! use, and this phone's answers to their session requests. Every call is signed as this install
//! ([`Core::edge_post`]) and sends the live VTOP session, so the edge can tell whose account it is
//! (`rust/vtop-edge/API.md`, "Accounts — phones"). Keys are made on demand: the first key links
//! this phone to the account. A new key is shown once and never logged; nor are cookies or
//! pairing codes.

use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{info, warn};

use crate::device::EdgeError;
use crate::{Core, CoreError, dev, traced};

/// The label of a key made by pairing the browser extension; [`KeyKind`] is read from it.
pub const BROWSER_LABEL: &str = "Browser";
/// The label of a key made for an MCP client.
const MCP_LABEL: &str = "MCP";
/// Crockford base32, the alphabet the extension draws pairing codes from (`vtop-edge/src/pair.ts`).
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

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ConnectedApps {
    /// This phone is one of the account's phones (it made a key, or linked before).
    pub linked: bool,
    pub keys: Vec<AccessKey>,
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
            let cookies = self.live_cookies().await?;
            let body = json!({ "session": { "cookies": cookies } });
            let view = self
                .edge_post(&self.forwarding_server(), "/v1/account", &flow, &body)
                .await
                .map_err(refused)?;
            let keys: Vec<AccessKey> = serde_json::from_value::<Vec<KeyView>>(view["keys"].clone())
                .unwrap_or_default()
                .into_iter()
                .map(AccessKey::from)
                .collect();
            let linked = view["thisPhone"].as_bool().unwrap_or(false);
            info!(linked, keys = keys.len(), "connected apps");
            Ok(ConnectedApps { linked, keys })
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
            let cookies = self.live_cookies().await?;
            self.create_key_with(
                &self.forwarding_server(),
                &flow,
                &cookies,
                outings,
                pair_code,
            )
            .await
        })
        .await
    }

    pub async fn revoke_key(&self, flow: String, id: String) -> Result<(), CoreError> {
        traced("revoke_key", &flow, async {
            let cookies = self.live_cookies().await?;
            let body = json!({ "session": { "cookies": cookies }, "id": id });
            self.edge_post(&self.forwarding_server(), "/v1/keys/revoke", &flow, &body)
                .await
                .map_err(refused)?;
            info!("key revoked");
            Ok(())
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
            let cookies = self.live_cookies().await?;
            let body = json!({ "session": { "cookies": cookies }, "id": id, "outings": outings });
            self.edge_post(&self.forwarding_server(), "/v1/keys/outings", &flow, &body)
                .await
                .map_err(refused)?;
            info!(outings, "key outings");
            Ok(())
        })
        .await
    }

    /// Answers a `vtop_cookie_request` (push or event log): this phone's live VTOP session,
    /// signing in again when it had expired, or why it can't. With `error` (the caller gave up,
    /// e.g. timed out) only that is sent. A failure here is sent too, then returned.
    pub async fn answer_session_request(
        &self,
        flow: String,
        request_id: String,
        callback_url: String,
        error: Option<String>,
    ) -> Result<(), CoreError> {
        traced("answer_session_request", &flow, async {
            let server = self.forwarding_server();
            let session = match error {
                Some(error) => Err(CoreError::Other { reason: error }),
                None => self.live_cookies().await,
            };
            self.answer_with(&server, &flow, &request_id, &callback_url, session)
                .await
        })
        .await
    }
}

impl Core {
    /// The live VTOP session's cookie header (checked with VTOP, signed in again if needed).
    pub(crate) async fn live_cookies(&self) -> Result<String, CoreError> {
        self.session_cookies().await?.ok_or(CoreError::NotSignedIn)
    }

    async fn create_key_with(
        &self,
        server: &str,
        flow: &str,
        cookies: &str,
        outings: bool,
        pair_code: Option<String>,
    ) -> Result<NewKey, CoreError> {
        let session = json!({ "cookies": cookies });
        let paired = pair_code.is_some();
        let mut body = json!({
            "session": session,
            "label": if paired { BROWSER_LABEL } else { MCP_LABEL },
            "outings": outings,
        });
        if let Some(code) = pair_code {
            body["pairCode"] = json!(code);
        }
        let mut linked_now = false;
        let answer = loop {
            match self.edge_post(server, "/v1/keys", flow, &body).await {
                Err(error) if error.code() == "not_linked" && !linked_now => {
                    let mut link = json!({ "session": session, "label": crate::server::label() });
                    if let Some(semester) = self.read::<String>(crate::SELECTED) {
                        link["semesterId"] = json!(semester);
                    }
                    self.edge_post(server, "/v1/link", flow, &link)
                        .await
                        .map_err(refused)?;
                    info!("phone linked");
                    linked_now = true;
                }
                answer => break answer.map_err(refused)?,
            }
        };
        let text = |key: &str| {
            answer[key]
                .as_str()
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
        let path = callback_url
            .strip_prefix(server.trim_end_matches('/'))
            .filter(|path| path.starts_with(ANSWER_PATH))
            .ok_or_else(|| {
                warn!("session request for another server");
                CoreError::Other {
                    reason: "The session request names another server.".into(),
                }
            })?;
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::forwarding::tests::{Edge, serve};

    fn core(name: &str) -> Core {
        let dir = std::env::temp_dir().join(format!("vtop-ffi-keys-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Core {
            cache: dir.clone(),
            dir,
            slot: Mutex::new(None),
        }
    }

    fn can(edge: &Arc<Mutex<Edge>>, path: &str, status: u16, body: Value) {
        let mut edge = edge.lock().unwrap();
        edge.canned
            .entry(path.into())
            .or_default()
            .push_back((status, body));
    }

    #[test]
    fn pair_codes_read_like_the_edge() {
        let code = |text: &str| pair_code(text.into());
        assert_eq!(code("abcde-12345").as_deref(), Some("ABCDE12345"));
        assert_eq!(code(" ABCDE 12345 ").as_deref(), Some("ABCDE12345"));
        assert_eq!(code("OILAB-CDEFG").as_deref(), Some("011ABCDEFG"));
        assert_eq!(
            code("https://vmate.aster0.dev/pair#ABCDE-12345").as_deref(),
            Some("ABCDE12345")
        );
        // U is not Crockford; too short; too long.
        assert_eq!(code("ABCDU-12345"), None);
        assert_eq!(code("ABCD-1234"), None);
        assert_eq!(code("ABCDE-123456"), None);
        assert_eq!(code("https://vmate.aster0.dev/pair"), None);
    }

    #[test]
    fn cookies_become_the_answer_list() {
        let list = cookie_list("JSESSIONID=abc; SERVERID=s1;  ; broken");
        assert_eq!(
            list,
            [
                json!({ "name": "JSESSIONID", "value": "abc", "domain": "vtop.vitap.ac.in" }),
                json!({ "name": "SERVERID", "value": "s1", "domain": "vtop.vitap.ac.in" }),
            ]
        );
    }

    #[test]
    fn keys_read_from_the_account_view() {
        let views: Vec<KeyView> = serde_json::from_value(json!([
            { "id": "ab12cd34", "label": "Browser", "createdAt": 10, "lastUsedAt": 20, "phoneId": "p", "outings": false,
              "uses": [{ "at": 20, "via": "extension" }] },
            { "id": "ef56ab78", "label": "MCP", "createdAt": 11, "lastUsedAt": null, "phoneId": "p", "outings": true },
        ]))
        .unwrap();
        let keys: Vec<AccessKey> = views.into_iter().map(AccessKey::from).collect();
        assert_eq!(keys[0].kind, KeyKind::Browser);
        assert_eq!(
            keys[0].uses,
            [KeyUse {
                at: 20,
                via: KeyKind::Browser
            }]
        );
        assert_eq!(
            (keys[1].kind, keys[1].outings, keys[1].last_used_at),
            (KeyKind::Mcp, true, None)
        );
        assert!(keys[1].uses.is_empty());
    }

    #[tokio::test]
    async fn a_first_key_links_the_phone_then_is_made() {
        let core = core("create");
        let edge = Arc::new(Mutex::new(Edge::default()));
        let server = serve(edge.clone()).await;
        let not_linked = json!({ "code": "not_linked", "error": "Link first." });
        can(&edge, "/v1/keys", 409, not_linked);
        let made = json!({ "id": "ab12cd34", "key": "vtm_secret", "mcpUrl": "https://vmate.aster0.dev/mcp" });
        can(&edge, "/v1/keys", 201, made);
        can(
            &edge,
            "/v1/link",
            200,
            json!({ "savedCredentials": false, "forwarding": false }),
        );
        let key = core
            .create_key_with(
                &server,
                "f-key",
                "JSESSIONID=c",
                true,
                Some("ABCDE12345".into()),
            )
            .await
            .unwrap();
        assert_eq!(
            (key.id.as_str(), key.key.as_str()),
            ("ab12cd34", "vtm_secret")
        );
        let edge = edge.lock().unwrap();
        let paths: Vec<&str> = edge.requests.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(
            paths,
            ["/v1/devices/register", "/v1/keys", "/v1/link", "/v1/keys"]
        );
        assert!(edge.requests.iter().all(|r| r.signed && r.flow == "f-key"));
        assert_eq!(
            edge.requests[1].body,
            json!({ "session": { "cookies": "JSESSIONID=c" }, "label": "Browser", "outings": true, "pairCode": "ABCDE12345" })
        );
        assert_eq!(
            edge.requests[2].body,
            json!({ "session": { "cookies": "JSESSIONID=c" }, "label": "vitapmate" })
        );
    }

    #[tokio::test]
    async fn refusals_read_as_what_to_do() {
        let core = core("refused");
        let edge = Arc::new(Mutex::new(Edge::default()));
        let server = serve(edge.clone()).await;
        can(
            &edge,
            "/v1/keys",
            409,
            json!({ "code": "key_limit", "error": "x" }),
        );
        can(
            &edge,
            "/v1/keys",
            400,
            json!({ "code": "bad_request", "error": "x" }),
        );
        let reason = |result: Result<NewKey, CoreError>| match result {
            Err(CoreError::Other { reason }) => reason,
            other => panic!("{other:?}"),
        };
        let full = core.create_key_with(&server, "f", "c=1", false, None).await;
        assert!(reason(full).starts_with("Revoke a key first"));
        let bad = core
            .create_key_with(&server, "f", "c=1", false, Some("ABCDE12345".into()))
            .await;
        assert!(reason(bad).starts_with("Check the code"));
        assert_eq!(edge.lock().unwrap().requests[1].body["label"], "MCP");
    }

    #[tokio::test]
    async fn session_requests_get_cookies_or_why_not() {
        let core = core("answer");
        let edge = Arc::new(Mutex::new(Edge::default()));
        let server = serve(edge.clone()).await;
        let path = "/v1/session/answer?account=22bce0001";
        let callback = format!("{server}{path}");
        can(&edge, path, 200, json!({ "ok": true }));
        core.answer_with(&server, "f-a", "r1", &callback, Ok("JSESSIONID=c".into()))
            .await
            .unwrap();
        // Signed out (no session in this core): the edge hears why, the caller gets the error.
        can(&edge, path, 200, json!({ "ok": true }));
        fs::write(core.at(crate::forwarding::SERVER_FILE), &server).unwrap();
        let signed_out = core
            .answer_session_request("f-b".into(), "r2".into(), callback.clone(), None)
            .await;
        assert_eq!(signed_out, Err(CoreError::NotSignedIn));
        // Another server's callback is not sent anything.
        let elsewhere = core
            .answer_with(
                &server,
                "f-c",
                "r3",
                "https://evil.example/v1/session/answer?account=x",
                Ok("c=1".into()),
            )
            .await;
        assert!(matches!(elsewhere, Err(CoreError::Other { .. })));

        let edge = edge.lock().unwrap();
        let answers: Vec<&Value> = edge
            .requests
            .iter()
            .filter(|r| r.path == path)
            .map(|r| &r.body)
            .collect();
        assert_eq!(
            answers,
            [
                &json!({ "requestId": "r1", "cookies": [{ "name": "JSESSIONID", "value": "c", "domain": "vtop.vitap.ac.in" }] }),
                &json!({ "requestId": "r2", "error": "The phone is signed out of VTOP." }),
            ]
        );
        assert!(edge.requests.iter().all(|r| r.signed));
    }
}
