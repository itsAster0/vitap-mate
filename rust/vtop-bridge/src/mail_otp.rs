//! Forwarded OTP mail. The student forwards VTOP's OTP email from Gmail to
//! `vtop-otp-<id>@<MAIL_DOMAIN>`; the Cloudflare mail Worker seals the code
//! to the phone's public key and hands the box to this relay, which pushes
//! it over FCM and keeps it a few minutes for the app to pull. The bridge
//! never sees the code: it has no private key.
//!
//! App routes (rate limited per IP, the mailbox secret proves ownership):
//! `POST /v1/mail-otp/register`, `/update`, `/delete`, `/inbox`, `/owner`.
//! `POST /v1/mail-otp/reclaim` instead takes a live VTOP session: VTOP says
//! whose it is, and that student's address moves to the calling phone.
//! Worker routes (`Authorization: Bearer $MAIL_RELAY_SECRET`):
//! `GET /v1/mail-otp/relay/{id}` (public key), `POST` (deliver a box).

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use rand::Rng;
use serde::Deserialize;
use serde_json::json;
use sha2::Digest;
use vtop_core::mail_seal;

use crate::error::ApiError;
use crate::fcm::FcmError;
use crate::keys;
use crate::store::{tokens_equal, MailboxDoc};
use crate::verify::VerifyError;
use crate::AppState;

/// How long a delivered box stays pullable.
pub const SEALED_TTL_SECS: u64 = 300;
const ID_LEN: usize = 10;
/// Lower-case letters and digits without look-alikes (no l, o, 0, 1).
const ID_ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789";
const MAX_SEALED_LEN: usize = 2048;
const MAX_FCM_TOKEN_LEN: usize = 4096;

pub fn new_id() -> String {
    let mut rng = rand::rngs::OsRng;
    (0..ID_LEN)
        .map(|_| ID_ALPHABET[rng.gen_range(0..ID_ALPHABET.len())] as char)
        .collect()
}

pub fn is_id(id: &str) -> bool {
    id.len() == ID_LEN && id.bytes().all(|byte| ID_ALPHABET.contains(&byte))
}

pub fn address(id: &str, domain: &str) -> String {
    format!("vtop-otp-{id}@{domain}")
}

fn bad_request(message: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "bad_request", message)
}

fn store_unavailable(error: impl std::fmt::Display) -> ApiError {
    tracing::warn!("mail-otp store: {error}");
    ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "store_unavailable",
        "The store is unavailable.",
    )
}

fn unknown_mailbox() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        "mailbox_not_found",
        "No such forwarding address, or the secret is wrong.",
    )
}

fn fcm_token(raw: Option<&str>) -> Result<Option<String>, ApiError> {
    match raw.map(str::trim) {
        None => Ok(None),
        Some(token) if token.is_empty() || token.len() > MAX_FCM_TOKEN_LEN => {
            Err(bad_request("fcmToken is not an FCM token."))
        }
        Some(token) => Ok(Some(token.to_string())),
    }
}

