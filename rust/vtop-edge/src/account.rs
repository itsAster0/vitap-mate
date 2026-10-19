//! Accounts on the edge: one `Account` object per student, named by the
//! registration number. It holds the linked phones' secrets, access keys,
//! settings, the sealed session cache and saved credentials, and the
//! session requests of agents and the extension.
//!
//! A session request is answered from the cache (checked with VTOP by the
//! engine), or left pending while the phone that created the key is asked
//! (an event on its install; every linked phone for keys from before keys
//! had one) and/or the engine signs in with saved credentials, the OTP read
//! from the student's forwarded mail. Server sign-in therefore needs email
//! forwarding: credentials are only kept for students who set it up.
//!
//! Routes:
//! - Key holders (`Authorization: Bearer vtm_…`): `GET /v1/whoami`,
//!   `POST /v1/session`, `GET /v1/session/requests/{id}`,
//!   `POST /v1/session/expire`.
//! - The phone's answer to a request: `POST /v1/session/answer?account=`.
//! - The app, with a live VTOP session: `/v1/link`, `/v1/keys`,
//!   `/v1/keys/revoke`, `/v1/account`, `/v1/account/{fcm-token,
//!   forget-credentials,delete,settings,semester,remove-phone}`. And with only its app
//!   secret: `/v1/account/credentials-rejected` (VTOP turned the password
//!   down on the phone, so the server must stop trying it too).
//! - The engine: `/internal/accounts/login-done`, `/internal/accounts/import`.

use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use vtop_shared::account::{
    account_id, is_account_id, key_id, key_label, new_app_secret, new_key, semester_id,
    AccountSettings, PHONE_UNREACHABLE,
};
use vtop_shared::cookies::{self, CookieEditorCookie};
use vtop_shared::engine::{
    LoginDone, LoginOutcome, LoginStart, SealVault, SealedVault, SessionCheck, SessionChecked,
    SessionStatus, LOGIN, SEAL, SESSION,
};
use vtop_shared::events::EventBody;
use vtop_shared::tokens::{self, KEY_PREFIX};
use worker::{
    durable_object, DurableObject, Env, Method, Request, Response, Result, SqlStorage,
    SqlStorageValue, State, Stub,
};

use crate::account_state::{
    plan, ready, AccountRecord, KeyRecord, PhoneRecord, Plan, RequestRecord, RequestStatus,
    Snapshot, REQUEST_KEEP_SECS,
};
use crate::sql;
use crate::state::member_label;
use crate::{
    bad_request, call, create_device, device_matches, device_stub, engine_post, error, now,
    owner_stub, set_device_fcm, var,
};

const PHONE_UNAVAILABLE: &str = "phone_unavailable: Could not reach the phone right now.";
const PHONE_SESSION_INVALID: &str = "phone_session_invalid: VTOP rejected your phone's session. Open VITAP Mate, refresh your login, and try again.";
const ACCOUNT_MISMATCH: &str =
    "account_mismatch: Your phone is signed in to VTOP as a different student.";

fn account_stub(env: &Env, id: &str) -> Result<Stub> {
    env.durable_object("ACCOUNT")?.get_by_name(id)
}

fn key_index(env: &Env, hash: &str) -> Result<Stub> {
    env.durable_object("INDEX")?
        .get_by_name(&format!("key:{hash}"))
}

/// The student's forwarding address (needed to sign in on the server), if
/// they have one.
async fn forwarding_mailbox(env: &Env, owner_hash: &str) -> Result<Option<String>> {
    if owner_hash.is_empty() {
        return Ok(None);
    }
    let mut response = call(&owner_stub(env, owner_hash)?, Method::Get, "", None).await?;
    if response.status_code() != 200 {
        return Ok(None);
    }
    Ok(Some(response.text().await?).filter(|id| !id.is_empty()))
}

async fn has_forwarding(env: &Env, owner_hash: &str) -> Result<bool> {
    Ok(forwarding_mailbox(env, owner_hash).await?.is_some())
}

/// A response with its own headers, so CORS can be added to it.
async fn relay(mut response: Response) -> Result<Response> {
    let status = response.status_code();
    let retry_after = response.headers().get("retry-after")?;
    let bytes = response.bytes().await?;
    // A 204 or 304 must have no body at all, not an empty one.
    if matches!(status, 204 | 304) {
        return Ok(Response::empty()?.with_status(status));
    }
    let mut out = Response::from_bytes(bytes)?.with_status(status);
    out.headers_mut().set("content-type", "application/json")?;
    if let Some(value) = retry_after {
        out.headers_mut().set("retry-after", &value)?;
    }
    Ok(out)
}

async fn to_account(env: &Env, id: &str, path: &str, body: Value) -> Result<Response> {
    let response = call(
        &account_stub(env, id)?,
        Method::Post,
        path,
        Some(body.to_string()),
    )
    .await?;
    relay(response).await
}

/// The routes this module answers at the edge.
pub fn handles(path: &str) -> bool {
    matches!(
        path,
        "/v1/whoami" | "/v1/session" | "/v1/link" | "/v1/keys" | "/v1/keys/revoke" | "/v1/account"
    ) || path.starts_with("/v1/session/")
        || path.starts_with("/v1/account/")
}

pub async fn handle(mut req: Request, env: &Env, path: &str) -> Result<Response> {
    match (req.method(), path) {
        (Method::Get, "/v1/whoami") => with_key(req, env, "/key/whoami", json!({})).await,
        (Method::Post, "/v1/session") => with_key(req, env, "/key/session", json!({})).await,
        (Method::Post, "/v1/session/expire") => with_key(req, env, "/key/expire", json!({})).await,
        (Method::Get, path) if path.starts_with("/v1/session/requests/") => {
            let id = path.trim_start_matches("/v1/session/requests/").to_string();
            with_key(req, env, "/key/request", json!({ "requestId": id })).await
        }
        (Method::Post, "/v1/session/answer") => {
            let account = req
                .url()?
                .query_pairs()
                .find(|(name, _)| name == "account")
                .map(|(_, value)| account_id(&value))
                .unwrap_or_default();
            if !is_account_id(&account) {
                return error(404, "request_not_found", "Unknown session request.");
            }
            let Ok(body) = req.json::<Value>().await else {
                return bad_request("Invalid JSON body.");
            };
            to_account(env, &account, "/answer", body).await
        }
        (Method::Post, "/v1/account/credentials-rejected") => {
            let Ok(body) = req.json::<Value>().await else {
                return bad_request("Invalid JSON body.");
            };
            let account = account_id(body["registrationNumber"].as_str().unwrap_or_default());
            if !is_account_id(&account) {
                return bad_request("registrationNumber is required.");
            }
            to_account(
                env,
                &account,
                "/app/credentials-rejected",
                json!({ "body": body }),
            )
            .await
        }
        (Method::Post, path) => {
            let op = match path {
                "/v1/link" => "link",
                "/v1/keys" => "create-key",
                "/v1/keys/revoke" => "revoke-key",
                "/v1/account" => "account",
                "/v1/account/fcm-token" => "fcm-token",
                "/v1/account/forget-credentials" => "forget-credentials",
                "/v1/account/delete" => "delete",
                "/v1/account/settings" => "settings",
                "/v1/account/semester" => "semester",
                "/v1/account/remove-phone" => "remove-phone",
                _ => return error(404, "not_found", "No such route."),
            };
            with_session(req, env, op).await
        }
        _ => error(405, "method_not_allowed", "Method not allowed."),
    }
}

