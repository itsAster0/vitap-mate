//! Hands out VTOP sessions per account: from the cache at once, or through
//! a pending request that a background task fulfils by asking the phone and
//! logging in with the vault, in the order the account's settings choose.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use vtop_core::gmail::GmailAccess;
use vtop_core::SessionState;

use crate::accounts::{account_id, AccountSettings, Accounts};
use crate::cookies::{self, CookieEditorCookie};
use crate::fcm::FcmError;
use crate::gmail::{GmailRefresher, RefreshError};
use crate::login::{LoginError, VtopLogin};
use crate::requests::PhoneRequests;
use crate::store::{Completion, RequestDoc, RequestStatus, Store, StoreError};
use crate::verify::{SessionVerifier, VerifyError};

pub const PHONE_UNREACHABLE: &str =
    "phone_unreachable: Open VITAP Mate → Connected apps → Reconnect this phone.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrokerError {
    /// No such request for this account.
    NotFound,
    Store(StoreError),
}

impl From<StoreError> for BrokerError {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

/// What `start` gives back.
pub enum Started {
    Ready(SessionState),
    Pending(RequestDoc),
}

/// A request as its client sees it.
#[derive(Debug, PartialEq, Eq)]
pub enum RequestView {
    Pending,
    Ready(SessionState),
    Error(String),
    Expired,
}

pub struct Broker {
    accounts: Arc<Accounts>,
    store: Arc<dyn Store>,
    phone: Arc<PhoneRequests>,
    refresher: Arc<dyn GmailRefresher>,
    login: Arc<dyn VtopLogin>,
    verifier: Arc<dyn SessionVerifier>,
    gmail_wait: Duration,
    /// Pending request per account, so concurrent callers share one.
    in_flight: tokio::sync::Mutex<HashMap<String, RequestDoc>>,
}

pub fn session_from_cookies(cookies_json: Option<&str>) -> SessionState {
    let cookies: Vec<CookieEditorCookie> = cookies_json
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default();
    SessionState {
        cookies: cookies::to_header(&cookies),
        csrf_token: None,
        registration_number: None,
        otp_issued_at: None,
        logged_in_at: None,
    }
}

impl Broker {
    pub fn new(
        accounts: Arc<Accounts>,
        store: Arc<dyn Store>,
        phone: Arc<PhoneRequests>,
        refresher: Arc<dyn GmailRefresher>,
        login: Arc<dyn VtopLogin>,
        verifier: Arc<dyn SessionVerifier>,
        gmail_wait: Duration,
    ) -> Self {
        Self {
            accounts,
            store,
            phone,
            refresher,
            login,
            verifier,
            gmail_wait,
            in_flight: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    /// The cached session, or a pending request being fulfilled in the
    /// background.
    pub async fn start(
        self: &Arc<Self>,
        registration_number: &str,
    ) -> Result<Started, BrokerError> {
        let id = account_id(registration_number);
        let now = vtop_core::now_unix();
        if let Some(session) = self.usable_cache(&id, now).await? {
            return Ok(Started::Ready(session));
        }
        // Held while a request is created, so concurrent callers share it.
        let mut in_flight = self.in_flight.lock().await;
        if let Some(doc) = in_flight.get(&id) {
            let still_pending = now < doc.expires_at
                && matches!(
                    self.store.get_request(&doc.id).await?,
                    Some(current) if current.status == RequestStatus::Pending
                );
            if still_pending {
                return Ok(Started::Pending(doc.clone()));
            }
            in_flight.remove(&id);
            // It may have finished since the cache was read.
            if let Some(session) = self.accounts.cached_session(&id, now).await? {
                return Ok(Started::Ready(session));
            }
        }
        let doc = self.phone.create_for_account(&id).await?;
        in_flight.insert(id.clone(), doc.clone());
        drop(in_flight);
        tokio::spawn(Arc::clone(self).fulfil(id, doc.clone()));
        Ok(Started::Pending(doc))
    }

    /// The cached session, checked with VTOP before it is handed out. A
    /// session VTOP rejects (or that belongs to someone else) is dropped;
    /// when VTOP cannot be reached the cache is handed out as is.
    async fn usable_cache(&self, id: &str, now: u64) -> Result<Option<SessionState>, BrokerError> {
        let Some(session) = self.accounts.cached_session(id, now).await? else {
            return Ok(None);
        };
        match self.verifier.registration_number(&session).await {
            Ok(owner) if owner == id => Ok(Some(session)),
            Err(VerifyError::Unavailable(reason)) => {
                tracing::info!(
                    "account {id}: cannot check the cached session ({reason}); using it"
                );
                Ok(Some(session))
            }
            Ok(_) | Err(VerifyError::Expired) => {
                tracing::info!("account {id}: cached session no longer works; dropping it");
                self.accounts.clear_session(id).await?;
                Ok(None)
            }
        }
    }

    /// How a request of this account is doing.
    pub async fn poll(
        &self,
        registration_number: &str,
        request_id: &str,
    ) -> Result<RequestView, BrokerError> {
        let id = account_id(registration_number);
        let doc = match self.store.get_request(request_id).await? {
            Some(doc) if doc.account.as_deref() == Some(id.as_str()) => doc,
            _ => return Err(BrokerError::NotFound),
        };
        if vtop_core::now_unix() >= doc.expires_at {
            // Kept so a late phone answer can still be cached; Firestore's
            // TTL removes it.
            return Ok(RequestView::Expired);
        }
        Ok(match doc.status {
            RequestStatus::Pending => RequestView::Pending,
            RequestStatus::Success => {
                RequestView::Ready(session_from_cookies(doc.cookies_json.as_deref()))
            }
            RequestStatus::Error => RequestView::Error(doc.error.unwrap_or_default()),
        })
    }

    /// Drops the cached session (VTOP said it expired).
    pub async fn expire(&self, registration_number: &str) -> Result<(), BrokerError> {
        Ok(self
            .accounts
            .clear_session(&account_id(registration_number))
            .await?)
    }

    /// Background. By default a vault login, else the phone. When the
    /// account does not always use its saved credentials, the phone is
    /// asked first and the vault login runs only if it stays silent for the
    /// chosen wait (or cannot be reached).
    async fn fulfil(self: Arc<Self>, id: String, doc: RequestDoc) {
        let settings = match self.accounts.settings(&id).await {
            Ok(settings) => settings,
            Err(error) => {
                tracing::warn!("account {id}: read settings: {error}");
                AccountSettings::default()
            }
        };
        let has_vault = matches!(self.accounts.vault(&id).await, Ok(Some(_)));
        if has_vault && !settings.always_use_vault {
            let phone = self.notify_phone(&id, &doc, &settings).await;
            if phone.is_ok()
                && self
                    .phone_answered_within(&doc, Duration::from_secs(settings.phone_wait_secs))
                    .await
            {
                return;
            }
            match self.vault_login(&id).await {
                Some(session) => self.complete_with(&doc, &session).await,
                // The phone may still answer; the request stays pending.
                None if phone.is_ok() => {}
                None => self.complete(&doc, failed(phone.unwrap_err())).await,
            }
            return;
        }
        match self.vault_login(&id).await {
            Some(session) => self.complete_with(&doc, &session).await,
            None => {
                if let Err(message) = self.notify_phone(&id, &doc, &settings).await {
                    self.complete(&doc, failed(message)).await;
                }
            }
        }
    }

    /// Whether the request left `Pending` (the phone answered) in time.
    async fn phone_answered_within(&self, doc: &RequestDoc, wait: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + wait;
        while tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(500)).await;
            match self.store.get_request(&doc.id).await {
                Ok(Some(current)) if current.status == RequestStatus::Pending => {}
                Ok(_) => return true,
                Err(error) => tracing::warn!("read session request {}: {error}", doc.id),
            }
        }
        false
    }

