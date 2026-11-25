//! The demo account, for UI tests, demos and store reviews without a VTOP account. Signing in
//! with [`USERNAME`] and [`PASSWORD`] asks VTOP nothing: it reads the sample pages from the
//! server (`guides:demo`), and every read page comes from them
//! ([`Core::demo_page`]). Calls that need a real session (outing writes, downloads, the web view,
//! forwarding) answer [`unavailable`], and nothing goes to the server.
//!
//! The pages are recorded from a real account in a debuggable build: create `demo_record.json`
//! in the app's data folder and every page fetched is added to it ([`Core::record_demo`]); then
//! `bun dev/demo.ts` in `backend/` swaps out the personal details.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{info, warn};
use vtop_core::sync::RwLock;

use crate::device::EdgeError;
use crate::remote::backend::Kind;
use crate::{Account, Core, CoreError, SignIn, Slot, dev, obs};

/// Not a VTOP registration number, so no student lands here by mistake.
pub(crate) const USERNAME: &str = "DEMO2026";
pub(crate) const PASSWORD: &str = "vitapmate-demo";

/// The downloaded pages, in the data folder.
pub(crate) const FILE: &str = "demo.json";
/// Present (debuggable builds only): every fetched page is added to it.
const RECORD: &str = "demo_record.json";
/// The JSON shape this app reads; the edge bumps it only for a change old apps can't read.
const SCHEMA: u32 = 1;

#[derive(Default, Serialize, Deserialize)]
struct Bundle {
    schema: u32,
    /// Each page's data, as the server route answers it, by [`key`].
    pages: BTreeMap<String, Value>,
}

/// `name`, `name@semester`, then `?key=value&…` for the arguments, in the caller's order.
fn key(name: &str, semester: Option<&str>, args: &[(&str, &str)]) -> String {
    let mut key = name.to_string();
    if let Some(semester) = semester {
        key.push('@');
        key.push_str(semester);
    }
    for (i, (arg, value)) in args.iter().enumerate() {
        key.push(if i == 0 { '?' } else { '&' });
        key.push_str(arg);
        key.push('=');
        key.push_str(value);
    }
    key
}

pub(crate) fn is(account: &Account) -> bool {
    account.username == USERNAME
}

pub(crate) fn unavailable() -> CoreError {
    CoreError::Other {
        reason: "Not in the demo.".into(),
    }
}

/// Err for the demo account: it has no VTOP session to call with.
pub(crate) fn refuse(account: &Account) -> Result<(), CoreError> {
    if is(account) {
        Err(unavailable())
    } else {
        Ok(())
    }
}

impl Core {
    /// Signed in as the demo account (Kotlin hears it as [`crate::AccountStatus::Demo`]).
    pub(crate) fn is_demo(&self) -> bool {
        self.read_account()
            .is_some_and(|saved| saved.username == USERNAME)
    }

    /// Signs in as the demo account: the sample pages from `server`, then the account saved
    /// like any other (another student's cached pages go).
    pub(crate) async fn sign_in_demo(
        &self,
        flow: &str,
        account: Account,
    ) -> Result<SignIn, CoreError> {
        let bundle = self
            .public_call(flow, Kind::Query, "guides:demo", json!({}))
            .await
            .map_err(|error| {
                warn!(code = error.code(), "demo not downloaded");
                match error {
                    EdgeError::Offline => CoreError::Offline,
                    EdgeError::Status { message, .. } => CoreError::VtopUnavailable {
                        reason: format!("The demo isn't available right now ({message})."),
                    },
                }
            })?;
        self.open_demo(account, bundle.to_string().into_bytes())
            .await
    }

    /// Signs in as the demo account on `bytes`, the downloaded sample pages.
    async fn open_demo(&self, account: Account, bytes: Vec<u8>) -> Result<SignIn, CoreError> {
        let bundle: Bundle = serde_json::from_slice(&bytes).map_err(|error| CoreError::Other {
            reason: format!("unreadable demo: {error}"),
        })?;
        if bundle.schema > SCHEMA {
            return Err(CoreError::Other {
                reason: "This demo needs a newer app.".into(),
            });
        }
        let slot = Arc::new(RwLock::new(crate::Session::new(account, &self.shared)?));
        self.install(Some(slot.clone()));
        self.save(&slot, &*slot.read().await);
        // After the save, which drops the last account's files (this one among them).
        self.write_bytes(FILE, &bytes);
        obs::set_regno(Some(USERNAME.into()));
        info!(pages = bundle.pages.len(), "demo signed in");
        Ok(SignIn::Done)
    }

    /// For the demo account, the page from the sample pages (the first of that name and
    /// semester when these arguments weren't recorded, as for another day's biometrics);
    /// None for any other account.
    pub(crate) async fn demo_page<T: DeserializeOwned>(
        &self,
        name: &str,
        semester: Option<&str>,
        args: &[(&str, &str)],
    ) -> Option<Result<T, CoreError>> {
        let slot: Arc<Slot> = self.current().ok()?;
        if !is(&slot.read().await.account) {
            return None;
        }
        let mut bundle = self.read::<Bundle>(FILE).unwrap_or_default();
        let wanted = key(name, semester, args);
        let base = key(name, semester, &[]);
        let found = bundle.pages.remove(&wanted).or_else(|| {
            let (_, page) = bundle
                .pages
                .into_iter()
                .find(|(key, _)| *key == base || key.starts_with(&format!("{base}?")))?;
            Some(page)
        });
        info!(page = wanted.as_str(), found = found.is_some(), "demo page");
        Some(
            found
                .ok_or_else(unavailable)
                .and_then(|page| serde_json::from_value(page).map_err(|_| unavailable())),
        )
    }

    /// Adds `data` to the recording when one was started (see the module's docs).
    pub(crate) fn record_demo(
        &self,
        name: &str,
        semester: Option<&str>,
        args: &[(&str, &str)],
        data: &impl Serialize,
    ) {
        static LOCK: Mutex<()> = Mutex::new(());
        if !dev::debuggable(&self.shared) || !self.at(RECORD).is_file() {
            return;
        }
        let _lock = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let mut bundle = self.read::<Bundle>(RECORD).unwrap_or_default();
        bundle.schema = SCHEMA;
        match serde_json::to_value(data) {
            Ok(value) => {
                bundle.pages.insert(key(name, semester, args), value);
                self.write_plain(RECORD, &bundle);
            }
            Err(error) => warn!(%error, "demo page not recorded"),
        }
    }
}

#[cfg(test)]
mod tests;