fn public_key(raw: Option<&str>) -> Result<Option<String>, ApiError> {
    match raw.map(str::trim) {
        None => Ok(None),
        Some(key) if mail_seal::is_public_key(key) => Ok(Some(key.to_string())),
        Some(_) => Err(bad_request("publicKey must be a base64url X25519 key.")),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterBody {
    #[serde(default)]
    public_key: Option<String>,
    #[serde(default)]
    fcm_token: Option<String>,
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    body: Result<Json<RegisterBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(body) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    let public_key = public_key(body.public_key.as_deref())?
        .ok_or_else(|| bad_request("publicKey is required."))?;
    let fcm_token = fcm_token(body.fcm_token.as_deref())?
        .ok_or_else(|| bad_request("fcmToken is required."))?;
    // 50 random bits: a clash is unlikely, but never overwrite one.
    let mut id = new_id();
    for _ in 0..3 {
        match state.store.get_mailbox(&id).await {
            Ok(None) => break,
            Ok(Some(_)) => id = new_id(),
            Err(error) => return Err(store_unavailable(error)),
        }
    }
    let secret = keys::new_mailbox_secret();
    let now = vtop_core::now_unix();
    state
        .store
        .put_mailbox(&MailboxDoc {
            id: id.clone(),
            public_key,
            fcm_token,
            secret_hash: keys::hash(&secret),
            owner_hash: None,
            sealed: None,
            sealed_at: None,
            created_at: now,
            updated_at: now,
        })
        .await
        .map_err(store_unavailable)?;
    tracing::info!("mail-otp: registered a mailbox");
    let body = json!({
        "id": id,
        "address": address(&id, &state.config.mail_domain),
        "secret": secret,
    });
    Ok((StatusCode::CREATED, Json(body)).into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerBody {
    #[serde(default)]
    id: String,
    #[serde(default)]
    secret: String,
    #[serde(default)]
    public_key: Option<String>,
    #[serde(default)]
    fcm_token: Option<String>,
    #[serde(default)]
    session: Option<SessionBody>,
}

#[derive(Deserialize)]
pub struct SessionBody {
    #[serde(default)]
    cookies: String,
}

/// A keyed hash of a registration number, so the store never holds the
/// number and nobody can work one out from a stored hash without the key.
pub fn owner_hash(vault_key: &[u8; 32], registration_number: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(b"vitapmate mail-otp owner v1\0");
    hasher.update(vault_key);
    hasher.update(b"\0");
    hasher.update(registration_number.trim().to_uppercase().as_bytes());
    hex::encode(hasher.finalize())
}

/// Asks VTOP whose `session` is and returns that student's owner hash.
/// The session is used for this check only and never stored.
async fn session_owner(state: &AppState, session: Option<SessionBody>) -> Result<String, ApiError> {
    let cookies = session
        .map(|session| session.cookies.trim().to_string())
        .filter(|cookies| !cookies.is_empty())
        .ok_or_else(|| bad_request("session.cookies is required."))?;
    let session = vtop_core::SessionState {
        cookies,
        csrf_token: None,
        registration_number: None,
        otp_issued_at: None,
        logged_in_at: None,
    };
    match state.verifier.registration_number(&session).await {
        Ok(account) => Ok(owner_hash(&state.config.vault_key, &account)),
        Err(VerifyError::Expired) => Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "session_invalid",
            "VTOP does not accept this session; sign in again.",
        )),
        Err(VerifyError::Unavailable(reason)) => {
            tracing::info!("mail-otp verify session: {reason}");
            Err(ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "vtop_unreachable",
                "VTOP is unreachable; try again.",
            ))
        }
    }
}

/// The mailbox, when `secret` is its owner's.
async fn owned(
    state: &AppState,
    body: Result<Json<OwnerBody>, JsonRejection>,
) -> Result<(MailboxDoc, OwnerBody), ApiError> {
    let Json(body) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    if !is_id(&body.id) || body.secret.is_empty() {
        return Err(unknown_mailbox());
    }
    let doc = state
        .store
        .get_mailbox(&body.id)
        .await
        .map_err(store_unavailable)?
        .ok_or_else(unknown_mailbox)?;
    if !tokens_equal(&doc.secret_hash, &keys::hash(&body.secret)) {
        return Err(unknown_mailbox());
    }
    Ok((doc, body))
}

/// Replaces the FCM token and/or public key.
pub async fn update(
    State(state): State<Arc<AppState>>,
    body: Result<Json<OwnerBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (mut doc, body) = owned(&state, body).await?;
    let new_key = public_key(body.public_key.as_deref())?;
    let new_token = fcm_token(body.fcm_token.as_deref())?;
    if new_key.is_none() && new_token.is_none() {
        return Err(bad_request("Nothing to update."));
    }
    if let Some(key) = new_key {
        if key != doc.public_key {
            // A box for the old key can no longer be opened.
            doc.sealed = None;
            doc.sealed_at = None;
        }
        doc.public_key = key;
    }
    if let Some(token) = new_token {
        doc.fcm_token = token;
    }
    doc.updated_at = vtop_core::now_unix();
    state
        .store
        .put_mailbox(&doc)
        .await
        .map_err(store_unavailable)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

pub async fn delete(
    State(state): State<Arc<AppState>>,
    body: Result<Json<OwnerBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (doc, _) = owned(&state, body).await?;
    if let Some(owner) = &doc.owner_hash {
        forget_owner(&state, owner, &doc.id).await?;
    }
    state
        .store
        .delete_mailbox(&doc.id)
        .await
        .map_err(store_unavailable)?;
    tracing::info!("mail-otp: deleted a mailbox");
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Drops `owner`'s index entry if it still points at mailbox `id`.
async fn forget_owner(state: &AppState, owner: &str, id: &str) -> Result<(), ApiError> {
    let current = state
        .store
        .get_mailbox_owner(owner)
        .await
        .map_err(store_unavailable)?;
    if current.as_deref() == Some(id) {
        state
            .store
            .delete_mailbox_owner(owner)
            .await
            .map_err(store_unavailable)?;
    }
    Ok(())
}

/// Ties the mailbox to the student whose VTOP session comes with it, so a
/// new phone of theirs can take it back with `/reclaim`. The latest
/// mailbox tied to a student wins.
pub async fn claim_owner(
    State(state): State<Arc<AppState>>,
    body: Result<Json<OwnerBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (mut doc, body) = owned(&state, body).await?;
    let owner = session_owner(&state, body.session).await?;
    if let Some(previous) = doc.owner_hash.as_deref().filter(|hash| *hash != owner) {
        forget_owner(&state, previous, &doc.id).await?;
    }
    doc.owner_hash = Some(owner.clone());
    doc.updated_at = vtop_core::now_unix();
    state
        .store
        .put_mailbox(&doc)
        .await
        .map_err(store_unavailable)?;
    state
        .store
        .put_mailbox_owner(&owner, &doc.id)
        .await
        .map_err(store_unavailable)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReclaimBody {
    #[serde(default)]
    session: Option<SessionBody>,
    #[serde(default)]
    public_key: Option<String>,
    #[serde(default)]
    fcm_token: Option<String>,
}

/// Moves the caller's own address to this phone: new key, push token and
/// secret. A box sealed for the old key is dropped; the old phone's secret
/// stops working.
pub async fn reclaim(
    State(state): State<Arc<AppState>>,
    body: Result<Json<ReclaimBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(body) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    let public_key = public_key(body.public_key.as_deref())?
        .ok_or_else(|| bad_request("publicKey is required."))?;
    let fcm_token = fcm_token(body.fcm_token.as_deref())?
        .ok_or_else(|| bad_request("fcmToken is required."))?;
    let owner = session_owner(&state, body.session).await?;
    let id = state
        .store
        .get_mailbox_owner(&owner)
        .await
        .map_err(store_unavailable)?
        .ok_or_else(unknown_mailbox)?;
    let mut doc = state
        .store
        .get_mailbox(&id)
        .await
        .map_err(store_unavailable)?
        .filter(|doc| doc.owner_hash.as_deref() == Some(owner.as_str()))
        .ok_or_else(unknown_mailbox)?;
    let secret = keys::new_mailbox_secret();
    doc.secret_hash = keys::hash(&secret);
    doc.public_key = public_key;
    doc.fcm_token = fcm_token;
    doc.sealed = None;
    doc.sealed_at = None;
    doc.updated_at = vtop_core::now_unix();
    state
        .store
        .put_mailbox(&doc)
        .await
        .map_err(store_unavailable)?;
    tracing::info!("mail-otp: a mailbox moved to a new phone");
    let body = json!({
        "id": doc.id,
        "address": address(&doc.id, &state.config.mail_domain),
        "secret": secret,
    });
    Ok(Json(body).into_response())
}

/// The latest box if it is still fresh, for when the push did not arrive.
/// Not rate limited: the app polls it every couple of seconds while VTOP
/// waits for an OTP.
pub async fn inbox(
    State(state): State<Arc<AppState>>,
    body: Result<Json<OwnerBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    let (doc, _) = owned(&state, body).await?;
    let now = vtop_core::now_unix();
    let fresh = doc
        .sealed_at
        .is_some_and(|at| now.saturating_sub(at) <= SEALED_TTL_SECS);
    let body = match (doc.sealed, fresh) {
        (Some(sealed), true) => json!({ "sealed": sealed, "receivedAt": doc.sealed_at }),
        _ => json!({ "sealed": null }),
    };
    Ok(Json(body).into_response())
}

fn require_relay(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let Some(expected) = state.config.mail_relay_secret.as_deref() else {
        return Err(ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "relay_disabled",
            "The mail relay is not configured.",
        ));
    };
    let presented = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default()
        .trim();
    if !tokens_equal(&keys::hash(expected), &keys::hash(presented)) {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "A relay secret is required.",
        ));
    }
    Ok(())
}

async fn relay_mailbox(state: &AppState, id: &str) -> Result<MailboxDoc, ApiError> {
    if !is_id(id) {
        return Err(unknown_mailbox());
    }
    state
        .store
        .get_mailbox(id)
        .await
        .map_err(store_unavailable)?
        .ok_or_else(unknown_mailbox)
}

/// The public key the Worker seals to.
pub async fn relay_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    require_relay(&state, &headers)?;
    let doc = relay_mailbox(&state, &id).await?;
    Ok(Json(json!({ "publicKey": doc.public_key })).into_response())
}

