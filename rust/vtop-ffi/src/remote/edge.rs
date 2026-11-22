//! Plumbing every call to the edge (`edge/`) shares: which server, and the VTOP
//! session's cookies for the calls that send it.

use std::fs;

use vtop_core::{VtopClient, VtopError};

use crate::remote::consent::{self, CookieKind};
use crate::{Core, CoreError};

/// Optional one-line server URL (e.g. `http://127.0.0.1:8787` for `wrangler dev` behind
/// `adb reverse`), read when an address is set up; the address keeps the server it was made on.
pub(crate) const SERVER_FILE: &str = "forwarding_server";
/// The edge (`edge/`).
const DEFAULT_SERVER: &str = "https://vmate.aster0.dev";

impl Core {
    pub(crate) fn forwarding_server(&self) -> String {
        fs::read_to_string(self.at(SERVER_FILE))
            .ok()
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| DEFAULT_SERVER.to_string())
    }

    /// The signed-in VTOP session's cookies, after VTOP confirmed it (signing in again when it
    /// had expired), so the edge can tell whose session it is. Only once the student allowed it
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
        if !consent::allowed(&self.shared, flow, kind, action, &server, ask).await {
            return Err(CoreError::ConsentDenied);
        }
        #[expect(clippy::disallowed_methods, reason = "consent::allowed said yes above")]
        let cookies = |client: &VtopClient| client.export_persisted_session(0).cookies;
        signed_in!(self, |client| async {
            client.verify_session().await.map(|_| cookies(client))
        })
    }
}