    async fn complete_with(&self, doc: &RequestDoc, session: &SessionState) {
        let cookies = serde_json::to_string(&cookies::from_header(&session.cookies))
            .expect("cookies serialise");
        self.complete(
            doc,
            Completion {
                status: RequestStatus::Success,
                cookies_json: Some(cookies),
                error: None,
            },
        )
        .await;
    }

    /// Logs in with the saved credentials. `None` when there are none, they
    /// were wiped, or the login failed for now (the vault is kept then).
    async fn vault_login(&self, id: &str) -> Option<SessionState> {
        let vault = match self.accounts.vault(id).await {
            Ok(vault) => vault?,
            Err(error) => {
                tracing::warn!("account {id}: read vault: {error}");
                return None;
            }
        };
        let token = match self.refresher.refresh(&vault.gmail).await {
            Ok(token) => token,
            Err(RefreshError::InvalidGrant) => {
                tracing::info!("account {id}: Gmail grant revoked; clearing saved credentials");
                self.clear_vault(id).await;
                return None;
            }
            Err(RefreshError::Unavailable(reason)) => {
                tracing::info!("account {id}: Gmail refresh failed for now: {reason}");
                return None;
            }
        };
        let gmail = GmailAccess {
            access_token: token.access_token,
            delete_after_reading: vault.gmail.delete_after_reading,
            expires_at: Some(token.expires_at),
        };
        match self
            .login
            .login(&vault.username, &vault.password, gmail, self.gmail_wait)
            .await
        {
            Ok(session) => {
                let owner = session.registration_number.as_deref().map(account_id);
                if owner.as_deref() != Some(id) {
                    tracing::warn!(
                        "account {id}: saved credentials belong to another student; clearing them"
                    );
                    self.clear_vault(id).await;
                    return None;
                }
                if let Err(error) = self
                    .accounts
                    .save_session(id, &session, vtop_core::now_unix())
                    .await
                {
                    tracing::warn!("account {id}: cache session: {error}");
                }
                tracing::info!("account {id}: signed in with saved credentials");
                Some(session)
            }
            Err(LoginError::InvalidCredentials) => {
                tracing::info!("account {id}: VTOP rejected saved credentials; clearing them");
                self.clear_vault(id).await;
                None
            }
            Err(LoginError::Failed(reason)) => {
                tracing::info!(
                    "account {id}: login with saved credentials failed for now: {reason}"
                );
                None
            }
        }
    }

