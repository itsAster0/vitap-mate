//! The edge's account token (`edge/API.md`, "Account tokens"): whose this phone is, issued
//! after the phone proved it ([`crate::remote::account_proof`]), so the VTOP session never goes.
//! Account calls send it, so they need no VTOP check and ask no consent. It is sent at any age:
//! while it is this install's live one the edge renews it in the answer near (or past) its
//! 14-day end. Without a token, or when the edge turns it down, the phone proves once more. Kept
//! in [`FILE`] for the signed-in student on one server; dropped on sign-out, another student and
//! when the server's account or this install is deleted. The token itself is never logged.

use std::fs;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::info;

use crate::device::EdgeError;
use crate::{Core, CoreError};

pub(crate) const FILE: &str = "account_token.json";

/// No `Debug`: it holds the token.
#[derive(Serialize, Deserialize)]
pub(crate) struct Saved {
    pub(crate) server: String,
    pub(crate) registration_number: String,
    pub(crate) token: String,
    /// Unix ms.
    pub(crate) expires_at: i64,
}

impl Core {
    /// POSTs `body` to the account route `path` on `server` as the signed-in student, with the
    /// account token kept for them there (at any age). Without one, or when the edge turns it
    /// down, it proves who the student is for a new one first, as `action` (for the logs); a
    /// call nobody started (`ask` false) never starts a proof and is `ConsentDenied`, which its
    /// callers take as "not now". A token in the answer is kept. The outer error is the phone's
    /// (not signed in, the proof), the inner one the edge's.
    pub(crate) async fn account_post(
        &self,
        server: &str,
        path: &str,
        flow: &str,
        action: &str,
        ask: bool,
        body: Value,
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
            let regno = &saved.registration_number;
            match self
                .post_with_token(server, path, flow, regno, &saved.token, &body)
                .await
            {
                Err(error) if matches!(error.code(), "token_invalid" | "token_expired") => {
                    info!(path, code = error.code(), "account token refused");
                    self.forget_account_token();
                }
                answer => return Ok(answer),
            }
        }
        if !ask {
            info!(path, "no account token: not proving unasked");
            return Err(CoreError::ConsentDenied);
        }
        info!(path, action, "no account token: proving");
        let (token, regno) = self.proved_token(server, flow).await?;
        Ok(self
            .post_with_token(server, path, flow, &regno, &token, &body)
            .await)
    }

    /// Proves the student and keeps the token it got; the token and their regno.
    #[cfg(feature = "tlsn")]
    async fn proved_token(&self, server: &str, flow: &str) -> Result<(String, String), CoreError> {
        let (token, expires_at, regno) = self.prove_account(server, flow).await?;
        let saved = Saved {
            server: server.to_string(),
            registration_number: regno.clone(),
            token: token.clone(),
            expires_at,
        };
        self.write_plain(FILE, &saved);
        info!(expires_at, "account token issued");
        Ok((token, regno))
    }

    #[cfg(not(feature = "tlsn"))]
    async fn proved_token(&self, _: &str, _: &str) -> Result<(String, String), CoreError> {
        Err(CoreError::Other {
            reason: "This build cannot prove who you are to the server.".into(),
        })
    }

    /// `body` with `token` to `path`; a renewed token in the answer is kept.
    async fn post_with_token(
        &self,
        server: &str,
        path: &str,
        flow: &str,
        regno: &str,
        token: &str,
        body: &Value,
    ) -> Result<Value, EdgeError> {
        let mut sent = body.clone();
        crate::json_set(&mut sent, "accountToken", json!(token));
        let answer = self.edge_answer(server, path, flow, &sent).await?;
        info!(path, "account token used");
        if let Some((token, expires_at)) = answer.account_token.clone() {
            let saved = Saved {
                server: server.to_string(),
                registration_number: regno.to_string(),
                token,
                expires_at,
            };
            self.write_plain(FILE, &saved);
            info!(path, expires_at, "account token renewed");
        }
        Ok(answer.body)
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
    use crate::remote::consent;
    use crate::test_edge::{Edge, serve, signed_in};
    use crate::unix_now;

    async fn setup(name: &str) -> (Core, Arc<Mutex<Edge>>, String) {
        let dir =
            std::env::temp_dir().join(format!("vtop-ffi-token-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let edge = Arc::new(Mutex::new(Edge::default()));
        let server = serve(edge.clone()).await;
        fs::write(dir.join(crate::remote::edge::SERVER_FILE), &server).unwrap();
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
    async fn a_kept_token_goes_alone_and_asks_nothing() {
        let (core, edge, server) = setup("used").await;
        keep(
            &core,
            &server,
            "22BCE0001",
            "tok",
            unix_now() * 1000 + DAY_MS,
        );
        can(&edge, 200, serde_json::from_str(VIEW).unwrap());
        let gate = consent::tests::no(&core);
        let apps = core.connected_apps("f".into()).await.unwrap();
        assert!(apps.linked);
        assert_eq!(gate.asks(), 0);
        assert_eq!(sent(&edge), [json!({ "accountToken": "tok" })]);
    }

    #[tokio::test]
    async fn a_renewed_token_in_the_answer_is_kept() {
        let (core, edge, server) = setup("renewed").await;
        keep(
            &core,
            &server,
            "22BCE0001",
            "tok",
            unix_now() * 1000 + DAY_MS,
        );
        edge.lock().unwrap().issue = Some("tok2".into());
        can(&edge, 200, serde_json::from_str(VIEW).unwrap());
        can(&edge, 200, serde_json::from_str(VIEW).unwrap());
        core.connected_apps("f".into()).await.unwrap();
        let kept = core.read::<Saved>(FILE).unwrap();
        assert_eq!((kept.token.as_str(), kept.server == server), ("tok2", true));
        assert!(kept.expires_at > unix_now() * 1000 + 13 * DAY_MS);
        core.connected_apps("f2".into()).await.unwrap();
        assert_eq!(
            sent(&edge),
            [
                json!({ "accountToken": "tok" }),
                json!({ "accountToken": "tok2" })
            ]
        );
    }

    #[tokio::test]
    async fn an_expired_token_is_sent_for_renewal_but_not_another_students() {
        let (core, edge, server) = setup("stale").await;
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
        assert_eq!(sent(&edge), [json!({ "accountToken": "old" })]);
        // Another student's is never sent: this one proves instead (and, with no proof
        // network here, fails).
        keep(
            &core,
            &server,
            "22BCE9999",
            "tok",
            unix_now() * 1000 + DAY_MS,
        );
        assert!(core.connected_apps("f2".into()).await.is_err());
        assert_eq!(sent(&edge).len(), 1);
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
