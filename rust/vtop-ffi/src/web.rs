//! The in-app VTOP web view's session. The web view shares the app's cookie store
//! (`CookieManager`, see [`crate::cookies`]), so nothing is copied: this only makes sure the
//! session is signed in before the view loads VTOP. Port of the Flutter app's
//! `vtop_webview_cookie_service.dart` (`loadVtopWebviewSession`), less its cookie copying.

use tracing::info;
use vtop_core::VtopError;
use vtop_core::config::DEFAULT_BASE_URL;

use crate::{Core, CoreError, traced};

/// The page the Flutter app opens first: VTOP's signed-in dashboard.
const START_PATH: &str = "/vtop/content?";

/// What the web view needs to open VTOP signed in.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct WebSession {
    /// VTOP's origin, e.g. `https://vtop.vitap.ac.in`.
    pub base_url: String,
    /// The page to load first.
    pub start_url: String,
}

#[uniffi::export]
impl Core {
    /// The signed-in session for the web view, after VTOP confirmed it (signing in again with
    /// the saved password when it had expired). With `force`, signs in again first (the web
    /// view landed on VTOP's login page, or the student asked). Fails like a fetch: OTP
    /// pending, password rejected, offline.
    pub async fn web_session(&self, flow: String, force: bool) -> Result<WebSession, CoreError> {
        traced("web_session", &flow, async {
            if force {
                let slot = self.current()?;
                let seen = slot.read().await.logins;
                self.relogin(&slot, seen).await?;
            }
            signed_in!(self, |client| client.verify_session())?;
            info!(force, "web session");
            Ok(WebSession {
                base_url: DEFAULT_BASE_URL.to_string(),
                start_url: format!("{DEFAULT_BASE_URL}{START_PATH}"),
            })
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn signed_out_has_no_web_session() {
        // Not `Core::new`: that starts the process-wide log, which another test owns.
        let core = Core {
            dir: std::env::temp_dir().join(format!("vtop-ffi-web-{}", std::process::id())),
            slot: std::sync::Mutex::new(None),
        };
        assert_eq!(
            core.web_session("f-web".into(), false).await,
            Err(CoreError::NotSignedIn)
        );
        assert_eq!(
            core.web_session("f-web".into(), true).await,
            Err(CoreError::NotSignedIn)
        );
    }
}