    /// Sends the request to the account's phone; the callback completes it.
    /// `Err` carries the error the request should end with if nothing else
    /// answers it. The phone is asked to resend credentials when none are
    /// saved, or when saved ones expire (each answer restarts the timer).
    async fn notify_phone(
        &self,
        id: &str,
        doc: &RequestDoc,
        settings: &AccountSettings,
    ) -> Result<(), &'static str> {
        let fcm_token = match self.accounts.fcm_token(id).await {
            Ok(Some(token)) => token,
            Ok(None) => return Err(PHONE_UNREACHABLE),
            Err(store_error) => {
                tracing::warn!("account {id}: read FCM token: {store_error}");
                return Err("store_unavailable: try again shortly.");
            }
        };
        let want_credentials =
            matches!(self.accounts.vault(id).await, Ok(None)) || settings.vault_ttl_secs.is_some();
        match self.phone.notify(doc, &fcm_token, want_credentials).await {
            Ok(()) => Ok(()),
            Err(FcmError::TokenInvalid) => {
                tracing::info!("account {id}: FCM token unregistered; clearing it");
                if let Err(store_error) = self.accounts.clear_fcm_token(id).await {
                    tracing::warn!("account {id}: clear FCM token: {store_error}");
                }
                Err(PHONE_UNREACHABLE)
            }
            Err(other) => {
                tracing::warn!("account {id}: FCM send failed: {other:?}");
                Err("phone_unavailable: Could not reach the phone right now.")
            }
        }
    }

    async fn clear_vault(&self, id: &str) {
        if let Err(error) = self.accounts.clear_vault(id).await {
            tracing::warn!("account {id}: clear vault: {error}");
        }
    }

    async fn complete(&self, doc: &RequestDoc, completion: Completion) {
        let token = doc.response_token.as_deref().unwrap_or_default();
        if let Err(error) = self
            .store
            .complete_request(&doc.id, token, completion, vtop_core::now_unix())
            .await
        {
            tracing::warn!("complete session request {}: {error:?}", doc.id);
        }
    }
}

