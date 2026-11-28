//! Which server the app talks to, and the VTOP session's cookies for the calls that send it.

use std::fs;

use vtop_core::{VtopClient, VtopError};

use crate::remote::consent;
use crate::remote::handover::{HandoverAsk, HandoverStatus};
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

    /// The signed-in VTOP session's cookies for a connected app's request, after VTOP confirmed
    /// it (signing in again when it had expired). Only once the student approved it on the sheet
    /// ([`consent::allowed_handover`], before any VTOP hit; else `ConsentDenied`); then the app
    /// hears [`HandoverStatus::Fetching`].
    pub(crate) async fn handover_cookies(
        &self,
        flow: &str,
        ask: HandoverAsk,
    ) -> Result<Option<String>, CoreError> {
        self.current()?;
        let (request_id, asker) = (ask.request_id.clone(), ask.asker.clone());
        consent::allowed_handover(&self.shared, flow, ask)
            .await
            .ok_or(CoreError::ConsentDenied)?;
        consent::handover_status(
            &self.shared,
            flow,
            &request_id,
            HandoverStatus::Fetching { asker },
        );
        self.checked_cookies().await
    }

    /// The session's cookies after VTOP confirmed it; only after a consent check said yes.
    async fn checked_cookies(&self) -> Result<Option<String>, CoreError> {
        #[expect(
            clippy::disallowed_methods,
            reason = "the callers' consent check said yes"
        )]
        let cookies = |client: &VtopClient| client.export_persisted_session(0).cookies;
        signed_in!(self, |client| async {
            client.verify_session().await.map(|_| cookies(client))
        })
    }
}