/// Finds the account of the access key on the request.
async fn with_key(req: Request, env: &Env, path: &str, mut body: Value) -> Result<Response> {
    let presented = req
        .headers()
        .get("authorization")?
        .and_then(|value| {
            value
                .strip_prefix("Bearer ")
                .map(|key| key.trim().to_string())
        })
        .unwrap_or_default();
    let unknown = || error(401, "key_unknown", "Unknown or revoked access key.");
    if !presented.starts_with(KEY_PREFIX) {
        return unknown();
    }
    let hash = tokens::hash(&presented);
    let mut index = call(&key_index(env, &hash)?, Method::Get, "", None).await?;
    if index.status_code() != 200 {
        return unknown();
    }
    let account = index.text().await?;
    body["keyHash"] = json!(hash);
    to_account(env, &account, path, body).await
}

/// Asks the engine whose live VTOP session the app sent, then hands the
/// call to that student's account.
async fn with_session(mut req: Request, env: &Env, op: &str) -> Result<Response> {
    let Ok(body) = req.json::<Value>().await else {
        return bad_request("Invalid JSON body.");
    };
    let cookies = body["session"]["cookies"]
        .as_str()
        .map(str::trim)
        .unwrap_or_default();
    if cookies.is_empty() {
        return bad_request("session.cookies is required.");
    }
    let check = SessionCheck {
        cookies: Some(cookies.to_string()),
        ..SessionCheck::default()
    };
    let checked = match engine_json::<SessionChecked>(env, SESSION, &check).await {
        Ok(checked) => checked,
        Err(reason) => {
            worker::console_log!("account: session check failed: {reason}");
            return error(503, "vtop_unreachable", "VTOP is unreachable; try again.");
        }
    };
    let (Some(account), Some(owner_hash)) = (&checked.registration_number, &checked.owner_hash)
    else {
        return match checked.status {
            SessionStatus::Expired => error(
                401,
                "session_invalid",
                "VTOP does not accept this session; sign in again.",
            ),
            _ => error(503, "vtop_unreachable", "VTOP is unreachable; try again."),
        };
    };
    let mailbox_id = forwarding_mailbox(env, owner_hash).await?;
    let account = account.clone();
    to_account(
        env,
        &account,
        &format!("/app/{op}"),
        json!({
            "checked": checked,
            "forwarding": mailbox_id.is_some(),
            "mailbox_id": mailbox_id,
            "body": body,
        }),
    )
    .await
}

/// Posts to the engine and reads a JSON answer.
async fn engine_json<T: DeserializeOwned>(
    env: &Env,
    path: &str,
    body: &impl Serialize,
) -> std::result::Result<T, String> {
    let mut response = engine_post(env, path, body)
        .await
        .map_err(|e| e.to_string())?;
    if response.status_code() != 200 {
        return Err(format!("engine answered {}", response.status_code()));
    }
    response.json().await.map_err(|e| e.to_string())
}

// ---- the object -------------------------------------------------------------

#[durable_object]
pub struct Account {
    state: State,
    env: Env,
    ready: std::cell::Cell<bool>,
}