fn failed(message: &str) -> Completion {
    Completion {
        status: RequestStatus::Error,
        cookies_json: None,
        error: Some(message.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{AccountSettings, Vault};
    use crate::fakes::{session, FakeLogin, FakeRefresher};
    use crate::fcm::FakeMessenger;
    use crate::gmail::GmailGrant;
    use crate::store::MemoryStore;
    use crate::vault::Sealer;

    const REG: &str = "22BCE0001";

    struct Rig {
        store: Arc<MemoryStore>,
        messenger: Arc<FakeMessenger>,
        refresher: Arc<FakeRefresher>,
        login: Arc<FakeLogin>,
        accounts: Arc<Accounts>,
        broker: Arc<Broker>,
    }

    fn rig_with(login: FakeLogin) -> Rig {
        let store = Arc::new(MemoryStore::default());
        let messenger = Arc::new(FakeMessenger::default());
        let refresher = Arc::new(FakeRefresher::default());
        let login = Arc::new(login);
        let accounts = Arc::new(Accounts::new(
            store.clone(),
            Arc::new(Sealer::new([5; 32])),
            Duration::from_secs(1800),
        ));
        let phone = Arc::new(PhoneRequests {
            store: store.clone(),
            messenger: messenger.clone(),
            public_base_url: "https://bridge.example".into(),
            request_timeout: Duration::from_secs(120),
        });
        let broker = Arc::new(Broker::new(
            accounts.clone(),
            store.clone(),
            phone,
            refresher.clone(),
            login.clone(),
            Arc::new(crate::fakes::FakeVerifier),
            Duration::from_secs(1),
        ));
        Rig {
            store,
            messenger,
            refresher,
            login,
            accounts,
            broker,
        }
    }

    fn rig() -> Rig {
        rig_with(FakeLogin::answering(Ok(owned_session(
            "JSESSIONID=fresh; REG=22BCE0001",
            REG,
        ))))
    }

    fn owned_session(cookies: &str, registration_number: &str) -> SessionState {
        SessionState {
            registration_number: Some(registration_number.into()),
            ..session(cookies)
        }
    }

    fn vault() -> Vault {
        Vault {
            username: "yaswanth314".into(),
            password: "pw".into(),
            gmail: GmailGrant {
                refresh_token: "r".into(),
                client_id: "c".into(),
                client_secret: None,
                delete_after_reading: false,
            },
        }
    }

    /// A linked account whose cache has expired.
    async fn linked(rig: &Rig, with_vault: bool, fcm_token: Option<&str>) {
        rig.accounts
            .record_phone(REG, fcm_token, &session("A=old"), with_vault.then(vault), 1)
            .await
            .unwrap();
    }

    async fn set(rig: &Rig, settings: AccountSettings) {
        rig.accounts
            .set_settings(REG, settings, vtop_core::now_unix())
            .await
            .unwrap();
    }

    /// The phone is asked first; the vault waits for the default 20 s.
    async fn phone_first(rig: &Rig) {
        set(
            rig,
            AccountSettings {
                always_use_vault: false,
                ..AccountSettings::default()
            },
        )
        .await;
    }

    /// Polls until the request leaves `Pending`, in steps of `step`.
    async fn settle_slowly(rig: &Rig, id: &str) -> RequestView {
        for _ in 0..2_000 {
            match rig.broker.poll(REG, id).await.unwrap() {
                RequestView::Pending => tokio::time::sleep(Duration::from_millis(100)).await,
                other => return other,
            }
        }
        panic!("request stayed pending");
    }

    /// The phone answers with `cookies` (the callback handler's job).
    async fn phone_answers(rig: &Rig, id: &str, token: &str, cookies: &str) {
        let cookies = serde_json::to_string(&cookies::from_header(cookies)).unwrap();
        rig.store
            .complete_request(
                id,
                token,
                Completion {
                    status: RequestStatus::Success,
                    cookies_json: Some(cookies),
                    error: None,
                },
                vtop_core::now_unix(),
            )
            .await
            .unwrap();
    }

    async fn pending_id(rig: &Rig) -> String {
        match rig.broker.start(REG).await.unwrap() {
            Started::Pending(doc) => doc.id,
            Started::Ready(_) => panic!("expected a pending request"),
        }
    }

    /// Polls until the request leaves `Pending`.
    async fn settle(rig: &Rig, id: &str) -> RequestView {
        for _ in 0..400 {
            match rig.broker.poll(REG, id).await.unwrap() {
                RequestView::Pending => tokio::time::sleep(Duration::from_millis(5)).await,
                other => return other,
            }
        }
        panic!("request stayed pending");
    }

    /// Waits for the FCM message and returns its data.
    async fn fcm_sent(rig: &Rig) -> std::collections::BTreeMap<String, String> {
        for _ in 0..400 {
            if let Some((_, data)) = rig.messenger.sends().last().cloned() {
                return data;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("no FCM message");
    }

    #[tokio::test]
    async fn cache_hit_is_ready() {
        let rig = rig();
        rig.accounts
            .save_session(
                REG,
                &session("A=cached; REG=22BCE0001"),
                vtop_core::now_unix(),
            )
            .await
            .unwrap();
        match rig.broker.start(REG).await.unwrap() {
            Started::Ready(state) => assert_eq!(state, session("A=cached; REG=22BCE0001")),
            Started::Pending(_) => panic!("expected ready"),
        }
        assert_eq!(rig.login.calls(), 0);
        assert!(rig.messenger.sends().is_empty());
    }

    #[tokio::test]
    async fn vault_login_completes_request_and_caches() {
        let rig = rig();
        linked(&rig, true, Some("fcm")).await;
        let id = pending_id(&rig).await;
        assert_eq!(
            settle(&rig, &id).await,
            RequestView::Ready(session("JSESSIONID=fresh; REG=22BCE0001"))
        );
        assert_eq!(rig.login.calls(), 1);
        assert!(rig.messenger.sends().is_empty());
        assert!(matches!(
            rig.broker.start(REG).await.unwrap(),
            Started::Ready(_)
        ));
    }

    #[tokio::test]
    async fn vault_login_with_other_regno_wipes_vault_and_asks_phone() {
        let rig = rig_with(FakeLogin::answering(Ok(owned_session(
            "A=other",
            "21BCE9999",
        ))));
        linked(&rig, true, Some("fcm")).await;
        pending_id(&rig).await;
        let data = fcm_sent(&rig).await;
        assert_eq!(data["wantCredentials"], "1");
        assert!(rig.accounts.vault(REG).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn invalid_credentials_wipes_and_asks_phone_with_want_credentials() {
        let rig = rig_with(FakeLogin::answering(Err(LoginError::InvalidCredentials)));
        linked(&rig, true, Some("fcm")).await;
        let id = pending_id(&rig).await;
        let data = fcm_sent(&rig).await;
        assert_eq!(data["requestId"], id);
        assert_eq!(data["wantCredentials"], "1");
        assert!(rig.accounts.vault(REG).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn invalid_grant_wipes_and_asks_phone() {
        let rig = rig();
        *rig.refresher.result.lock().unwrap() = Err(RefreshError::InvalidGrant);
        linked(&rig, true, Some("fcm")).await;
        pending_id(&rig).await;
        assert_eq!(fcm_sent(&rig).await["wantCredentials"], "1");
        assert!(rig.accounts.vault(REG).await.unwrap().is_none());
        assert_eq!(rig.login.calls(), 0);
    }

    #[tokio::test]
    async fn transient_login_error_keeps_vault_and_asks_phone_without_credentials() {
        let rig = rig_with(FakeLogin::answering(Err(LoginError::Failed(
            "VTOP down".into(),
        ))));
        linked(&rig, true, Some("fcm")).await;
        pending_id(&rig).await;
        assert_eq!(fcm_sent(&rig).await["wantCredentials"], "0");
        assert!(rig.accounts.vault(REG).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn no_vault_asks_phone_and_its_answer_is_ready() {
        let rig = rig();
        linked(&rig, false, Some("fcm")).await;
        let id = pending_id(&rig).await;
        let data = fcm_sent(&rig).await;
        assert_eq!(rig.messenger.sends()[0].0, "fcm");
        // The phone answers (the callback handler's job, done by hand here).
        let cookies = serde_json::to_string(&cookies::from_header("A=phone")).unwrap();
        rig.store
            .complete_request(
                &id,
                &data["responseToken"],
                Completion {
                    status: RequestStatus::Success,
                    cookies_json: Some(cookies),
                    error: None,
                },
                vtop_core::now_unix(),
            )
            .await
            .unwrap();
        assert_eq!(
            settle(&rig, &id).await,
            RequestView::Ready(session("A=phone"))
        );
    }

    #[tokio::test]
    async fn no_fcm_token_is_phone_unreachable() {
        let rig = rig();
        linked(&rig, false, None).await;
        let id = pending_id(&rig).await;
        assert_eq!(
            settle(&rig, &id).await,
            RequestView::Error(PHONE_UNREACHABLE.into())
        );
        assert!(rig.messenger.sends().is_empty());
    }

    #[tokio::test]
    async fn unregistered_token_is_cleared_and_phone_unreachable() {
        let rig = rig();
        *rig.messenger.result.lock().unwrap() = Err(FcmError::TokenInvalid);
        linked(&rig, false, Some("dead")).await;
        let id = pending_id(&rig).await;
        assert_eq!(
            settle(&rig, &id).await,
            RequestView::Error(PHONE_UNREACHABLE.into())
        );
        assert!(rig.accounts.fcm_token(REG).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn fcm_outage_is_an_error() {
        let rig = rig();
        *rig.messenger.result.lock().unwrap() = Err(FcmError::Unavailable);
        linked(&rig, false, Some("fcm")).await;
        let id = pending_id(&rig).await;
        assert!(
            matches!(settle(&rig, &id).await, RequestView::Error(message) if message.starts_with("phone_unavailable"))
        );
        assert!(rig.accounts.fcm_token(REG).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn poll_other_accounts_request_is_not_found() {
        let rig = rig();
        linked(&rig, false, Some("fcm")).await;
        let id = pending_id(&rig).await;
        assert_eq!(
            rig.broker.poll("21BCE9999", &id).await,
            Err(BrokerError::NotFound)
        );
        assert_eq!(
            rig.broker.poll(REG, "missing").await,
            Err(BrokerError::NotFound)
        );
    }

    #[tokio::test]
    async fn poll_expired_request() {
        let rig = rig();
        let doc = RequestDoc {
            id: "old".into(),
            response_token: Some("t".into()),
            status: RequestStatus::Pending,
            cookies_json: None,
            error: None,
            fcm_hash: None,
            want_credentials: false,
            account: Some(REG.into()),
            created_at: 1,
            expires_at: 2,
        };
        rig.store.create_request(&doc).await.unwrap();
        assert_eq!(rig.broker.poll(REG, "old").await, Ok(RequestView::Expired));
        // Kept for a late phone answer; Firestore's TTL removes it.
        assert!(rig.store.get_request("old").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn concurrent_starts_share_one_request_and_log_in_once() {
        let login = FakeLogin {
            delay: Duration::from_millis(50),
            ..FakeLogin::answering(Ok(owned_session("JSESSIONID=fresh; REG=22BCE0001", REG)))
        };
        let rig = rig_with(login);
        linked(&rig, true, Some("fcm")).await;
        let (a, b) = tokio::join!(rig.broker.start(REG), rig.broker.start("22bce0001"));
        let ids: Vec<String> = [a.unwrap(), b.unwrap()]
            .into_iter()
            .map(|started| match started {
                Started::Pending(doc) => doc.id,
                Started::Ready(_) => panic!("expected pending"),
            })
            .collect();
        assert_eq!(ids[0], ids[1]);
        settle(&rig, &ids[0]).await;
        assert_eq!(rig.login.calls(), 1);
    }

    #[tokio::test]
    async fn expire_clears_only_session() {
        let rig = rig();
        linked(&rig, true, Some("fcm")).await;
        rig.accounts
            .save_session(REG, &session("A=1"), vtop_core::now_unix())
            .await
            .unwrap();
        rig.broker.expire("22bce0001").await.unwrap();
        assert!(rig
            .accounts
            .cached_session(REG, vtop_core::now_unix())
            .await
            .unwrap()
            .is_none());
        assert!(rig.accounts.vault(REG).await.unwrap().is_some());
        assert_eq!(account_id("22bce0001"), REG);
    }

    #[tokio::test(start_paused = true)]
    async fn phone_answer_within_the_wait_skips_the_vault() {
        let rig = rig();
        linked(&rig, true, Some("fcm")).await;
        phone_first(&rig).await;
        let id = pending_id(&rig).await;
        let data = fcm_sent(&rig).await;
        assert_eq!(data["wantCredentials"], "0");
        phone_answers(&rig, &id, &data["responseToken"], "A=phone").await;
        assert_eq!(
            settle_slowly(&rig, &id).await,
            RequestView::Ready(session("A=phone"))
        );
        tokio::time::sleep(Duration::from_secs(60)).await;
        assert_eq!(rig.login.calls(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn silent_phone_falls_back_to_the_vault_after_the_wait() {
        let rig = rig();
        linked(&rig, true, Some("fcm")).await;
        phone_first(&rig).await;
        let started = tokio::time::Instant::now();
        let id = pending_id(&rig).await;
        assert_eq!(
            settle_slowly(&rig, &id).await,
            RequestView::Ready(session("JSESSIONID=fresh; REG=22BCE0001"))
        );
        assert!(started.elapsed() >= Duration::from_secs(20));
        assert!(started.elapsed() < Duration::from_secs(25));
        assert_eq!(rig.messenger.sends().len(), 1);
        assert_eq!(rig.login.calls(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn unreachable_phone_skips_the_wait() {
        let rig = rig();
        linked(&rig, true, None).await;
        phone_first(&rig).await;
        let started = tokio::time::Instant::now();
        let id = pending_id(&rig).await;
        assert_eq!(
            settle_slowly(&rig, &id).await,
            RequestView::Ready(session("JSESSIONID=fresh; REG=22BCE0001"))
        );
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[tokio::test]
    async fn expiring_credentials_ask_the_phone_to_resend_them() {
        let rig = rig();
        linked(&rig, true, Some("fcm")).await;
        set(
            &rig,
            AccountSettings {
                vault_ttl_secs: Some(86_400),
                always_use_vault: false,
                ..AccountSettings::default()
            },
        )
        .await;
        pending_id(&rig).await;
        assert_eq!(fcm_sent(&rig).await["wantCredentials"], "1");
    }

    #[tokio::test]
    async fn cached_session_is_checked_with_vtop_before_it_is_handed_out() {
        let rig = rig();
        linked(&rig, false, Some("fcm")).await;
        let live = session("JSESSIONID=a; REG=22BCE0001");
        rig.accounts
            .save_session(REG, &live, vtop_core::now_unix())
            .await
            .unwrap();
        assert!(matches!(rig.broker.start(REG).await.unwrap(), Started::Ready(s) if s == live));

        rig.accounts
            .save_session(REG, &session("JSESSIONID=dead"), vtop_core::now_unix())
            .await
            .unwrap();
        assert!(matches!(
            rig.broker.start(REG).await.unwrap(),
            Started::Pending(_)
        ));
        assert!(rig
            .accounts
            .cached_session(REG, vtop_core::now_unix())
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn cached_session_for_another_student_is_dropped() {
        let rig = rig();
        linked(&rig, false, Some("fcm")).await;
        rig.accounts
            .save_session(
                REG,
                &session("JSESSIONID=b; REG=21BCE9999"),
                vtop_core::now_unix(),
            )
            .await
            .unwrap();
        assert!(matches!(
            rig.broker.start(REG).await.unwrap(),
            Started::Pending(_)
        ));
    }

    #[tokio::test]
    async fn vtop_down_hands_out_the_cache_unchecked() {
        let rig = rig();
        linked(&rig, false, Some("fcm")).await;
        let cached = session("JSESSIONID=a; down=1");
        rig.accounts
            .save_session(REG, &cached, vtop_core::now_unix())
            .await
            .unwrap();
        assert!(matches!(rig.broker.start(REG).await.unwrap(), Started::Ready(s) if s == cached));
    }
}
