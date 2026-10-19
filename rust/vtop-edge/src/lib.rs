//! The Cloudflare edge. One Rust Worker with:
//!
//! - `email`: the aster0.dev catch-all. Mail to `vtop-otp-<id>@…` is parsed
//!   in memory, the code sealed to the phone's key, and the box handed to
//!   that install's event log. The edge never keeps anything readable.
//! - `fetch`: app routes for mailboxes (`/v1/mail-otp/*`), installs
//!   (`/v1/devices/*`) and the event channel (`GET /v1/events`, a WebSocket
//!   or a catch-up read). `/mcp` goes on to the engine (`MCP_URL`), which serves the MCP tools,
//!   so clients only ever talk to this host. The legacy `/cookie*` routes
//!   are not served here; they stay on the old bridge.
//! - Accounts, access keys and VTOP sessions for the extension and agents
//!   (`/v1/link`, `/v1/keys*`, `/v1/account*`, `/v1/whoami`, `/v1/session*`),
//!   see [`account`]. The engine does the VTOP work for them.
//! - `/internal/accounts/*`, `/internal/mail-otp/await`: the engine reports
//!   sign-ins and waits for forwarded OTPs (sealed to its own key).
//!
//! State lives in Durable Objects: `Device` (per install), `Mailbox` (per
//! address), `Index` (owner hash → mailbox, key hash → account), `Account`
//! (per student) and `FcmToken` (one, caches the OAuth token), each in its
//! own SQLite tables (see [`sql`]). Whose VTOP
//! session a request carries is asked of the engine, which holds the key
//! the owner hash is made with.

mod account;
pub mod account_state;
mod device;
mod engine;
mod fcm;
pub mod mail;
mod sql;
pub mod state;
mod store;

use serde::Deserialize;
use serde_json::json;
use vtop_shared::events::EventBody;
use vtop_shared::mail_seal;
use vtop_shared::mailbox::{address, id_from_address, is_id, new_id};
use vtop_shared::tokens::{self, DEVICE_SECRET_PREFIX, MAILBOX_SECRET_PREFIX};
use worker::{
    event, Context, Env, ForwardableEmailMessage, Headers, Method, Request, RequestInit, Response,
    Result, Stub,
};

pub use account::Account;
pub use device::Device;
pub use engine::Engine;
pub use fcm::FcmToken;
use state::{fcm_token, is_device_id, member_label, public_key, MailboxDoc, MailboxMember};
pub use store::{Index, Mailbox};

pub fn now() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}

/// `{"code", "error"}`, as the bridge answers.
pub fn error(status: u16, code: &str, message: &str) -> Result<Response> {
    Ok(Response::from_json(&json!({ "code": code, "error": message }))?.with_status(status))
}

pub(crate) fn bad_request(message: &str) -> Result<Response> {
    error(400, "bad_request", message)
}

fn unknown_mailbox() -> Result<Response> {
    error(404, "mailbox_not_found", "No such forwarding address.")
}

/// Calls a Durable Object on an internal path.
pub(crate) async fn call(
    stub: &Stub,
    method: Method,
    path: &str,
    body: Option<String>,
) -> Result<Response> {
    let mut init = RequestInit::new();
    init.with_method(method);
    if let Some(body) = body {
        let headers = Headers::new();
        headers.set("content-type", "application/json")?;
        init.with_headers(headers).with_body(Some(body.into()));
    }
    stub.fetch_with_request(Request::new_with_init(&format!("https://do{path}"), &init)?)
        .await
}

pub(crate) fn device_stub(env: &Env, id: &str) -> Result<Stub> {
    env.durable_object("DEVICE")?.get_by_name(id)
}

fn mailbox_stub(env: &Env, id: &str) -> Result<Stub> {
    env.durable_object("MAILBOX")?.get_by_name(id)
}

pub(crate) fn owner_stub(env: &Env, owner_hash: &str) -> Result<Stub> {
    env.durable_object("INDEX")?
        .get_by_name(&format!("owner:{owner_hash}"))
}