const ACCOUNT_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS account (
    id TEXT PRIMARY KEY,
    app_secret_hash TEXT,
    always_use_vault INTEGER NOT NULL,
    phone_wait_secs INTEGER NOT NULL,
    vault_ttl_secs INTEGER,
    semester_id TEXT,
    device_id TEXT,
    mailbox_id TEXT,
    sealed_vault TEXT,
    vault_saved_at INTEGER,
    sealed_session TEXT,
    session_saved_at INTEGER,
    session_source TEXT,
    credentials_rejected_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    window_start INTEGER NOT NULL,
    window_hits INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS keys (
    hash TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    last_used_at INTEGER,
    device_id TEXT
);
CREATE TABLE IF NOT EXISTS requests (
    id TEXT PRIMARY KEY,
    response_token TEXT NOT NULL,
    status TEXT NOT NULL,
    cookies TEXT,
    error TEXT,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    vault_due_at INTEGER,
    vault_started INTEGER NOT NULL,
    phone_asked INTEGER NOT NULL,
    phone_error TEXT,
    source TEXT,
    phones TEXT
);
CREATE TABLE IF NOT EXISTS pending (request_id TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS phones (
    device_id TEXT PRIMARY KEY,
    secret_hash TEXT NOT NULL,
    label TEXT,
    added_at INTEGER NOT NULL,
    seen_at INTEGER NOT NULL
)";

/// Made by objects created before several phones could link.
const PHONES_TABLE: &str = "CREATE TABLE IF NOT EXISTS phones (
    device_id TEXT PRIMARY KEY,
    secret_hash TEXT NOT NULL,
    label TEXT,
    added_at INTEGER NOT NULL,
    seen_at INTEGER NOT NULL
)";

fn account_from(
    row: &sql::Row,
    keys: Vec<KeyRecord>,
    phones: Vec<PhoneRecord>,
) -> Option<AccountRecord> {
    Some(AccountRecord {
        id: sql::text(row, "id")?,
        app_secret_hash: sql::text(row, "app_secret_hash"),
        settings: AccountSettings {
            always_use_vault: sql::flag(row, "always_use_vault"),
            phone_wait_secs: sql::int(row, "phone_wait_secs")?,
            vault_ttl_secs: sql::int(row, "vault_ttl_secs"),
        },
        semester_id: sql::text(row, "semester_id"),
        device_id: sql::text(row, "device_id"),
        phones,
        mailbox_id: sql::text(row, "mailbox_id"),
        sealed_vault: sql::text(row, "sealed_vault"),
        vault_saved_at: sql::int(row, "vault_saved_at"),
        sealed_session: sql::text(row, "sealed_session"),
        session_saved_at: sql::int(row, "session_saved_at"),
        session_source: sql::text(row, "session_source"),
        credentials_rejected_at: sql::int(row, "credentials_rejected_at"),
        keys,
        created_at: sql::int(row, "created_at")?,
        updated_at: sql::int(row, "updated_at")?,
        window: (
            sql::int(row, "window_start").unwrap_or(0),
            sql::int(row, "window_hits").unwrap_or(0) as u32,
        ),
        loaded: Snapshot::default(),
    })
}

fn key_from(row: &sql::Row) -> Option<KeyRecord> {
    Some(KeyRecord {
        hash: sql::text(row, "hash")?,
        label: sql::text(row, "label")?,
        created_at: sql::int(row, "created_at")?,
        last_used_at: sql::int(row, "last_used_at"),
        device_id: sql::text(row, "device_id"),
    })
}

fn phone_from(row: &sql::Row) -> Option<PhoneRecord> {
    Some(PhoneRecord {
        device_id: sql::text(row, "device_id")?,
        secret_hash: sql::text(row, "secret_hash")?,
        label: sql::text(row, "label"),
        added_at: sql::int(row, "added_at")?,
        seen_at: sql::int(row, "seen_at").unwrap_or(0),
    })
}

fn status_text(status: &RequestStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn request_from(row: &sql::Row) -> Option<RequestRecord> {
    Some(RequestRecord {
        id: sql::text(row, "id")?,
        response_token: sql::text(row, "response_token")?,
        status: serde_json::from_value(Value::String(sql::text(row, "status")?)).ok()?,
        cookies: sql::text(row, "cookies"),
        error: sql::text(row, "error"),
        created_at: sql::int(row, "created_at")?,
        expires_at: sql::int(row, "expires_at")?,
        vault_due_at: sql::int(row, "vault_due_at"),
        vault_started: sql::flag(row, "vault_started"),
        phone_asked: sql::flag(row, "phone_asked"),
        phone_error: sql::text(row, "phone_error"),
        source: sql::text(row, "source"),
        phones: sql::text(row, "phones")
            .map(|joined| {
                joined
                    .split(',')
                    .filter(|d| !d.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// The `account` row's columns besides `id`.
fn account_columns(doc: &AccountRecord) -> Vec<(&'static str, SqlStorageValue)> {
    vec![
        (
            "app_secret_hash",
            sql::v_opt_text(doc.app_secret_hash.as_deref()),
        ),
        (
            "always_use_vault",
            sql::v_flag(doc.settings.always_use_vault),
        ),
        ("phone_wait_secs", sql::v_int(doc.settings.phone_wait_secs)),
        (
            "vault_ttl_secs",
            sql::v_opt_int(doc.settings.vault_ttl_secs),
        ),
        ("semester_id", sql::v_opt_text(doc.semester_id.as_deref())),
        ("device_id", sql::v_opt_text(doc.device_id.as_deref())),
        ("mailbox_id", sql::v_opt_text(doc.mailbox_id.as_deref())),
        ("sealed_vault", sql::v_opt_text(doc.sealed_vault.as_deref())),
        ("vault_saved_at", sql::v_opt_int(doc.vault_saved_at)),
        (
            "sealed_session",
            sql::v_opt_text(doc.sealed_session.as_deref()),
        ),
        ("session_saved_at", sql::v_opt_int(doc.session_saved_at)),
        (
            "session_source",
            sql::v_opt_text(doc.session_source.as_deref()),
        ),
        (
            "credentials_rejected_at",
            sql::v_opt_int(doc.credentials_rejected_at),
        ),
        ("created_at", sql::v_int(doc.created_at)),
        ("updated_at", sql::v_int(doc.updated_at)),
        ("window_start", sql::v_int(doc.window.0)),
        ("window_hits", sql::v_int(u64::from(doc.window.1))),
    ]
}

fn put_key(db: &SqlStorage, key: &KeyRecord) -> Result<()> {
    sql::run(
        db,
        "INSERT OR REPLACE INTO keys (hash, label, created_at, last_used_at, device_id)
         VALUES (?, ?, ?, ?, ?)",
        vec![
            sql::v_text(&key.hash),
            sql::v_text(&key.label),
            sql::v_int(key.created_at),
            sql::v_opt_int(key.last_used_at),
            sql::v_opt_text(key.device_id.as_deref()),
        ],
    )
}

fn put_phone(db: &SqlStorage, phone: &PhoneRecord) -> Result<()> {
    sql::run(
        db,
        "INSERT OR REPLACE INTO phones (device_id, secret_hash, label, added_at, seen_at)
         VALUES (?, ?, ?, ?, ?)",
        vec![
            sql::v_text(&phone.device_id),
            sql::v_text(&phone.secret_hash),
            sql::v_opt_text(phone.label.as_deref()),
            sql::v_int(phone.added_at),
            sql::v_int(phone.seen_at),
        ],
    )
}

/// Replaces everything stored with `doc`.
fn write_account(db: &SqlStorage, doc: &AccountRecord) -> Result<()> {
    let columns = account_columns(doc);
    let names: Vec<&str> = columns.iter().map(|(name, _)| *name).collect();
    let mut values = vec![sql::v_text(&doc.id)];
    values.extend(columns.into_iter().map(|(_, value)| value));
    sql::run(db, "DELETE FROM account", vec![])?;
    sql::run(
        db,
        &format!(
            "INSERT INTO account (id, {}) VALUES (?{})",
            names.join(", "),
            ", ?".repeat(names.len())
        ),
        values,
    )?;
    sql::run(db, "DELETE FROM keys", vec![])?;
    for key in &doc.keys {
        put_key(db, key)?;
    }
    sql::run(db, "DELETE FROM phones", vec![])?;
    for phone in &doc.phones {
        put_phone(db, phone)?;
    }
    Ok(())
}

/// Writes only what changed since `base` was read, so requests running at
/// the same time (each waits on the network between reading and saving)
/// do not undo each other's changes.
fn write_changes(db: &SqlStorage, base: &AccountRecord, doc: &AccountRecord) -> Result<()> {
    let changed: Vec<(&str, SqlStorageValue)> = account_columns(doc)
        .into_iter()
        .zip(account_columns(base))
        .filter(|(now, before)| now.1 != before.1)
        .map(|(now, _)| now)
        .collect();
    if !changed.is_empty() {
        let set: Vec<String> = changed
            .iter()
            .map(|(name, _)| format!("{name} = ?"))
            .collect();
        let mut values: Vec<SqlStorageValue> =
            changed.into_iter().map(|(_, value)| value).collect();
        values.push(sql::v_text(&doc.id));
        sql::run(
            db,
            &format!("UPDATE account SET {} WHERE id = ?", set.join(", ")),
            values,
        )?;
    }
    for key in &base.keys {
        if !doc.keys.iter().any(|k| k.hash == key.hash) {
            sql::run(
                db,
                "DELETE FROM keys WHERE hash = ?",
                vec![sql::v_text(&key.hash)],
            )?;
        }
    }
    for key in &doc.keys {
        match base.keys.iter().find(|k| k.hash == key.hash) {
            None => put_key(db, key)?,
            // A key revoked meanwhile stays revoked: only update a row
            // that is still there.
            Some(before) if before != key => sql::run(
                db,
                "UPDATE keys SET label = ?, last_used_at = ?, device_id = ? WHERE hash = ?",
                vec![
                    sql::v_text(&key.label),
                    sql::v_opt_int(key.last_used_at),
                    sql::v_opt_text(key.device_id.as_deref()),
                    sql::v_text(&key.hash),
                ],
            )?,
            Some(_) => {}
        }
    }
    for phone in &base.phones {
        if !doc.phones.iter().any(|p| p.device_id == phone.device_id) {
            sql::run(
                db,
                "DELETE FROM phones WHERE device_id = ?",
                vec![sql::v_text(&phone.device_id)],
            )?;
        }
    }
    for phone in &doc.phones {
        if base.phones.iter().all(|p| p != phone) {
            put_phone(db, phone)?;
        }
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeyCall {
    key_hash: String,
    #[serde(default)]
    request_id: String,
}

/// No `Debug`: it holds a password.
#[derive(Deserialize, Default)]
struct Credentials {
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AppBody {
    #[serde(default)]
    fcm_token: Option<String>,
    #[serde(default)]
    device_id: Option<String>,
    #[serde(default)]
    device_secret: Option<String>,
    #[serde(default)]
    credentials: Option<Credentials>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    app_secret: Option<String>,
    #[serde(default)]
    settings: Option<Value>,
    #[serde(default)]
    semester_id: Option<String>,
}

#[derive(Deserialize)]
struct AppCall {
    #[serde(default)]
    checked: Option<SessionChecked>,
    #[serde(default)]
    forwarding: bool,
    #[serde(default)]
    mailbox_id: Option<String>,
    body: AppBody,
}

/// The phone's answer to a session request.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Answer {
    #[serde(default)]
    request_id: String,
    #[serde(default)]
    response_token: String,
    #[serde(default)]
    cookies: Vec<CookieEditorCookie>,
    #[serde(default)]
    error: String,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    fcm_token: Option<String>,
    /// The install answering, so its token can be updated.
    #[serde(default)]
    device_id: Option<String>,
    #[serde(default)]
    credentials: Option<Credentials>,
}

/// An account brought over from the bridge's store.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Import {
    record: AccountRecord,
    #[serde(default)]
    fcm_token: Option<String>,
    #[serde(default)]
    overwrite: bool,
}

fn new_request_id() -> String {
    let hex = tokens::random_token("");
    format!(
        "{}-{}-4{}-a{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..15],
        &hex[15..18],
        &hex[18..30]
    )
}

fn usable_text(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

impl Account {
    /// The tables; key-value data from before them is dropped (the
    /// student links again).
    async fn db(&self) -> Result<SqlStorage> {
        let db = self.state.storage().sql();
        if !self.ready.get() {
            if sql::needs_schema(&db)? {
                self.state.storage().delete_all().await?;
                sql::create(&db, ACCOUNT_SCHEMA)?;
            }
            // Columns added after the first accounts were made.
            sql::add_column(&db, "account", "session_source TEXT")?;
            sql::add_column(&db, "requests", "source TEXT")?;
            sql::add_column(&db, "keys", "device_id TEXT")?;
            sql::add_column(&db, "requests", "phones TEXT")?;
            sql::run(&db, PHONES_TABLE, vec![])?;
            self.ready.set(true);
        }
        Ok(db)
    }

    async fn doc(&self) -> Result<Option<AccountRecord>> {
        let db = self.db().await?;
        let Some(row) = sql::first(&db, "SELECT * FROM account LIMIT 1", vec![])? else {
            return Ok(None);
        };
        let keys = sql::rows(&db, "SELECT * FROM keys ORDER BY created_at", vec![])?
            .iter()
            .filter_map(key_from)
            .collect();
        let phones = sql::rows(&db, "SELECT * FROM phones ORDER BY added_at", vec![])?
            .iter()
            .filter_map(phone_from)
            .collect();
        let Some(mut doc) = account_from(&row, keys, phones) else {
            return Ok(None);
        };
        doc.mark_loaded();
        // Saved by whatever this request saves next.
        doc.adopt_legacy_phone();
        Ok(Some(doc))
    }

    async fn save(&self, doc: &mut AccountRecord) -> Result<()> {
        doc.updated_at = now();
        let db = self.db().await?;
        match doc.loaded.0.take() {
            Some(base) => write_changes(&db, &base, doc)?,
            None => write_account(&db, doc)?,
        }
        doc.mark_loaded();
        Ok(())
    }

    async fn request(&self, id: &str) -> Result<Option<RequestRecord>> {
        if id.is_empty() || id.len() > 64 {
            return Ok(None);
        }
        let db = self.db().await?;
        Ok(sql::first(
            &db,
            "SELECT * FROM requests WHERE id = ?",
            vec![sql::v_text(id)],
        )?
        .as_ref()
        .and_then(request_from))
    }

    async fn put_request(&self, request: &RequestRecord) -> Result<()> {
        sql::run(
            &self.db().await?,
            "INSERT OR REPLACE INTO requests (id, response_token, status, cookies, error,
                 created_at, expires_at, vault_due_at, vault_started, phone_asked, phone_error,
                 source, phones)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            vec![
                sql::v_text(&request.id),
                sql::v_text(&request.response_token),
                sql::v_text(&status_text(&request.status)),
                sql::v_opt_text(request.cookies.as_deref()),
                sql::v_opt_text(request.error.as_deref()),
                sql::v_int(request.created_at),
                sql::v_int(request.expires_at),
                sql::v_opt_int(request.vault_due_at),
                sql::v_flag(request.vault_started),
                sql::v_flag(request.phone_asked),
                sql::v_opt_text(request.phone_error.as_deref()),
                sql::v_opt_text(request.source.as_deref()),
                sql::v_text(&request.phones.join(",")),
            ],
        )
    }

    async fn requests(&self) -> Result<Vec<RequestRecord>> {
        let db = self.db().await?;
        Ok(sql::rows(&db, "SELECT * FROM requests", vec![])?
            .iter()
            .filter_map(request_from)
            .collect())
    }

    async fn delete_request(&self, id: &str) -> Result<()> {
        sql::run(
            &self.db().await?,
            "DELETE FROM requests WHERE id = ?",
            vec![sql::v_text(id)],
        )
    }

    async fn pending_id(&self) -> Result<Option<String>> {
        let db = self.db().await?;
        Ok(
            sql::first(&db, "SELECT request_id FROM pending LIMIT 1", vec![])?
                .and_then(|row| sql::text(&row, "request_id")),
        )
    }

    async fn set_pending(&self, id: &str) -> Result<()> {
        let db = self.db().await?;
        sql::run(&db, "DELETE FROM pending", vec![])?;
        sql::run(
            &db,
            "INSERT INTO pending (request_id) VALUES (?)",
            vec![sql::v_text(id)],
        )
    }

    // ---- key holders --------------------------------------------------------

    async fn key_doc(
        &self,
        call: &KeyCall,
    ) -> Result<std::result::Result<AccountRecord, Response>> {
        match self.doc().await? {
            Some(doc) if doc.has_key(&call.key_hash) => Ok(Ok(doc)),
            _ => Ok(Err(error(
                401,
                "key_unknown",
                "Unknown or revoked access key.",
            )?)),
        }
    }

    async fn whoami(&self, call: KeyCall) -> Result<Response> {
        let doc = match self.key_doc(&call).await? {
            Ok(doc) => doc,
            Err(response) => return Ok(response),
        };
        let key = doc.keys.iter().find(|key| key.hash == call.key_hash);
        let now = now();
        // Read-only: how a session for this key will be made, for clients
        // to show. Changing it stays in the app.
        Response::from_json(&json!({
            "registrationNumber": doc.id,
            "keyLabel": key.map(|key| key.label.clone()),
            "keyCreatedAt": key.map(|key| key.created_at),
            "keyLastUsedAt": key.and_then(|key| key.last_used_at),
            "semesterId": doc.semester_id,
            "settings": doc.settings,
            "phoneLinked": doc.linked(),
            "savedCredentials": doc.usable_vault(now).is_some(),
            "vaultSavedAt": doc.usable_vault(now).and(doc.vault_saved_at),
            "credentialsRejectedAt": doc.credentials_rejected_at,
        }))
    }

    async fn start_session(&self, call: KeyCall) -> Result<Response> {
        let mut doc = match self.key_doc(&call).await? {
            Ok(doc) => doc,
            Err(response) => return Ok(response),
        };
        let now = now();
        if !doc.allow(now) {
            self.save(&mut doc).await?;
            let mut response = error(429, "rate_limited", "Too many requests for this account.")?;
            response.headers_mut().set("retry-after", "60")?;
            return Ok(response);
        }
        doc.touch_key(&call.key_hash, now);

        if let Some(sealed) = doc.cached_session(now).map(str::to_string) {
            let check = SessionCheck {
                sealed_session: Some(sealed),
                account: Some(doc.id.clone()),
                ..SessionCheck::default()
            };
            match engine_json::<SessionChecked>(&self.env, SESSION, &check).await {
                Ok(checked) if checked.status != SessionStatus::Expired => {
                    if let Some(cookies) = checked.cookies {
                        self.save(&mut doc).await?;
                        return Response::from_json(&ready(&cookies, doc.cached_source()));
                    }
                }
                Ok(_) => {
                    worker::console_log!("account: cached session no longer works");
                    doc.clear_session();
                }
                Err(reason) => worker::console_log!("account: cannot check the cache: {reason}"),
            }
        }

        let pending_id = self.pending_id().await?;
        if let Some(id) = pending_id {
            if let Some(request) = self.request(&id).await?.filter(|r| r.pending(now)) {
                self.save(&mut doc).await?;
                return pending(&request);
            }
        }

        // Recorded before any network call, so a session call that comes
        // in meanwhile joins this request instead of starting another.
        let mut request = RequestRecord::new(new_request_id(), tokens::random_token(""), now);
        request.phones = doc
            .phones_for_key(&call.key_hash)
            .into_iter()
            .map(str::to_string)
            .collect();
        self.put_request(&request).await?;
        self.set_pending(&request.id).await?;
        let phones = request.phones.clone();
        self.begin(&mut doc, &mut request, &phones, now).await;
        self.save(&mut doc).await?;
        // The phone or the engine may have finished it already.
        if self
            .request(&request.id)
            .await?
            .is_some_and(|stored| stored.pending(now))
        {
            self.put_request(&request).await?;
        }
        self.reschedule(&doc).await?;
        pending(&request)
    }

    /// Asks the key's phones and/or starts a sign-in, in the order the
    /// settings choose.
    async fn begin(
        &self,
        doc: &mut AccountRecord,
        request: &mut RequestRecord,
        phones: &[String],
        now: u64,
    ) {
        match plan(&doc.settings, doc.usable_vault(now).is_some()) {
            Plan::VaultFirst => {
                if !self.start_login(doc, request, now).await {
                    self.ask_phone_or_fail(doc, request, phones, now).await;
                }
            }
            Plan::PhoneFirst { wait_secs } => match self.ask_phone(doc, request, phones, now).await
            {
                Ok(()) => request.vault_due_at = Some(now + wait_secs),
                Err(message) => {
                    request.phone_error = Some(message.to_string());
                    if !self.start_login(doc, request, now).await {
                        request.fail(message);
                    }
                }
            },
            Plan::PhoneOnly => self.ask_phone_or_fail(doc, request, phones, now).await,
        }
    }

    async fn ask_phone_or_fail(
        &self,
        doc: &AccountRecord,
        request: &mut RequestRecord,
        phones: &[String],
        now: u64,
    ) {
        if let Err(message) = self.ask_phone(doc, request, phones, now).await {
            request.fail(message);
        }
    }

    /// An event on each of `phones` (installs); the first to answer at
    /// `/v1/session/answer` wins.
    async fn ask_phone(
        &self,
        doc: &AccountRecord,
        request: &mut RequestRecord,
        phones: &[String],
        now: u64,
    ) -> std::result::Result<(), &'static str> {
        if phones.is_empty() {
            return Err(PHONE_UNREACHABLE);
        }
        let Ok(base) = var(&self.env, "PUBLIC_URL") else {
            return Err(PHONE_UNAVAILABLE);
        };
        let event = EventBody::CookieRequest {
            request_id: request.id.clone(),
            response_token: request.response_token.clone(),
            callback_url: format!("{base}/v1/session/answer?account={}", doc.id),
            want_credentials: if doc.want_credentials(now) { "1" } else { "0" }.to_string(),
        };
        let body = json!({ "body": event }).to_string();
        let mut failure = PHONE_UNREACHABLE;
        for device in phones {
            let appended = match device_stub(&self.env, device) {
                Ok(stub) => call(&stub, Method::Post, "/append", Some(body.clone())).await,
                Err(e) => Err(e),
            };
            match appended {
                Ok(response) if response.status_code() == 200 => request.phone_asked = true,
                Ok(_) => {}
                Err(e) => {
                    worker::console_log!("account: asking a phone failed: {e}");
                    failure = PHONE_UNAVAILABLE;
                }
            }
        }
        if request.phone_asked {
            Ok(())
        } else {
            Err(failure)
        }
    }

    /// Starts a sign-in with saved credentials; false when there are none
    /// or the engine cannot take it.
    async fn start_login(
        &self,
        doc: &AccountRecord,
        request: &mut RequestRecord,
        now: u64,
    ) -> bool {
        let Some(sealed_vault) = doc.usable_vault(now) else {
            return false;
        };
        let start = LoginStart {
            account: doc.id.clone(),
            sealed_vault: sealed_vault.to_string(),
            request_id: request.id.clone(),
        };
        match engine_post(&self.env, LOGIN, &start).await {
            Ok(response) if response.status_code() == 202 => {
                request.vault_started = true;
                true
            }
            Ok(response) => {
                worker::console_log!(
                    "account: engine refused a sign-in: {}",
                    response.status_code()
                );
                false
            }
            Err(e) => {
                worker::console_log!("account: engine unreachable for a sign-in: {e}");
                false
            }
        }
    }

    async fn poll(&self, call: KeyCall) -> Result<Response> {
        if let Err(response) = self.key_doc(&call).await? {
            return Ok(response);
        }
        let Some(request) = self.request(call.request_id.trim()).await? else {
            return error(404, "request_not_found", "Unknown session request.");
        };
        let (status, body) = request.poll_view(now());
        Ok(Response::from_json(&body)?.with_status(status))
    }

    async fn expire(&self, call: KeyCall) -> Result<Response> {
        let mut doc = match self.key_doc(&call).await? {
            Ok(doc) => doc,
            Err(response) => return Ok(response),
        };
        doc.clear_session();
        self.save(&mut doc).await?;
        Ok(Response::empty()?.with_status(204))
    }

    // ---- answers ------------------------------------------------------------

    /// The engine finished a sign-in with saved credentials.
    async fn login_done(&self, done: LoginDone) -> Result<Response> {
        let Some(mut doc) = self.doc().await? else {
            return error(404, "not_linked", "No such account.");
        };
        let now = now();
        let mut request = self.request(&done.request_id).await?;
        let fallback = match done.outcome {
            LoginOutcome::Ok {
                cookies,
                sealed_session,
            } => {
                doc.save_session(sealed_session, now, "server");
                if let Some(request) = request.as_mut().filter(|r| r.pending(now)) {
                    request.succeed(cookies, "server");
                }
                None
            }
            LoginOutcome::InvalidCredentials => {
                worker::console_log!("account: VTOP rejected saved credentials; dropping them");
                doc.reject_credentials(now);
                Some(Some("credentials_rejected: VTOP rejected the saved password. Sign in again in VITAP Mate."))
            }
            LoginOutcome::ForwardingRequired => {
                // Without forwarding the server cannot sign in, and each try
                // would mail the student an OTP for nothing.
                doc.clear_vault();
                Some(Some(
                    "forwarding_required: Set up OTP email forwarding in VITAP Mate to sign in from the server.",
                ))
            }
            LoginOutcome::VaultUnusable => {
                doc.clear_vault();
                Some(None)
            }
            LoginOutcome::Failed { reason } => {
                worker::console_log!("account: server sign-in failed for now: {reason}");
                Some(None)
            }
        };
        if let (Some(reason), Some(request)) = (fallback, request.as_mut()) {
            if request.pending(now) && !request.phone_asked {
                let phones = request.phones.clone();
                if let Err(message) = self.ask_phone(&doc, request, &phones, now).await {
                    request.fail(reason.unwrap_or(message));
                }
            } else if request.pending(now) && request.phone_error.is_some() {
                let message = request.phone_error.clone().unwrap_or_default();
                request.fail(reason.unwrap_or(&message));
            }
        }
        self.save(&mut doc).await?;
        if let Some(request) = &request {
            self.put_request(request).await?;
        }
        Ok(Response::empty()?.with_status(204))
    }

    async fn answer(&self, answer: Answer) -> Result<Response> {
        let not_found = || error(404, "request_not_found", "Unknown session request.");
        let Some(mut doc) = self.doc().await? else {
            return not_found();
        };
        let Some(mut request) = self.request(answer.request_id.trim()).await? else {
            return not_found();
        };
        if !tokens::hashes_equal(
            &tokens::hash(&request.response_token),
            &tokens::hash(answer.response_token.trim()),
        ) {
            return error(403, "token_invalid", "The response token does not match.");
        }
        let now = now();
        if !answer.error.trim().is_empty() {
            if request.pending(now) {
                request.fail(answer.error.trim());
                self.put_request(&request).await?;
            }
            return Response::from_json(&json!({ "ok": true }));
        }
        if cookies::validate(&answer.cookies).is_err() {
            return error(
                400,
                "COOKIES_INVALID",
                "The callback contained invalid cookies.",
            );
        }
        let header = cookies::to_header(&answer.cookies);
        let check = SessionCheck {
            cookies: Some(header.clone()),
            ..SessionCheck::default()
        };
        let checked = match engine_json::<SessionChecked>(&self.env, SESSION, &check).await {
            Ok(checked) => checked,
            Err(reason) => {
                worker::console_log!("account: checking a phone answer failed: {reason}");
                return error(503, "vtop_unreachable", "VTOP is unreachable; try again.");
            }
        };
        match checked.status {
            SessionStatus::Valid if checked.registration_number.as_deref() == Some(&doc.id) => {}
            SessionStatus::Valid => {
                if request.pending(now) {
                    request.fail(ACCOUNT_MISMATCH);
                    self.put_request(&request).await?;
                }
                return error(
                    403,
                    "account_mismatch",
                    "These cookies belong to a different student than the one who asked.",
                );
            }
            SessionStatus::Expired => {
                if request.pending(now) {
                    request.fail(PHONE_SESSION_INVALID);
                    self.put_request(&request).await?;
                }
                return error(
                    401,
                    "session_invalid",
                    "VTOP does not accept these cookies.",
                );
            }
            SessionStatus::Unknown => {
                return error(503, "vtop_unreachable", "VTOP is unreachable; try again.")
            }
        }

        // The answer is genuine: cache it even if the client gave up.
        let late = !request.pending(now);
        if !late {
            request.succeed(header, "phone");
            self.put_request(&request).await?;
        }
        if let Some(sealed) = checked.sealed_session.clone() {
            doc.save_session(sealed, now, "phone");
        }
        // Which phone answered is known only when it says so, or when
        // there is just one.
        let answered_by = match usable_text(answer.device_id.as_deref()) {
            Some(device) => doc.phones.iter().find(|p| p.device_id == device),
            None if doc.phones.len() == 1 => doc.phones.first(),
            None => None,
        };
        if let (Some(token), Some(phone)) = (usable_text(answer.fcm_token.as_deref()), answered_by)
        {
            set_device_fcm(&self.env, &phone.device_id, &token).await?;
        }
        if let Some(credentials) = answer.credentials {
            let username = usable_text(Some(&credentials.username))
                .or_else(|| usable_text(answer.username.as_deref()));
            if let Some(username) = username {
                let owner = checked.owner_hash.as_deref().unwrap_or_default();
                self.keep_credentials(&mut doc, owner, username, credentials.password, now)
                    .await?;
            }
        }
        self.save(&mut doc).await?;
        Response::from_json(&json!({ "ok": true, "late": late }))
    }

    /// Seals and keeps credentials, only for students with forwarding set
    /// up; true when kept.
    async fn keep_credentials(
        &self,
        doc: &mut AccountRecord,
        owner_hash: &str,
        username: String,
        password: String,
        now: u64,
    ) -> Result<bool> {
        if password.is_empty() || owner_hash.is_empty() {
            return Ok(false);
        }
        if !has_forwarding(&self.env, owner_hash).await? {
            worker::console_log!("account: credentials not kept: no forwarding address");
            return Ok(false);
        }
        let seal = SealVault {
            account: doc.id.clone(),
            username,
            password,
        };
        match engine_json::<SealedVault>(&self.env, SEAL, &seal).await {
            Ok(sealed) => {
                doc.save_vault(sealed.sealed_vault, now);
                Ok(true)
            }
            Err(reason) => {
                worker::console_log!("account: sealing credentials failed: {reason}");
                Ok(false)
            }
        }
    }

    // ---- the app ------------------------------------------------------------

    async fn app(&self, op: &str, call: AppCall) -> Result<Response> {
        let now = now();
        let existing = self.doc().await?;
        let body = call.body;

        if op == "credentials-rejected" {
            return match existing {
                Some(mut doc) if doc.phone_by_secret(body.app_secret.as_deref()).is_some() => {
                    doc.reject_credentials(now);
                    self.save(&mut doc).await?;
                    worker::console_log!("account: the phone reported rejected credentials");
                    Ok(Response::empty()?.with_status(204))
                }
                _ => app_secret_invalid(),
            };
        }

        let Some(checked) = call.checked else {
            return bad_request("session.cookies is required.");
        };
        let mut existing = existing;
        if let Some(doc) = existing
            .as_mut()
            .filter(|doc| doc.mailbox_id != call.mailbox_id)
        {
            doc.mailbox_id = call.mailbox_id.clone();
            self.save(doc).await?;
        }
        let id = account_id(checked.registration_number.as_deref().unwrap_or_default());
        let this_phone = existing
            .as_ref()
            .and_then(|doc| doc.phone_by_secret(body.app_secret.as_deref()));
        let linked = existing.as_ref().is_some_and(AccountRecord::linked);

        match op {
            "link" => {
                self.link(existing, id, checked, call.forwarding, body, now)
                    .await
            }
            "account" => {
                let mut doc = existing.unwrap_or_else(|| AccountRecord::new(&id, now));
                if let Some(me) = this_phone {
                    if doc.touch_phone(me, now) {
                        self.save(&mut doc).await?;
                    }
                }
                Response::from_json(&doc.view(this_phone, call.forwarding, now))
            }
            "forget-credentials" => {
                if let Some(mut doc) = existing {
                    doc.clear_vault();
                    self.save(&mut doc).await?;
                }
                Ok(Response::empty()?.with_status(204))
            }
            _ if !linked && op != "revoke-key" && op != "delete" => {
                error(409, "not_linked", "Link this account from the app first.")
            }
            _ => {
                let mut doc = existing.unwrap_or_else(|| AccountRecord::new(&id, now));
                // Revoking and deleting work before any phone is linked;
                // the rest need a linked phone's secret.
                match this_phone {
                    Some(me) => self.linked_op(op, &mut doc, Some(me), body, now).await,
                    None if !linked => self.linked_op(op, &mut doc, None, body, now).await,
                    None => app_secret_invalid(),
                }
            }
        }
    }

    /// Links this install, next to any phones linked already; linking it
    /// again hands it a new secret.
    async fn link(
        &self,
        existing: Option<AccountRecord>,
        id: String,
        checked: SessionChecked,
        forwarding: bool,
        body: AppBody,
        now: u64,
    ) -> Result<Response> {
        let me = existing
            .as_ref()
            .and_then(|doc| doc.phone_by_secret(body.app_secret.as_deref()));
        let semester = match body.semester_id.as_deref().map(semester_id) {
            Some(None) => return bad_request("semesterId is not a VTOP semester id."),
            Some(Some(semester)) => Some(semester),
            None => None,
        };
        let fcm = usable_text(body.fcm_token.as_deref());
        let mut doc = existing.unwrap_or_else(|| AccountRecord::new(&id, now));
        let previous = me.map(|i| doc.phones[i].clone());

        let device = match (&body.device_id, &body.device_secret) {
            (Some(device), Some(secret)) => {
                if !device_matches(&self.env, device, secret).await? {
                    return bad_request("Unknown device.");
                }
                if let Some(token) = &fcm {
                    set_device_fcm(&self.env, device, token).await?;
                }
                device.clone()
            }
            _ => {
                let Some(token) = fcm else {
                    return bad_request("fcmToken, or deviceId and deviceSecret, are required.");
                };
                match previous.as_ref().map(|p| p.device_id.clone()) {
                    Some(device) => {
                        set_device_fcm(&self.env, &device, &token).await?;
                        device
                    }
                    None => create_device(&self.env, token).await?.0,
                }
            }
        };

        // The install may have moved (a new one after a reinstall of the
        // push token): its old entry goes.
        if let Some(previous) = &previous {
            doc.phones.retain(|p| p.device_id != previous.device_id);
        }
        if let Some(previous) = previous.as_ref().filter(|p| p.device_id != device) {
            doc.phone_moved(&previous.device_id, &device);
        }
        let secret = new_app_secret();
        doc.add_phone(PhoneRecord {
            device_id: device,
            secret_hash: tokens::hash(&secret),
            label: member_label(body.label.as_deref())
                .or_else(|| previous.as_ref().and_then(|p| p.label.clone())),
            added_at: previous.as_ref().map_or(now, |p| p.added_at),
            seen_at: now,
        });
        if let Some(sealed) = checked.sealed_session {
            doc.save_session(sealed, now, "phone");
        }
        if let Some(semester) = semester {
            doc.semester_id = Some(semester);
        }
        let mut saved = false;
        if let Some(credentials) = body.credentials {
            if forwarding {
                if let Some(username) = usable_text(Some(&credentials.username)) {
                    let owner = checked.owner_hash.unwrap_or_default();
                    saved = self
                        .keep_credentials(&mut doc, &owner, username, credentials.password, now)
                        .await?;
                }
            }
        }
        self.save(&mut doc).await?;
        Response::from_json(&json!({
            "appSecret": secret,
            "savedCredentials": saved || doc.usable_vault(now).is_some(),
            "forwarding": forwarding,
        }))
    }

    /// `me`: the calling phone, or none while the account has no linked
    /// phone (only revoking and deleting get here then).
    async fn linked_op(
        &self,
        op: &str,
        doc: &mut AccountRecord,
        me: Option<usize>,
        body: AppBody,
        now: u64,
    ) -> Result<Response> {
        if me.is_none() && op != "revoke-key" && op != "delete" {
            return app_secret_invalid();
        }
        match op {
            "create-key" => {
                let key = new_key();
                let hash = tokens::hash(&key);
                // Only this phone is asked for the key's sessions.
                let phone = me.map(|i| doc.phones[i].device_id.clone());
                if !doc.add_key(hash.clone(), key_label(body.label.as_deref()), phone, now) {
                    return error(
                        409,
                        "key_limit",
                        "This account already has the most keys allowed; revoke one first.",
                    );
                }
                call(
                    &key_index(&self.env, &hash)?,
                    Method::Put,
                    "",
                    Some(doc.id.clone()),
                )
                .await?;
                self.save(doc).await?;
                let mcp_url = var(&self.env, "PUBLIC_MCP_URL")?;
                Ok(Response::from_json(
                    &json!({ "id": key_id(&hash), "key": key, "mcpUrl": mcp_url }),
                )?
                .with_status(201))
            }
            "revoke-key" => {
                let Some(hash) = doc.revoke_key(body.id.as_deref().unwrap_or_default()) else {
                    return error(404, "key_not_found", "No such key on this account.");
                };
                self.forget_key(&doc.id, &hash).await?;
                self.save(doc).await?;
                Ok(Response::empty()?.with_status(204))
            }
            "fcm-token" => {
                let Some(token) = usable_text(body.fcm_token.as_deref()) else {
                    return bad_request("fcmToken is required.");
                };
                let Some(me) = me else {
                    return app_secret_invalid();
                };
                let before = Some(doc.phones[me].device_id.clone());
                match (&body.device_id, &body.device_secret) {
                    (Some(device), Some(secret)) => {
                        if !device_matches(&self.env, device, secret).await? {
                            return bad_request("Unknown device.");
                        }
                        set_device_fcm(&self.env, device, &token).await?;
                        doc.phones[me].device_id = device.clone();
                    }
                    _ => {
                        let device = doc.phones[me].device_id.clone();
                        // The install is gone: make one for this token, so
                        // the phone can still be asked.
                        if !set_device_fcm(&self.env, &device, &token).await? {
                            doc.phones[me].device_id = create_device(&self.env, token).await?.0;
                        }
                    }
                }
                let moved = doc.phones[me].device_id.clone();
                if let Some(from) = before.filter(|from| *from != moved) {
                    doc.phone_moved(&from, &moved);
                }
                // Another entry for the install it moved to is this phone.
                let mut seen = false;
                doc.phones.retain(|p| {
                    let keep = p.device_id != moved || !seen;
                    seen |= p.device_id == moved;
                    keep
                });
                self.save(doc).await?;
                Ok(Response::empty()?.with_status(204))
            }
            "remove-phone" => {
                if doc
                    .remove_phone(body.id.as_deref().unwrap_or_default())
                    .is_none()
                {
                    return error(404, "phone_not_found", "No such phone on this account.");
                }
                self.save(doc).await?;
                Ok(Response::empty()?.with_status(204))
            }
            "settings" => {
                let Some(settings) = body
                    .settings
                    .and_then(|raw| serde_json::from_value::<AccountSettings>(raw).ok())
                    .filter(AccountSettings::is_valid)
                else {
                    return bad_request("settings must use the offered choices.");
                };
                doc.settings = settings;
                self.save(doc).await?;
                Ok(Response::empty()?.with_status(204))
            }
            "semester" => {
                let Some(semester) = body.semester_id.as_deref().and_then(semester_id) else {
                    return bad_request("semesterId is required.");
                };
                doc.semester_id = Some(semester);
                self.save(doc).await?;
                Ok(Response::empty()?.with_status(204))
            }
            "delete" => {
                self.drop_keys(doc).await?;
                self.state.storage().delete_all().await?;
                self.state.storage().delete_alarm().await?;
                self.ready.set(false);
                worker::console_log!("account: deleted at the user's request");
                Ok(Response::empty()?.with_status(204))
            }
            _ => error(404, "not_found", "No such route."),
        }
    }

    async fn forget_key(&self, id: &str, hash: &str) -> Result<()> {
        call(
            &key_index(&self.env, hash)?,
            Method::Delete,
            "",
            Some(id.to_string()),
        )
        .await?;
        Ok(())
    }

    async fn drop_keys(&self, doc: &mut AccountRecord) -> Result<()> {
        for key in std::mem::take(&mut doc.keys) {
            self.forget_key(&doc.id, &key.hash).await?;
        }
        Ok(())
    }

    async fn import(&self, import: Import) -> Result<Response> {
        if self.doc().await?.is_some() && !import.overwrite {
            return error(409, "exists", "Account exists.");
        }
        let mut doc = import.record;
        if !is_account_id(&doc.id) {
            return bad_request("record.id is not an account id.");
        }
        if let Some(token) = usable_text(import.fcm_token.as_deref()) {
            doc.device_id = Some(create_device(&self.env, token).await?.0);
        }
        for key in &doc.keys {
            call(
                &key_index(&self.env, &key.hash)?,
                Method::Put,
                "",
                Some(doc.id.clone()),
            )
            .await?;
        }
        write_account(&self.db().await?, &doc)?;
        Ok(Response::empty()?.with_status(201))
    }

    // ---- timers -------------------------------------------------------------

    /// The next thing due: a phone-first wait running out, a finished
    /// request to drop, saved credentials expiring.
    async fn reschedule(&self, doc: &AccountRecord) -> Result<()> {
        let mut next: Option<u64> = None;
        let mut due = |at: u64| next = Some(next.map_or(at, |n: u64| n.min(at)));
        for request in self.requests().await? {
            if let (Some(at), false) = (request.vault_due_at, request.vault_started) {
                if request.status == crate::account_state::RequestStatus::Pending {
                    due(at);
                }
            }
            due(request.created_at + REQUEST_KEEP_SECS);
        }
        if let (Some(ttl), Some(at), Some(_)) = (
            doc.settings.vault_ttl_secs,
            doc.vault_saved_at,
            &doc.sealed_vault,
        ) {
            due(at + ttl);
        }
        match next {
            Some(at) => {
                let wait = at.saturating_sub(now()).max(1);
                self.state
                    .storage()
                    .set_alarm(Duration::from_secs(wait))
                    .await
            }
            None => self.state.storage().delete_alarm().await,
        }
    }
}

fn pending(request: &RequestRecord) -> Result<Response> {
    let mut response = Response::from_json(&request.started_view())?.with_status(202);
    response.headers_mut().set("retry-after", "2")?;
    Ok(response)
}

fn app_secret_invalid() -> Result<Response> {
    error(
        403,
        "app_secret_invalid",
        "This phone is not linked to the account; link it from the app.",
    )
}

impl DurableObject for Account {
    fn new(state: State, env: Env) -> Self {
        Self {
            state,
            env,
            ready: std::cell::Cell::new(false),
        }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        let path = req.path();
        let Ok(body) = req.json::<Value>().await else {
            return bad_request("Invalid JSON body.");
        };
        fn parse<T: DeserializeOwned>(body: Value) -> std::result::Result<T, Result<Response>> {
            serde_json::from_value(body).map_err(|_| bad_request("Invalid JSON body."))
        }
        macro_rules! parsed {
            ($body:expr) => {
                match parse($body) {
                    Ok(value) => value,
                    Err(response) => return response,
                }
            };
        }
        match path.as_str() {
            "/key/whoami" => self.whoami(parsed!(body)).await,
            "/key/session" => self.start_session(parsed!(body)).await,
            "/key/request" => self.poll(parsed!(body)).await,
            "/key/expire" => self.expire(parsed!(body)).await,
            "/answer" => self.answer(parsed!(body)).await,
            "/login-done" => self.login_done(parsed!(body)).await,
            "/import" => self.import(parsed!(body)).await,
            path if path.starts_with("/app/") => {
                let op = path.trim_start_matches("/app/").to_string();
                self.app(&op, parsed!(body)).await
            }
            _ => error(404, "not_found", "No such route."),
        }
    }

    async fn alarm(&self) -> Result<Response> {
        let now = now();
        let Some(mut doc) = self.doc().await? else {
            return Response::empty();
        };
        for mut request in self.requests().await? {
            if now >= request.created_at + REQUEST_KEEP_SECS {
                self.delete_request(&request.id).await?;
                continue;
            }
            let due = request.vault_due_at.is_some_and(|at| now >= at);
            if due && !request.vault_started && request.pending(now) {
                // The phone stayed silent: sign in with saved credentials.
                if !self.start_login(&doc, &mut request, now).await {
                    request.vault_due_at = None;
                }
                self.put_request(&request).await?;
            }
        }
        if doc.sealed_vault.is_some() && doc.usable_vault(now).is_none() {
            doc.clear_vault();
            self.save(&mut doc).await?;
        }
        self.reschedule(&doc).await?;
        Response::empty()
    }
}

/// `POST /internal/accounts/{login-done,import}`, from the engine.
pub async fn internal(mut req: Request, env: &Env, path: &str) -> Result<Response> {
    if !crate::is_engine(&req, env) {
        return error(401, "unauthorized", "The engine secret is required.");
    }
    let Ok(body) = req.json::<Value>().await else {
        return bad_request("Invalid JSON body.");
    };
    match path {
        "/internal/accounts/login-done" => {
            let account = account_id(body["account"].as_str().unwrap_or_default());
            if !is_account_id(&account) {
                return bad_request("account is required.");
            }
            to_account(env, &account, "/login-done", body).await
        }
        "/internal/accounts/import" => {
            let account = account_id(body["record"]["id"].as_str().unwrap_or_default());
            if !is_account_id(&account) {
                return bad_request("record.id is required.");
            }
            to_account(env, &account, "/import", body).await
        }
        _ => error(404, "not_found", "No such route."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_routes() {
        for path in [
            "/v1/whoami",
            "/v1/session",
            "/v1/session/requests/abc",
            "/v1/session/answer",
            "/v1/link",
            "/v1/keys",
            "/v1/account",
            "/v1/account/credentials-rejected",
        ] {
            assert!(handles(path), "{path}");
        }
        for path in [
            "/cookie",
            "/cookie/callback",
            "/v1/mail-otp/register",
            "/mcp",
            "/healthz",
        ] {
            assert!(!handles(path), "{path}");
        }
    }

    #[test]
    fn request_ids_look_like_uuids() {
        let id = new_request_id();
        assert_eq!(id.len(), 36);
        assert_eq!(id.matches('-').count(), 4);
        assert_ne!(id, new_request_id());
    }
}
