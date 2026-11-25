//! Which server the app talks to, and the VTOP session's cookies for the calls that send it.

use std::fs;

use vtop_core::{VtopClient, VtopError};

use crate::remote::consent::{self, CookieKind};
use crate::{Core, CoreError};

/// Optional one-line server URL (e.g. `http://127.0.0.1:3210` for a local Convex behind
/// `adb reverse`), read when the server's client is made; an address keeps the server it was
/// made on.
pub(crate) const SERVER_FILE: &str = "forwarding_server";
/// The server (`backend/`, Convex). Point a debug build at a local one from
/// Settings → Developer → Server.
pub(crate) const DEFAULT_SERVER: &str = "https://vmate.aster0.dev";

/// [`DEFAULT_SERVER`], for the app's plain HTTP reads on the same host (the help pages).
#[uniffi::export]
pub fn default_server() -> String {
    DEFAULT_SERVER.into()
}

impl Core {
    pub(crate) fn forwarding_server(&self) -> String {
        fs::read_to_string(self.at(SERVER_FILE))
            .ok()
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| DEFAULT_SERVER.to_string())
    }

    /// The signed-in VTOP session's cookies, after VTOP confirmed it (signing in again when it
    /// had expired), so a connected app gets it. Only once the student allowed it
    /// for `action` ([`consent::allowed`], before any VTOP hit); else `ConsentDenied`.
    pub(crate) async fn session_cookies(
        &self,
        flow: &str,
        kind: CookieKind,
        action: &str,
        ask: bool,
    ) -> Result<Option<String>, CoreError> {
        self.current()?;
        let server = consent::host(&self.forwarding_server());
        if !consent::allowed(
            &self.shared,
            flow,
            kind,
            consent::CookiePayload::Cookie,
            action,
            &server,
            ask,
        )
        .await
        {
            return Err(CoreError::ConsentDenied);
        }
        #[expect(clippy::disallowed_methods, reason = "consent::allowed said yes above")]
        let cookies = |client: &VtopClient| client.export_persisted_session(0).cookies;
        signed_in!(self, |client| async {
            client.verify_session().await.map(|_| cookies(client))
        })
    }
}