async fn load_mailbox(env: &Env, id: &str) -> Result<Option<MailboxDoc>> {
    if !is_id(id) {
        return Ok(None);
    }
    let mut response = call(&mailbox_stub(env, id)?, Method::Get, "/doc", None).await?;
    match response.status_code() {
        200 => Ok(Some(response.json().await?)),
        404 => Ok(None),
        status => Err(format!("mailbox read failed: {status}").into()),
    }
}

async fn save_mailbox(env: &Env, doc: &MailboxDoc) -> Result<()> {
    let response = call(
        &mailbox_stub(env, &doc.id)?,
        Method::Put,
        "/doc",
        Some(serde_json::to_string(doc)?),
    )
    .await?;
    if response.status_code() != 204 {
        return Err(format!("mailbox write failed: {}", response.status_code()).into());
    }
    Ok(())
}

/// A required `[vars]` entry; every address the edge uses is configured.
pub fn var(env: &Env, name: &str) -> Result<String> {
    let value = env.var(name)?.to_string();
    if value.trim().is_empty() {
        return Err(format!("{name} is not configured").into());
    }
    Ok(value.trim().trim_end_matches('/').to_string())
}

// ---- installs ---------------------------------------------------------------

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct DeviceRef {
    #[serde(default)]
    device_id: Option<String>,
    #[serde(default)]
    device_secret: Option<String>,
}

pub(crate) async fn create_device(env: &Env, fcm: String) -> Result<(String, String)> {
    let id = tokens::random_token("")[..32].to_string();
    let secret = tokens::random_token(DEVICE_SECRET_PREFIX);
    let body = json!({ "id": id, "secret_hash": tokens::hash(&secret), "fcm_token": fcm });
    let response = call(
        &device_stub(env, &id)?,
        Method::Post,
        "/init",
        Some(body.to_string()),
    )
    .await?;
    if response.status_code() != 201 {
        return Err(format!("device create failed: {}", response.status_code()).into());
    }
    Ok((id, secret))
}

pub(crate) async fn device_matches(env: &Env, id: &str, secret: &str) -> Result<bool> {
    if !is_device_id(id) {
        return Ok(false);
    }
    let body = json!({ "secret_hash": tokens::hash(secret) });
    let response = call(
        &device_stub(env, id)?,
        Method::Post,
        "/auth",
        Some(body.to_string()),
    )
    .await?;
    Ok(response.status_code() == 204)
}

/// False when the install no longer exists.
pub(crate) async fn set_device_fcm(env: &Env, id: &str, token: &str) -> Result<bool> {
    let body = json!({ "fcm_token": token });
    let response = call(
        &device_stub(env, id)?,
        Method::Post,
        "/fcm",
        Some(body.to_string()),
    )
    .await?;
    Ok(response.status_code() == 204)
}