#[derive(Deserialize)]
pub struct DeliverBody {
    #[serde(default)]
    sealed: String,
}

/// Keeps the box and pushes it to the phone. The push is best effort: the
/// app pulls from `/inbox` when it does not arrive.
pub async fn relay_deliver(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<DeliverBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    require_relay(&state, &headers)?;
    let Json(body) = body.map_err(|_| bad_request("Invalid JSON body."))?;
    let sealed = body.sealed.trim().to_string();
    let well_formed = !sealed.is_empty()
        && sealed.len() <= MAX_SEALED_LEN
        && sealed
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    if !well_formed {
        return Err(bad_request("sealed must be a base64url box."));
    }
    let mut doc = relay_mailbox(&state, &id).await?;
    let now = vtop_core::now_unix();
    doc.sealed = Some(sealed.clone());
    doc.sealed_at = Some(now);
    doc.updated_at = now;
    state
        .store
        .put_mailbox(&doc)
        .await
        .map_err(store_unavailable)?;
    let data = BTreeMap::from([
        ("type".to_string(), "mail_otp".to_string()),
        ("mailbox".to_string(), doc.id.clone()),
        ("sealed".to_string(), sealed),
        ("receivedAt".to_string(), now.to_string()),
    ]);
    let pushed = match state.phone.messenger.send(&doc.fcm_token, &data).await {
        Ok(()) => true,
        Err(error) => {
            if error == FcmError::Misconfigured {
                tracing::error!("mail-otp: FCM is misconfigured");
            }
            false
        }
    };
    Ok((StatusCode::ACCEPTED, Json(json!({ "pushed": pushed }))).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::fakes::{FakeLogin, FakeRefresher, FakeVerifier};
    use crate::fcm::FakeMessenger;
    use crate::store::MemoryStore;
    use crate::Parts;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    const RELAY: &str = "relay-secret-relay-secret-relay-secret";

    fn config() -> Config {
        let mut config = Config::from_lookup(|name| match name {
            "PUBLIC_BASE_URL" => Some("https://bridge.example".into()),
            "FIREBASE_CREDENTIALS_JSON" => Some("{}".into()),
            "VAULT_KEY" => Some("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into()),
            _ => None,
        })
        .unwrap();
        config.mail_relay_secret = Some(RELAY.into());
        config
    }

    fn app_with(config: Config) -> (axum::Router, Arc<AppState>, Arc<FakeMessenger>) {
        let messenger = Arc::new(FakeMessenger::default());
        let state = Arc::new(AppState::new(
            config,
            Parts {
                store: Arc::new(MemoryStore::default()),
                messenger: messenger.clone(),
                refresher: Arc::new(FakeRefresher::default()),
                login: Arc::new(FakeLogin::default()),
                verifier: Arc::new(FakeVerifier),
            },
        ));
        (crate::router(state.clone()), state, messenger)
    }

    fn app() -> (axum::Router, Arc<AppState>, Arc<FakeMessenger>) {
        app_with(config())
    }

    async fn call(
        app: &axum::Router,
        method: &str,
        uri: &str,
        auth: Option<&str>,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json");
        if let Some(auth) = auth {
            request = request.header("authorization", format!("Bearer {auth}"));
        }
        let body = if method == "GET" {
            Body::empty()
        } else {
            Body::from(body.to_string())
        };
        let response = app
            .clone()
            .oneshot(request.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    async fn registered(app: &axum::Router) -> (String, String, String) {
        let (_, public) = mail_seal::generate_keypair();
        let (status, body) = call(
            app,
            "POST",
            "/v1/mail-otp/register",
            None,
            json!({ "publicKey": public, "fcmToken": "device" }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let id = body["id"].as_str().unwrap().to_string();
        assert_eq!(body["address"], format!("vtop-otp-{id}@aster0.dev"));
        (id, body["secret"].as_str().unwrap().to_string(), public)
    }

    #[test]
    fn ids_use_the_alphabet() {
        for _ in 0..50 {
            assert!(is_id(&new_id()));
        }
        assert!(!is_id("ABCDEFGHJK"));
        assert!(!is_id("abc"));
    }

    #[tokio::test]
    async fn register_stores_only_the_secret_hash() {
        let (app, state, _) = app();
        let (id, secret, public) = registered(&app).await;
        let doc = state.store.get_mailbox(&id).await.unwrap().unwrap();
        assert_eq!(doc.public_key, public);
        assert_eq!(doc.secret_hash, keys::hash(&secret));
        assert!(secret.starts_with("vmb_"));
    }

    #[tokio::test]
    async fn register_rejects_bad_keys() {
        let (app, _, _) = app();
        let (status, _) = call(
            &app,
            "POST",
            "/v1/mail-otp/register",
            None,
            json!({ "publicKey": "short", "fcmToken": "device" }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn relay_needs_the_secret() {
        let (app, _, _) = app();
        let (id, _, _) = registered(&app).await;
        let uri = format!("/v1/mail-otp/relay/{id}");
        let (status, _) = call(&app, "GET", &uri, None, json!({})).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let (status, _) = call(&app, "GET", &uri, Some("wrong"), json!({})).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn deliver_pushes_and_inbox_returns_the_box() {
        let (app, _, messenger) = app();
        let (id, secret, public) = registered(&app).await;
        let uri = format!("/v1/mail-otp/relay/{id}");
        let (status, body) = call(&app, "GET", &uri, Some(RELAY), json!({})).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["publicKey"], public);

        let sealed = mail_seal::seal(&public, &id, b"{}").unwrap();
        let (status, body) =
            call(&app, "POST", &uri, Some(RELAY), json!({ "sealed": sealed })).await;
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(body["pushed"], true);

        let sends = messenger.sends();
        assert_eq!(sends.len(), 1);
        assert_eq!(sends[0].0, "device");
        assert_eq!(sends[0].1["type"], "mail_otp");
        assert_eq!(sends[0].1["sealed"], sealed);

        let (status, body) = call(
            &app,
            "POST",
            "/v1/mail-otp/inbox",
            None,
            json!({ "id": id, "secret": secret }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["sealed"], sealed);
    }

    #[tokio::test]
    async fn inbox_is_not_rate_limited() {
        let (app, _, _) = app();
        let (id, secret, _) = registered(&app).await;
        for _ in 0..200 {
            let (status, _) = call(
                &app,
                "POST",
                "/v1/mail-otp/inbox",
                None,
                json!({ "id": id, "secret": secret }),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn deliver_survives_a_dead_fcm_token() {
        let (app, state, messenger) = app();
        let (id, _, public) = registered(&app).await;
        *messenger.result.lock().unwrap() = Err(FcmError::TokenInvalid);
        let sealed = mail_seal::seal(&public, &id, b"{}").unwrap();
        let (status, body) = call(
            &app,
            "POST",
            &format!("/v1/mail-otp/relay/{id}"),
            Some(RELAY),
            json!({ "sealed": sealed }),
        )
        .await;
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(body["pushed"], false);
        let doc = state.store.get_mailbox(&id).await.unwrap().unwrap();
        assert_eq!(doc.sealed.as_deref(), Some(sealed.as_str()));
    }

    #[tokio::test]
    async fn stale_boxes_are_not_returned() {
        let (app, state, _) = app();
        let (id, secret, _) = registered(&app).await;
        let mut doc = state.store.get_mailbox(&id).await.unwrap().unwrap();
        doc.sealed = Some("AQ".into());
        doc.sealed_at = Some(vtop_core::now_unix() - SEALED_TTL_SECS - 1);
        state.store.put_mailbox(&doc).await.unwrap();
        let (_, body) = call(
            &app,
            "POST",
            "/v1/mail-otp/inbox",
            None,
            json!({ "id": id, "secret": secret }),
        )
        .await;
        assert_eq!(body["sealed"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn owner_calls_need_the_secret() {
        let (app, state, _) = app();
        let (id, secret, _) = registered(&app).await;
        let (status, _) = call(
            &app,
            "POST",
            "/v1/mail-otp/update",
            None,
            json!({ "id": id, "secret": "vmb_wrong", "fcmToken": "x" }),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let (status, _) = call(
            &app,
            "POST",
            "/v1/mail-otp/update",
            None,
            json!({ "id": id, "secret": secret, "fcmToken": "new-device" }),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        let doc = state.store.get_mailbox(&id).await.unwrap().unwrap();
        assert_eq!(doc.fcm_token, "new-device");

        let (status, _) = call(
            &app,
            "POST",
            "/v1/mail-otp/delete",
            None,
            json!({ "id": id, "secret": secret }),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(state.store.get_mailbox(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn relay_is_off_without_a_secret() {
        let mut config = config();
        config.mail_relay_secret = None;
        let (app, _, _) = app_with(config);
        let (status, _) = call(
            &app,
            "GET",
            "/v1/mail-otp/relay/abcdefghij",
            Some(RELAY),
            json!({}),
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }

    fn session(reg: &str) -> serde_json::Value {
        json!({ "cookies": format!("JSESSIONID=x; REG={reg}") })
    }

    async fn claim(app: &axum::Router, id: &str, secret: &str, reg: &str) -> StatusCode {
        call(
            app,
            "POST",
            "/v1/mail-otp/owner",
            None,
            json!({ "id": id, "secret": secret, "session": session(reg) }),
        )
        .await
        .0
    }

    async fn reclaim_as(app: &axum::Router, reg: &str) -> (StatusCode, serde_json::Value) {
        let (_, public) = mail_seal::generate_keypair();
        call(
            app,
            "POST",
            "/v1/mail-otp/reclaim",
            None,
            json!({ "session": session(reg), "publicKey": public, "fcmToken": "new-phone" }),
        )
        .await
    }

    #[tokio::test]
    async fn a_new_phone_takes_its_own_address_back() {
        let (app, state, _) = app();
        let (id, old_secret, old_public) = registered(&app).await;
        assert_eq!(
            claim(&app, &id, &old_secret, "24mic7076").await,
            StatusCode::NO_CONTENT
        );

        let (status, body) = reclaim_as(&app, "24MIC7076").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["id"], id.as_str());
        assert_eq!(body["address"], format!("vtop-otp-{id}@aster0.dev"));
        let new_secret = body["secret"].as_str().unwrap();
        assert_ne!(new_secret, old_secret);

        let doc = state.store.get_mailbox(&id).await.unwrap().unwrap();
        assert_ne!(doc.public_key, old_public);
        assert_eq!(doc.fcm_token, "new-phone");
        // Only a keyed hash of the registration number is stored.
        let owner = doc.owner_hash.unwrap();
        assert!(!owner.contains("24MIC7076") && !owner.contains("24mic7076"));

        // The old phone's secret no longer works; the new one does.
        let inbox = |secret: String| {
            let app = app.clone();
            let id = id.clone();
            async move {
                call(
                    &app,
                    "POST",
                    "/v1/mail-otp/inbox",
                    None,
                    json!({ "id": id, "secret": secret }),
                )
                .await
                .0
            }
        };
        assert_eq!(inbox(old_secret).await, StatusCode::NOT_FOUND);
        assert_eq!(inbox(new_secret.to_string()).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn nobody_else_can_reclaim_it() {
        let (app, _, _) = app();
        let (id, secret, _) = registered(&app).await;
        assert_eq!(
            claim(&app, &id, &secret, "24MIC7076").await,
            StatusCode::NO_CONTENT
        );
        assert_eq!(reclaim_as(&app, "24BCE0001").await.0, StatusCode::NOT_FOUND);
        // A session VTOP does not accept proves nothing.
        let (status, body) = call(
            &app,
            "POST",
            "/v1/mail-otp/reclaim",
            None,
            json!({
                "session": { "cookies": "JSESSIONID=x" },
                "publicKey": mail_seal::generate_keypair().1,
                "fcmToken": "t",
            }),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{body}");
    }

    #[tokio::test]
    async fn an_untagged_or_deleted_address_cannot_be_reclaimed() {
        let (app, _, _) = app();
        let (id, secret, _) = registered(&app).await;
        assert_eq!(reclaim_as(&app, "24MIC7076").await.0, StatusCode::NOT_FOUND);

        assert_eq!(
            claim(&app, &id, &secret, "24MIC7076").await,
            StatusCode::NO_CONTENT
        );
        let (status, _) = call(
            &app,
            "POST",
            "/v1/mail-otp/delete",
            None,
            json!({ "id": id, "secret": secret }),
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(reclaim_as(&app, "24MIC7076").await.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn claiming_needs_the_mailbox_secret() {
        let (app, _, _) = app();
        let (id, _, _) = registered(&app).await;
        assert_eq!(
            claim(&app, &id, "vmb_wrong", "24MIC7076").await,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn the_latest_address_wins() {
        let (app, _, _) = app();
        let (first, first_secret, _) = registered(&app).await;
        let (second, second_secret, _) = registered(&app).await;
        claim(&app, &first, &first_secret, "24MIC7076").await;
        claim(&app, &second, &second_secret, "24MIC7076").await;
        assert_eq!(reclaim_as(&app, "24MIC7076").await.1["id"], second.as_str());
    }
}
