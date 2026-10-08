//! Shared test helpers: a config, an app over in-memory parts, request
//! helpers.
#![allow(dead_code)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, Response};
use http_body_util::BodyExt;
use tower::ServiceExt;
use vtop_bridge::accounts::Accounts;
use vtop_bridge::config::Config;
use vtop_bridge::fakes::{FakeLogin, FakeRefresher, FakeVerifier};
use vtop_bridge::fcm::FakeMessenger;
use vtop_bridge::store::MemoryStore;
use vtop_bridge::{router, AppState, Parts};

/// The account the default fake login and verifier belong to.
pub const REG: &str = "22BCE0001";

pub fn config_with(extra: &[(&str, &str)]) -> Config {
    let extra: Vec<(String, String)> = extra
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    Config::from_lookup(|name| {
        if let Some((_, value)) = extra.iter().find(|(key, _)| key == name) {
            return Some(value.clone());
        }
        match name {
            "PUBLIC_BASE_URL" => Some("https://bridge.example.test"),
            "FIREBASE_CREDENTIALS_JSON" => Some("{}"),
            "VAULT_KEY" => Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="),
            _ => None,
        }
        .map(String::from)
    })
    .unwrap()
}

pub fn config() -> Config {
    config_with(&[])
}

/// An app over in-memory parts, with handles to them.
pub struct TestApp {
    pub store: Arc<MemoryStore>,
    pub messenger: Arc<FakeMessenger>,
    pub refresher: Arc<FakeRefresher>,
    pub login: Arc<FakeLogin>,
    pub state: Arc<AppState>,
}

impl TestApp {
    pub fn new() -> Self {
        Self::with_config(config())
    }

    pub fn with_config(config: Config) -> Self {
        Self::build(
            config,
            FakeLogin::default(),
            Arc::new(MemoryStore::default()),
        )
    }

    pub fn build(config: Config, login: FakeLogin, store: Arc<MemoryStore>) -> Self {
        let messenger = Arc::new(FakeMessenger::default());
        let refresher = Arc::new(FakeRefresher::default());
        let login = Arc::new(login);
        let state = Arc::new(AppState::new(
            config,
            Parts {
                store: store.clone(),
                messenger: messenger.clone(),
                refresher: refresher.clone(),
                login: login.clone(),
                verifier: Arc::new(FakeVerifier),
            },
        ));
        Self {
            store,
            messenger,
            refresher,
            login,
            state,
        }
    }

    pub fn accounts(&self) -> &Accounts {
        &self.state.accounts
    }

    pub async fn post_with_key(
        &self,
        path: &str,
        key: Option<&str>,
        body: serde_json::Value,
    ) -> Response<Body> {
        let mut request = Request::post(path).header("content-type", "application/json");
        if let Some(key) = key {
            request = request.header("x-service-key", key);
        }
        self.send(request.body(Body::from(body.to_string())).unwrap())
            .await
    }

    /// Polls a cookie request until it leaves `pending`.
    pub async fn settled_status(&self, id: &str) -> serde_json::Value {
        for _ in 0..200 {
            let body = json(self.get(&format!("/cookie/status/{id}")).await).await;
            if body["status"] != "pending" {
                return body;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        panic!("request {id} stayed pending");
    }

    pub async fn send(&self, request: Request<Body>) -> Response<Body> {
        router(self.state.clone()).oneshot(request).await.unwrap()
    }

    pub async fn post(&self, path: &str, body: serde_json::Value) -> Response<Body> {
        self.send(
            Request::post(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
    }

    pub async fn get(&self, path: &str) -> Response<Body> {
        self.send(Request::get(path).body(Body::empty()).unwrap())
            .await
    }

    pub async fn delete(&self, path: &str) -> Response<Body> {
        self.send(Request::delete(path).body(Body::empty()).unwrap())
            .await
    }

    /// The data of the last FCM message sent.
    pub fn last_fcm(&self) -> std::collections::BTreeMap<String, String> {
        self.messenger
            .sends()
            .last()
            .expect("no FCM message sent")
            .1
            .clone()
    }
}

pub async fn json(response: Response<Body>) -> serde_json::Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}