/// The install a mailbox call names: a registered device, or (older app
/// builds) a new one made for the FCM token sent.
async fn resolve_device(
    env: &Env,
    device: &DeviceRef,
    fcm: Option<String>,
) -> Result<std::result::Result<String, &'static str>> {
    if let (Some(id), Some(secret)) = (&device.device_id, &device.device_secret) {
        if !device_matches(env, id, secret).await? {
            return Ok(Err("Unknown device."));
        }
        if let Some(token) = fcm {
            set_device_fcm(env, id, &token).await?;
        }
        return Ok(Ok(id.clone()));
    }
    match fcm {
        Some(token) => Ok(Ok(create_device(env, token).await?.0)),
        None => Ok(Err("deviceId and deviceSecret, or fcmToken, are required.")),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceBody {
    #[serde(default)]
    id: String,
    #[serde(default)]
    secret: String,
    #[serde(default)]
    fcm_token: Option<String>,
}

async fn device_register(mut req: Request, env: &Env) -> Result<Response> {
    let Ok(body) = req.json::<DeviceBody>().await else {
        return bad_request("Invalid JSON body.");
    };
    let token = match fcm_token(body.fcm_token.as_deref()) {
        Ok(Some(token)) => token,
        Ok(None) => return bad_request("fcmToken is required."),
        Err(message) => return bad_request(message),
    };
    let (id, secret) = create_device(env, token).await?;
    Ok(Response::from_json(&json!({ "id": id, "secret": secret }))?.with_status(201))
}

async fn device_update(mut req: Request, env: &Env) -> Result<Response> {
    let Ok(body) = req.json::<DeviceBody>().await else {
        return bad_request("Invalid JSON body.");
    };
    if !device_matches(env, &body.id, &body.secret).await? {
        return error(404, "unknown_device", "This install is not registered.");
    }
    match fcm_token(body.fcm_token.as_deref()) {
        Ok(Some(token)) => {
            set_device_fcm(env, &body.id, &token).await?;
        }
        Ok(None) => return bad_request("Nothing to update."),
        Err(message) => return bad_request(message),
    }
    Ok(Response::empty()?.with_status(204))
}

#[derive(Deserialize)]
struct EventsQuery {
    #[serde(default)]
    device: String,
}

/// Hands the request to the install's object, which checks the secret and
/// answers with a socket or a catch-up page.
async fn events(req: Request, env: &Env) -> Result<Response> {
    let id = req
        .query::<EventsQuery>()
        .map(|q| q.device)
        .unwrap_or_default();
    if !is_device_id(&id) {
        return error(404, "unknown_device", "This install is not registered.");
    }
    device_stub(env, &id)?.fetch_with_request(req).await
}

// ---- mailboxes --------------------------------------------------------------

#[derive(Deserialize)]
struct SessionBody {
    #[serde(default)]
    cookies: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MailBody {
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
    /// What the app calls this install, for the device list.
    #[serde(default)]
    label: Option<String>,
    /// `delete`: the address itself, for every install.
    #[serde(default)]
    everywhere: bool,
    /// `devices/remove`: the install to take off the address.
    #[serde(default)]
    remove_device: Option<String>,
    #[serde(flatten)]
    device: DeviceRef,
}

async fn mail_body(req: &mut Request) -> std::result::Result<MailBody, Result<Response>> {
    req.json::<MailBody>()
        .await
        .map_err(|_| bad_request("Invalid JSON body."))
}

/// The mailbox the body's id and secret open, and which of its installs
/// the secret is. Records the visit, so quiet installs can be told apart.
async fn owned(env: &Env, body: &MailBody) -> Result<Option<(MailboxDoc, usize)>> {
    if body.secret.is_empty() {
        return Ok(None);
    }
    let Some(mut doc) = load_mailbox(env, &body.id).await? else {
        return Ok(None);
    };
    let Some(index) = doc.member_by_secret(&tokens::hash(&body.secret)) else {
        return Ok(None);
    };
    if doc.touch(index, now()) {
        save_mailbox(env, &doc).await?;
    }
    Ok(Some((doc, index)))
}

fn new_member(
    device_id: String,
    secret: &str,
    key: String,
    label: Option<&str>,
    now: u64,
) -> MailboxMember {
    MailboxMember {
        device_id,
        secret_hash: tokens::hash(secret),
        public_key: key,
        label: member_label(label),
        sealed: None,
        sealed_at: None,
        added_at: now,
        seen_at: now,
    }
}

/// Posts to the engine with the shared secret.
pub(crate) async fn engine_post(
    env: &Env,
    path: &str,
    body: &impl serde::Serialize,
) -> Result<Response> {
    let headers = Headers::new();
    headers.set("content-type", "application/json")?;
    headers.set(
        "authorization",
        &format!("Bearer {}", env.secret("BRIDGE_SECRET")?),
    )?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post)
        .with_headers(headers)
        .with_body(Some(serde_json::to_string(body)?.into()));
    engine_send(env, "BRIDGE_URL", path, &init).await
}

/// Sends a request to the engine: to the container (`IN_CONTAINER`) or to
/// the address in `service`.
async fn engine_send(env: &Env, service: &str, path: &str, init: &RequestInit) -> Result<Response> {
    let base = var(env, service)?;
    if base == engine::IN_CONTAINER {
        let request = Request::new_with_init(&format!("http://engine{path}"), init)?;
        return env
            .durable_object("ENGINE")?
            .get_by_name("engine")?
            .fetch_with_request(request)
            .await;
    }
    let request = Request::new_with_init(&format!("{base}{path}"), init)?;
    worker::Fetch::Request(request).send().await
}

/// `POST /internal/mail-otp/await`, from the engine: the next OTP of a
/// student, sealed to the key of the engine's sign-in.
async fn mail_await(mut req: Request, env: &Env) -> Result<Response> {
    if !is_engine(&req, env) {
        return error(401, "unauthorized", "The engine secret is required.");
    }
    let Ok(body) = req.json::<vtop_shared::engine::OtpAwait>().await else {
        return bad_request("Invalid JSON body.");
    };
    if !mail_seal::is_public_key(&body.public_key) {
        return bad_request("publicKey must be a base64url X25519 key.");
    }
    let mut index = call(&owner_stub(env, &body.owner_hash)?, Method::Get, "", None).await?;
    if index.status_code() != 200 {
        return unknown_mailbox();
    }
    let id = index.text().await?;
    let forwarded = json!({
        "publicKey": body.public_key,
        "ownerHash": body.owner_hash,
        "since": body.since,
        "waitSecs": body.wait_secs,
    });
    call(
        &mailbox_stub(env, &id)?,
        Method::Post,
        "/await",
        Some(forwarded.to_string()),
    )
    .await
}

/// CORS as the bridge sends it: every origin, or the listed ones.
fn with_cors(mut response: Response, origin: Option<&str>, env: &Env) -> Result<Response> {
    let allowed = env
        .var("ALLOWED_ORIGINS")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let allowed: Vec<&str> = allowed
        .split(',')
        .map(str::trim)
        .filter(|o| !o.is_empty())
        .collect();
    let headers = response.headers_mut();
    if allowed.is_empty() {
        headers.set("access-control-allow-origin", "*")?;
    } else if let Some(origin) = origin.filter(|origin| allowed.contains(origin)) {
        headers.set("access-control-allow-origin", origin)?;
        headers.set("vary", "Origin")?;
    }
    headers.set("access-control-allow-methods", "GET, POST, DELETE, OPTIONS")?;
    headers.set("access-control-allow-headers", "Content-Type")?;
    Ok(response)
}

/// Whose VTOP session this is, as the bridge's owner hash.
async fn session_owner(
    env: &Env,
    session: Option<&SessionBody>,
) -> Result<std::result::Result<String, Response>> {
    let Some(cookies) = session.map(|s| s.cookies.trim()).filter(|c| !c.is_empty()) else {
        return Ok(Err(bad_request("session.cookies is required.")?));
    };
    let mut response = engine_post(
        env,
        "/internal/mail-otp/owner",
        &json!({ "cookies": cookies }),
    )
    .await?;
    match response.status_code() {
        200 => {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Owner {
                owner_hash: String,
            }
            Ok(Ok(response.json::<Owner>().await?.owner_hash))
        }
        401 => Ok(Err(error(
            401,
            "session_invalid",
            "VTOP does not accept this session; sign in again.",
        )?)),
        status => {
            worker::console_log!("mail-otp: owner check failed: {status}");
            Ok(Err(error(
                503,
                "vtop_unreachable",
                "Could not reach VTOP to check the session. Try again.",
            )?))
        }
    }
}

async fn forget_owner(env: &Env, owner: &str, id: &str) -> Result<()> {
    call(
        &owner_stub(env, owner)?,
        Method::Delete,
        "",
        Some(id.to_string()),
    )
    .await?;
    Ok(())
}

fn mailbox_reply(env: &Env, id: &str, secret: &str, status: u16) -> Result<Response> {
    let body =
        json!({ "id": id, "address": address(id, &var(env, "MAIL_DOMAIN")?), "secret": secret });
    Ok(Response::from_json(&body)?.with_status(status))
}

async fn mail_register(mut req: Request, env: &Env) -> Result<Response> {
    let body = match mail_body(&mut req).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let key = match public_key(body.public_key.as_deref()) {
        Ok(Some(key)) => key,
        Ok(None) => return bad_request("publicKey is required."),
        Err(message) => return bad_request(message),
    };
    let fcm = match fcm_token(body.fcm_token.as_deref()) {
        Ok(token) => token,
        Err(message) => return bad_request(message),
    };
    let device_id = match resolve_device(env, &body.device, fcm).await? {
        Ok(id) => id,
        Err(message) => return bad_request(message),
    };
    let secret = tokens::random_token(MAILBOX_SECRET_PREFIX);
    let now = now();
    for _ in 0..3 {
        let id = new_id();
        let doc = MailboxDoc {
            id: id.clone(),
            owner_hash: None,
            members: vec![new_member(
                device_id.clone(),
                &secret,
                key.clone(),
                body.label.as_deref(),
                now,
            )],
            created_at: now,
            updated_at: now,
        };
        let response = call(
            &mailbox_stub(env, &id)?,
            Method::Post,
            "/create",
            Some(serde_json::to_string(&doc)?),
        )
        .await?;
        if response.status_code() == 201 {
            worker::console_log!("mail-otp: registered a mailbox");
            return mailbox_reply(env, &id, &secret, 201);
        }
    }
    error(
        503,
        "store_unavailable",
        "Could not create an address. Try again.",
    )
}

async fn mail_update(mut req: Request, env: &Env) -> Result<Response> {
    let body = match mail_body(&mut req).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let Some((mut doc, index)) = owned(env, &body).await? else {
        return unknown_mailbox();
    };
    let key = match public_key(body.public_key.as_deref()) {
        Ok(key) => key,
        Err(message) => return bad_request(message),
    };
    let fcm = match fcm_token(body.fcm_token.as_deref()) {
        Ok(token) => token,
        Err(message) => return bad_request(message),
    };
    let label = member_label(body.label.as_deref());
    let moves = body.device.device_id.is_some();
    if key.is_none() && fcm.is_none() && label.is_none() && !moves {
        return bad_request("Nothing to update.");
    }
    if let Some(key) = key {
        doc.members[index].set_public_key(key);
    }
    if label.is_some() {
        doc.members[index].label = label;
    }
    if moves {
        match resolve_device(env, &body.device, fcm).await? {
            Ok(id) => doc.members[index].device_id = id,
            Err(message) => return bad_request(message),
        }
    } else if let Some(token) = fcm {
        set_device_fcm(env, &doc.members[index].device_id, &token).await?;
    }
    doc.updated_at = now();
    save_mailbox(env, &doc).await?;
    Ok(Response::empty()?.with_status(204))
}

async fn mail_delete(mut req: Request, env: &Env) -> Result<Response> {
    let body = match mail_body(&mut req).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let Some((mut doc, index)) = owned(env, &body).await? else {
        return unknown_mailbox();
    };
    if !body.everywhere && doc.members.len() > 1 {
        doc.members.remove(index);
        doc.updated_at = now();
        save_mailbox(env, &doc).await?;
        worker::console_log!("mail-otp: a device left a mailbox");
        return Ok(Response::empty()?.with_status(204));
    }
    delete_mailbox(env, &doc).await?;
    Ok(Response::empty()?.with_status(204))
}

async fn delete_mailbox(env: &Env, doc: &MailboxDoc) -> Result<()> {
    if let Some(owner) = &doc.owner_hash {
        forget_owner(env, owner, &doc.id).await?;
    }
    call(&mailbox_stub(env, &doc.id)?, Method::Delete, "/doc", None).await?;
    worker::console_log!("mail-otp: deleted a mailbox");
    Ok(())
}

/// The installs an address forwards to, for the app's device list.
async fn mail_devices(mut req: Request, env: &Env) -> Result<Response> {
    let body = match mail_body(&mut req).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let Some((doc, index)) = owned(env, &body).await? else {
        return unknown_mailbox();
    };
    let devices: Vec<_> = doc
        .members
        .iter()
        .enumerate()
        .map(|(i, m)| {
            json!({
                "deviceId": m.device_id,
                "label": m.label,
                "addedAt": m.added_at,
                "seenAt": m.seen_at,
                "thisDevice": i == index,
            })
        })
        .collect();
    Response::from_json(&json!({ "devices": devices }))
}

/// Takes another install off the address; it stops getting OTPs.
async fn mail_remove_device(mut req: Request, env: &Env) -> Result<Response> {
    let body = match mail_body(&mut req).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let Some((mut doc, index)) = owned(env, &body).await? else {
        return unknown_mailbox();
    };
    let Some(target) = body.remove_device.as_deref() else {
        return bad_request("removeDevice is required.");
    };
    let Some(position) = doc.members.iter().position(|m| m.device_id == target) else {
        return error(
            404,
            "unknown_device",
            "That device does not use this address.",
        );
    };
    if position == index {
        return bad_request("Use delete to take this device off the address.");
    }
    doc.members.remove(position);
    doc.updated_at = now();
    save_mailbox(env, &doc).await?;
    worker::console_log!("mail-otp: a device was removed from a mailbox");
    Ok(Response::empty()?.with_status(204))
}

async fn mail_inbox(mut req: Request, env: &Env) -> Result<Response> {
    let body = match mail_body(&mut req).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let Some((doc, index)) = owned(env, &body).await? else {
        return unknown_mailbox();
    };
    Response::from_json(&match doc.members[index].fresh_sealed(now()) {
        Some((sealed, at)) => json!({ "sealed": sealed, "receivedAt": at }),
        None => json!({ "sealed": null }),
    })
}

/// Ties the mailbox to the student whose session comes with it, so a new
/// phone of theirs can take it back with `/reclaim`. The latest wins.
async fn mail_owner(mut req: Request, env: &Env) -> Result<Response> {
    let body = match mail_body(&mut req).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let Some((mut doc, _)) = owned(env, &body).await? else {
        return unknown_mailbox();
    };
    let owner = match session_owner(env, body.session.as_ref()).await? {
        Ok(owner) => owner,
        Err(response) => return Ok(response),
    };
    if let Some(previous) = doc.owner_hash.clone().filter(|hash| *hash != owner) {
        forget_owner(env, &previous, &doc.id).await?;
    }
    doc.owner_hash = Some(owner.clone());
    doc.updated_at = now();
    save_mailbox(env, &doc).await?;
    call(
        &owner_stub(env, &owner)?,
        Method::Put,
        "",
        Some(doc.id.clone()),
    )
    .await?;
    Ok(Response::empty()?.with_status(204))
}

/// Adds this install to the caller's own address, with its own key and
/// secret. The other installs keep getting OTPs.
async fn mail_reclaim(mut req: Request, env: &Env) -> Result<Response> {
    let body = match mail_body(&mut req).await {
        Ok(body) => body,
        Err(response) => return response,
    };
    let key = match public_key(body.public_key.as_deref()) {
        Ok(Some(key)) => key,
        Ok(None) => return bad_request("publicKey is required."),
        Err(message) => return bad_request(message),
    };
    let fcm = match fcm_token(body.fcm_token.as_deref()) {
        Ok(token) => token,
        Err(message) => return bad_request(message),
    };
    let owner = match session_owner(env, body.session.as_ref()).await? {
        Ok(owner) => owner,
        Err(response) => return Ok(response),
    };
    let mut index = call(&owner_stub(env, &owner)?, Method::Get, "", None).await?;
    if index.status_code() != 200 {
        return unknown_mailbox();
    }
    let id = index.text().await?;
    let Some(mut doc) = load_mailbox(env, &id)
        .await?
        .filter(|doc| doc.owner_hash.as_deref() == Some(owner.as_str()))
    else {
        return unknown_mailbox();
    };
    let device_id = match resolve_device(env, &body.device, fcm).await? {
        Ok(id) => id,
        Err(message) => return bad_request(message),
    };
    let secret = tokens::random_token(MAILBOX_SECRET_PREFIX);
    let now = now();
    doc.add_member(
        new_member(device_id, &secret, key, body.label.as_deref(), now),
        now,
    );
    doc.updated_at = now;
    save_mailbox(env, &doc).await?;
    worker::console_log!("mail-otp: a device joined a mailbox");
    mailbox_reply(env, &doc.id, &secret, 200)
}

// ---- entry points -----------------------------------------------------------

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let path = req.path();
    match (req.method(), path.as_str()) {
        (Method::Get, "/healthz") => {
            Response::from_json(&json!({ "service": "vtop-edge", "status": "ok" }))
        }
        (Method::Get, "/v1/events") => events(req, &env).await,
        (Method::Post, "/v1/devices/register") => device_register(req, &env).await,
        (Method::Post, "/v1/devices/update") => device_update(req, &env).await,
        (Method::Post, "/v1/mail-otp/register") => mail_register(req, &env).await,
        (Method::Post, "/v1/mail-otp/update") => mail_update(req, &env).await,
        (Method::Post, "/v1/mail-otp/delete") => mail_delete(req, &env).await,
        (Method::Post, "/v1/mail-otp/inbox") => mail_inbox(req, &env).await,
        (Method::Post, "/v1/mail-otp/owner") => mail_owner(req, &env).await,
        (Method::Post, "/v1/mail-otp/reclaim") => mail_reclaim(req, &env).await,
        (Method::Post, "/v1/mail-otp/devices") => mail_devices(req, &env).await,
        (Method::Post, "/v1/mail-otp/devices/remove") => mail_remove_device(req, &env).await,
        (Method::Post, "/internal/mail-otp/await") => mail_await(req, &env).await,
        (_, path) if path.starts_with("/internal/accounts/") => {
            account::internal(req, &env, path).await
        }
        (_, path) if account::handles(path) => {
            let origin = req.headers().get("origin")?;
            let response = if req.method() == Method::Options {
                Response::empty()?.with_status(204)
            } else {
                account::handle(req, &env, path).await?
            };
            with_cors(response, origin.as_deref(), &env)
        }
        // Engine-only: reaches the container whatever BRIDGE_URL says, to
        // try it before switching.
        (_, path) if path.starts_with("/internal/engine/") => {
            if !is_engine(&req, &env) {
                return error(401, "unauthorized", "The engine secret is required.");
            }
            let inner = path.trim_start_matches("/internal/engine").to_string();
            let request = Request::new_with_init(
                &format!("http://engine{inner}"),
                RequestInit::new().with_method(req.method()),
            )?;
            env.durable_object("ENGINE")?
                .get_by_name("engine")?
                .fetch_with_request(request)
                .await
        }
        (_, path) if path.starts_with("/internal/") => error(404, "not_found", "No such route."),
        (_, path) if path == "/mcp" || path.starts_with("/mcp/") => {
            pass_on(req, &env, "MCP_URL").await
        }
        _ => error(404, "not_found", "No such route."),
    }
}

