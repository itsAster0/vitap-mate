//! The in-app VTOP web view's session: the signed-in client's cookies, shaped
//! for Android's `CookieManager`, after VTOP confirmed them. Port of the
//! Flutter app's `vtop_webview_cookie_service.dart` (`loadVtopWebviewSession`).
//! Cookie names and values are never logged, only how many there are.

use tracing::info;
use vtop_core::config::DEFAULT_BASE_URL;
use vtop_core::{VtopClient, VtopError};

use crate::{Core, CoreError, traced};

/// Set for the parent domain, as the Flutter app does, so VTOP's pages under
/// `/vtop` all see them.
const COOKIE_DOMAIN: &str = ".vitap.ac.in";
const COOKIE_PATH: &str = "/vtop";
/// The page the Flutter app opens first: VTOP's signed-in dashboard.
const START_PATH: &str = "/vtop/content?";

/// What the web view needs to open VTOP signed in.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct WebSession {
    /// VTOP's origin, e.g. `https://vtop.vitap.ac.in`.
    pub base_url: String,
    /// The page to load first.
    pub start_url: String,
    /// Set these on a cleared cookie jar before loading [`Self::start_url`].
    pub cookies: Vec<WebCookie>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct WebCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
}

#[uniffi::export(async_runtime = "tokio")]
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
            let header = signed_in!(self, |client| async {
                client.verify_session().await.map(|_| header(client))
            })?;
            let session = web_session(&header);
            info!(cookies = session.cookies.len(), force, "web session");
            if session.cookies.is_empty() {
                return Err(CoreError::Other {
                    reason: "VTOP gave no session cookies".to_string(),
                });
            }
            Ok(session)
        })
        .await
    }
}

fn header(client: &VtopClient) -> String {
    client.cookie_header().unwrap_or_default()
}

/// Shapes a `Cookie` header (`a=1; b=2`) for the web view; pairs without a name or value are
/// dropped, as in the Flutter app's `parseCookiePair`.
fn web_session(header: &str) -> WebSession {
    let cookies = header
        .split(';')
        .filter_map(|pair| {
            let (name, value) = pair.split_once('=')?;
            let (name, value) = (name.trim(), value.trim());
            (!name.is_empty() && !value.is_empty()).then(|| WebCookie {
                name: name.to_string(),
                value: value.to_string(),
                domain: COOKIE_DOMAIN.to_string(),
                path: COOKIE_PATH.to_string(),
                secure: true,
            })
        })
        .collect();
    WebSession {
        base_url: DEFAULT_BASE_URL.to_string(),
        start_url: format!("{DEFAULT_BASE_URL}{START_PATH}"),
        cookies,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes_cookie_header_for_the_web_view() {
        let session = web_session(" JSESSIONID=abc=; SERVERID=s1 ;bad; =x; empty=; ");
        assert_eq!(session.start_url, "https://vtop.vitap.ac.in/vtop/content?");
        assert_eq!(session.base_url, "https://vtop.vitap.ac.in");
        let pairs: Vec<_> = session
            .cookies
            .iter()
            .map(|cookie| (cookie.name.as_str(), cookie.value.as_str()))
            .collect();
        assert_eq!(pairs, [("JSESSIONID", "abc="), ("SERVERID", "s1")]);
        assert!(session.cookies.iter().all(|cookie| cookie.secure
            && cookie.domain == ".vitap.ac.in"
            && cookie.path == "/vtop"));
        assert!(web_session("").cookies.is_empty());
    }

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
