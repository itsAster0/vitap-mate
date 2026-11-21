//! The edge's account token (`rust/vtop-edge/API.md`, "Account tokens"): whose this phone is,
//! issued after the edge checked the VTOP session once. Account calls send it in place of the
//! session, so they need no VTOP check and ask no cookie consent. It is sent at any age: while it
//! is this install's live one the edge renews it in the answer near (or past) its 14-day end. The
//! session goes once more only when there is no token or the edge turns it down. Kept in [`FILE`] for
//! the signed-in student on one server; dropped on sign-out, another student and when the
//! server's account or this install is deleted. The token itself is never logged.

use std::fs;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::info;

use crate::consent::CookieKind;
use crate::device::EdgeError;
use crate::{Core, CoreError};

pub(crate) const FILE: &str = "account_token.json";

/// No `Debug`: it holds the token.
#[derive(Serialize, Deserialize)]
struct Saved {
    server: String,
    registration_number: String,
    token: String,
    /// Unix ms.
    expires_at: i64,
}

impl Core {
    /// POSTs `body` to the account route `path` on `server` as the signed-in student: with the
    /// account token kept for them there (at any age), else with the live VTOP session
    /// once allowed for `action` ([`Core::session_cookies`], asking as `ask` says). A token the
    /// edge turns down is dropped and the call made once more with the session; a token in the
    /// answer is kept. The outer error is the session's (not signed in, consent denied), the
    /// inner one the edge's.
    pub(crate) async fn account_post(
        &self,
        server: &str,
        path: &str,
        flow: &str,
        action: &str,
        ask: bool,
        mut body: Value,
    ) -> Result<Result<Value, EdgeError>, CoreError> {
        let regno = match self.current() {
            Ok(slot) => slot.read().await.client.registration_number().ok(),
            Err(_) => None,
        };
        let saved = self
            .read::<Saved>(FILE)
            .filter(|saved| Some(&saved.registration_number) == regno.as_ref())
            .filter(|saved| saved.server == server);
        if let Some(saved) = saved {
            let mut sent = body.clone();
            crate::json_set(&mut sent, "accountToken", json!(saved.token));
            match self.edge_answer(server, path, flow, &sent).await {
                Err(error) if matches!(error.code(), "token_invalid" | "token_expired") => {
                    info!(path, code = error.code(), "account token refused");
                    self.forget_account_token();
                }
                answer => {
                    info!(path, "account token used");
                    self.keep_answer_token(server, path, regno, &answer);
                    return Ok(answer.map(|answer| answer.body));
                }
            }
        }
        let cookies = self
            .session_cookies(flow, CookieKind::Account, action, ask)
            .await?
            .ok_or(CoreError::NotSignedIn)?;
        crate::json_set(&mut body, "session", json!({ "cookies": cookies }));
        let answer = self.edge_answer(server, path, flow, &body).await;
        self.keep_answer_token(server, path, regno, &answer);
        Ok(answer.map(|answer| answer.body))
    }

    /// Keeps the token an answer carries (issued after the session, or renewed).
    fn keep_answer_token(
        &self,
        server: &str,
        path: &str,
        regno: Option<String>,
        answer: &Result<crate::device::Answer, EdgeError>,
    ) {
        if let (Ok(answer), Some(regno)) = (answer, regno)
            && let Some((token, expires_at)) = answer.account_token.clone()
        {
            let saved = Saved {
                server: server.to_string(),
                registration_number: regno,
                token,
                expires_at,
            };
            self.write_plain(FILE, &saved);
            info!(path, expires_at, "account token issued");
        }
    }