/// Hands a request to the service at `[vars].<service>` as it came, with
/// the caller's address for its rate limits.
async fn pass_on(mut req: Request, env: &Env, service: &str) -> Result<Response> {
    let url = req.url()?;
    let mut target = url.path().to_string();
    if let Some(query) = url.query() {
        target = format!("{target}?{query}");
    }
    let headers = Headers::new();
    for (name, value) in req.headers().entries() {
        if !matches!(name.as_str(), "host" | "x-forwarded-for" | "x-real-ip")
            && !name.starts_with("cf-")
        {
            headers.append(&name, &value)?;
        }
    }
    if let Some(ip) = req.headers().get("cf-connecting-ip")? {
        headers.set("x-forwarded-for", &ip)?;
    }
    let method = req.method();
    let mut init = RequestInit::new();
    init.with_method(method.clone()).with_headers(headers);
    if !matches!(method, Method::Get | Method::Head | Method::Options) {
        let body = req.bytes().await?;
        if !body.is_empty() {
            init.with_body(Some(js_sys::Uint8Array::from(body.as_slice()).into()));
        }
    }
    engine_send(env, service, &target, &init).await
}

/// The catch-all. Throwing makes the sending server retry later.
#[event(email)]
async fn email(message: ForwardableEmailMessage, env: Env, _ctx: Context) -> Result<()> {
    let Some(id) = id_from_address(&message.to()) else {
        match env.var("FALLBACK_FORWARD").map(|v| v.to_string()) {
            Ok(to) if !to.is_empty() => {
                message
                    .forward(&to)
                    .await
                    .map_err(|e| worker::Error::RustError(format!("forward: {e:?}")))?;
            }
            _ => message.set_reject("Unknown address"),
        }
        return Ok(());
    };
    let Some(doc) = load_mailbox(&env, &id).await? else {
        message.set_reject("Unknown address");
        return Ok(());
    };
    let raw = message.raw_bytes().await?;
    let Some(payload) = mail::payload_for(&raw, now()) else {
        worker::console_log!("mail-otp: dropped a mail for a mailbox");
        return Ok(());
    };
    let plaintext = serde_json::to_vec(&payload)?;
    // Sealed once per install, each to its own key.
    let mut boxes = Vec::with_capacity(doc.members.len());
    for member in &doc.members {
        let sealed = mail_seal::seal(&member.public_key, &id, &plaintext)
            .map_err(|e| worker::Error::RustError(format!("seal: {e:?}")))?;
        boxes.push((member.device_id.clone(), sealed));
    }
    let body: Vec<_> = boxes
        .iter()
        .map(|(device_id, sealed)| json!({ "deviceId": device_id, "sealed": sealed }))
        .collect();
    let delivered = call(
        &mailbox_stub(&env, &id)?,
        Method::Post,
        "/deliver",
        Some(json!({ "boxes": body }).to_string()),
    )
    .await?;
    if delivered.status_code() != 200 {
        return Err(format!("deliver failed: {}", delivered.status_code()).into());
    }
    // The code is stored now. What follows is best effort: an error here
    // must not make the mail server send it again (a second push for the
    // same code).
    if matches!(payload, mail::Payload::Otp { .. }) {
        if let Err(e) = hand_to_waiters(&env, &id, &payload, &plaintext).await {
            worker::console_log!("mail-otp: handing to a server sign-in failed: {e}");
        }
    }
    for (device_id, sealed) in boxes {
        let event = json!({ "body": EventBody::MailOtp { mailbox: id.clone(), sealed } });
        let appended = match device_stub(&env, &device_id) {
            Ok(stub) => call(&stub, Method::Post, "/append", Some(event.to_string()))
                .await
                .map(|response| response.status_code()),
            Err(e) => Err(e),
        };
        if !matches!(appended, Ok(200)) {
            // The box is kept; the app still pulls it from /inbox.
            worker::console_log!("mail-otp: event append failed: {appended:?}");
        }
    }
    worker::console_log!(
        "mail-otp: relayed a {} to {} device(s)",
        payload.kind(),
        doc.members.len()
    );
    Ok(())
}