    pub(crate) fn forget_account_token(&self) {
        match fs::remove_file(self.at(FILE)) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                tracing::warn!(%error, "account token not removed");
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::consent;
    use crate::forwarding::tests::{Edge, serve, signed_in};
    use crate::unix_now;

    async fn setup(name: &str) -> (Core, Arc<Mutex<Edge>>, String) {
        let dir =
            std::env::temp_dir().join(format!("vtop-ffi-token-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let edge = Arc::new(Mutex::new(Edge::default()));
        let server = serve(edge.clone()).await;
        fs::write(dir.join(crate::forwarding::SERVER_FILE), &server).unwrap();
        (signed_in(&dir, server.clone()), edge, server)
    }

    fn keep(core: &Core, server: &str, regno: &str, token: &str, expires_at: i64) {
        let saved = Saved {
            server: server.into(),
            registration_number: regno.into(),
            token: token.into(),
            expires_at,
        };
        core.write_plain(FILE, &saved);
    }

    fn can(edge: &Arc<Mutex<Edge>>, status: u16, body: Value) {
        let mut edge = edge.lock().unwrap();
        let canned = edge.canned.entry("/v1/account".into()).or_default();
        canned.push_back((status, body));
    }

    fn sent(edge: &Arc<Mutex<Edge>>) -> Vec<Value> {
        let edge = edge.lock().unwrap();
        let calls = edge.requests.iter().filter(|r| r.path == "/v1/account");
        calls.map(|r| r.body.clone()).collect()
    }

    const DAY_MS: i64 = 86_400_000;
    const VIEW: &str = r#"{ "thisPhone": true, "keys": [] }"#;

    #[tokio::test]
    async fn a_kept_token_goes_instead_of_the_session_and_asks_nothing() {
        let (core, edge, server) = setup("used").await;
        keep(
            &core,
            &server,
            "22BCE0001",
            "tok",
            unix_now() * 1000 + DAY_MS,
        );
        can(&edge, 200, serde_json::from_str(VIEW).unwrap());
        let gate = consent::tests::no();
        let apps = core.connected_apps("f".into()).await.unwrap();
        assert!(apps.linked);
        assert_eq!(gate.asks(), 0);
        assert_eq!(sent(&edge), [json!({ "accountToken": "tok" })]);
    }

    #[tokio::test]
    async fn a_refused_token_is_dropped_and_the_session_sent_once() {
        let (core, edge, server) = setup("refused").await;
        keep(
            &core,
            &server,
            "22BCE0001",
            "tok",
            unix_now() * 1000 + DAY_MS,
        );
        let expired = json!({ "code": "token_expired", "error": "x" });
        can(&edge, 401, expired);
        can(&edge, 200, serde_json::from_str(VIEW).unwrap());
        edge.lock().unwrap().issue = Some("tok2".into());
        let gate = consent::tests::yes();
        core.connected_apps("f".into()).await.unwrap();
        assert_eq!(gate.asks(), 1);
        let bodies = sent(&edge);
        assert_eq!(bodies[0], json!({ "accountToken": "tok" }));
        assert!(bodies[1]["session"]["cookies"].is_string());
        assert!(bodies[1].get("accountToken").is_none());
        // The token that came back is kept, and goes next time.
        let kept = core.read::<Saved>(FILE).unwrap();
        assert_eq!((kept.token.as_str(), kept.server == server), ("tok2", true));
        assert!(kept.expires_at > unix_now() * 1000 + 13 * DAY_MS);
        can(&edge, 200, serde_json::from_str(VIEW).unwrap());
        core.connected_apps("f2".into()).await.unwrap();
        assert_eq!(sent(&edge)[2], json!({ "accountToken": "tok2" }));
        assert_eq!(gate.asks(), 1);
    }

    #[tokio::test]
    async fn an_expired_token_is_sent_for_renewal_but_not_another_students() {
        let (core, edge, server) = setup("stale").await;
        let _gate = consent::tests::yes();
        // Past its end: the edge renews it while it is this install's live one.
        keep(
            &core,
            &server,
            "22BCE0001",
            "old",
            unix_now() * 1000 - DAY_MS,
        );
        can(&edge, 200, serde_json::from_str(VIEW).unwrap());
        core.connected_apps("f".into()).await.unwrap();
        assert_eq!(sent(&edge)[0]["accountToken"], "old");
        assert!(sent(&edge)[0].get("session").is_none());
        keep(
            &core,
            &server,
            "22BCE9999",
            "tok",
            unix_now() * 1000 + DAY_MS,
        );
        can(&edge, 200, serde_json::from_str(VIEW).unwrap());
        core.connected_apps("f2".into()).await.unwrap();
        assert!(sent(&edge)[1].get("accountToken").is_none());
    }

    #[tokio::test]
    async fn sign_out_drops_the_token() {
        let (core, _edge, server) = setup("sign-out").await;
        keep(
            &core,
            &server,
            "22BCE0001",
            "tok",
            unix_now() * 1000 + DAY_MS,
        );
        core.sign_out("f".into());
        assert!(!core.at(FILE).exists());
    }
}