/// Seals an OTP to each engine sign-in waiting on this mailbox.
async fn hand_to_waiters(
    env: &Env,
    id: &str,
    payload: &mail::Payload,
    plaintext: &[u8],
) -> Result<()> {
    let stub = mailbox_stub(env, id)?;
    let mut listed = call(&stub, Method::Get, "/waiters", None).await?;
    if listed.status_code() != 200 {
        return Ok(());
    }
    let waiters: Vec<state::Waiter> = listed.json().await?;
    let now = now();
    for waiter in waiters.iter().filter(|w| w.wants(payload.sent_at(), now)) {
        let Ok(sealed) = mail_seal::seal(&waiter.public_key, &waiter.owner_hash, plaintext) else {
            continue;
        };
        let body = json!({ "publicKey": waiter.public_key, "sealed": sealed });
        call(
            &stub,
            Method::Post,
            "/waiter-result",
            Some(body.to_string()),
        )
        .await?;
        worker::console_log!("mail-otp: handed an OTP to a server sign-in");
    }
    Ok(())
}

/// The caller holds the secret shared with the engine.
pub(crate) fn is_engine(req: &Request, env: &Env) -> bool {
    let Ok(secret) = env.secret("BRIDGE_SECRET").map(|s| s.to_string()) else {
        return false;
    };
    let presented = req
        .headers()
        .get("authorization")
        .ok()
        .flatten()
        .and_then(|value| value.strip_prefix("Bearer ").map(str::to_string))
        .unwrap_or_default();
    !secret.is_empty() && tokens::hashes_equal(&tokens::hash(&secret), &tokens::hash(&presented))
}
