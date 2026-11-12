//! The Server page (Settings → Account): what the edge keeps for this student and the phones
//! linked to the account (`rust/vtop-edge/API.md`, "Accounts — phones"). `/v1/account` sends the
//! live VTOP session; `/v1/app/*` names the account by registration number. Every call is signed
//! as this install ([`Core::edge_post`]); cookies are never logged.

use std::sync::OnceLock;

use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

use crate::device::EdgeError;
use crate::keys::{AccessKey, KeyView, refused};
use crate::{Core, CoreError, traced};

/// The name the server shows for this phone, set by the app at start.
static LABEL: OnceLock<String> = OnceLock::new();

/// This phone's name on the server: its model, or `vitapmate` before the app set it.
pub(crate) fn label() -> &'static str {
    LABEL.get().map_or("vitapmate", String::as_str)
}

/// Sets the name the server shows for this phone ("OnePlus CPH2573"); the first call wins.
#[uniffi::export]
pub fn set_device_label(label: String) {
    let _ = LABEL.set(label.trim().chars().take(64).collect());
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ServerSignIn {
    /// Unix seconds.
    pub at: i64,
    /// The server signed in with the saved password, else a phone did.
    pub by_server: bool,
    pub ok: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct LinkedPhone {
    /// The short id the edge removes it by.
    pub id: String,
    pub label: Option<String>,
    pub added_at: i64,
    pub seen_at: i64,
    pub this_phone: bool,
}

/// How the server serves sessions to connected apps (`/v1/account/settings`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase", default)]
pub struct ServerOptions {
    /// Sign in with the saved password at once, without asking a phone first.
    pub always_use_vault: bool,
    /// Otherwise, how long a phone gets to answer first: 10, 20 or 45.
    pub phone_wait_secs: u32,
    /// How long the saved password is kept: 86400, 172800, or None (until VTOP rejects it).
    pub vault_ttl_secs: Option<i64>,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            always_use_vault: true,
            phone_wait_secs: 20,
            vault_ttl_secs: None,
        }
    }
}

/// A phone on this phone's forwarding address.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MailPhone {
    /// Its device id, which removes it.
    pub id: String,
    pub label: Option<String>,
    pub added_at: i64,
    pub seen_at: i64,
    pub this_phone: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ServerMailbox {
    pub address: String,
    pub phones: Vec<MailPhone>,
}

/// This install as the server knows it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ServerDevice {
    /// The first 8 characters of its id; None while it isn't registered.
    pub id: Option<String>,
    pub label: String,
    pub push_token: bool,
    /// From the server; None when it doesn't say (before `/v1/devices/info`).
    pub registered_at: Option<i64>,
    pub events: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ServerStatus {
    /// This phone is one of the account's phones; the account's lists are empty when it is not.
    pub linked: bool,
    pub registration_number: String,
    /// When the server made the account (None from servers that don't say, or no account yet).
    pub created_at: Option<i64>,
    /// The semester connected apps read.
    pub semester_id: Option<String>,
    pub options: ServerOptions,
    /// The student has a forwarding address, on this phone or another.
    pub forwarding: bool,
    /// This phone's forwarding address and the phones on it.
    pub mailbox: Option<ServerMailbox>,
    pub device: ServerDevice,
    /// When the sealed VTOP session was saved, and by whom (`phone` or `server`).
    pub session_saved_at: Option<i64>,
    pub session_source: Option<String>,
    /// When the sealed password was saved and when it expires (None: kept until forgotten).
    pub vault_saved_at: Option<i64>,
    pub vault_expires_at: Option<i64>,
    pub vault_gmail: bool,
    pub credentials_rejected_at: Option<i64>,
    /// Newest first, at most 10.
    pub sign_ins: Vec<ServerSignIn>,
    pub phones: Vec<LinkedPhone>,
    pub keys: Vec<AccessKey>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountView {
    registration_number: String,
    created_at: Option<i64>,
    semester_id: Option<String>,
    #[serde(default)]
    settings: ServerOptions,
    #[serde(default)]
    forwarding: bool,
    credentials_rejected_at: Option<i64>,
    #[serde(default)]
    this_phone: bool,
    #[serde(default)]
    phones: Vec<PhoneView>,
    #[serde(default)]
    keys: Vec<KeyView>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PhoneView {
    id: String,
    label: Option<String>,
    added_at: i64,
    seen_at: i64,
    this_phone: bool,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct StatusView {
    session_saved_at: Option<i64>,
    session_source: Option<String>,
    vault_saved_at: Option<i64>,
    vault_expires_at: Option<i64>,
    vault_gmail: bool,
    sign_ins: Vec<SignInView>,
}

#[derive(Deserialize)]
struct SignInView {
    at: i64,
    by: String,
    ok: bool,
    reason: Option<String>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct DeviceView {
    label: Option<String>,
    push_token: bool,
    created_at: Option<i64>,
    events: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MailPhoneView {
    device_id: String,
    label: Option<String>,
    added_at: i64,
    seen_at: i64,
    this_device: bool,
}

fn unexpected(_: serde_json::Error) -> CoreError {
    CoreError::Other {
        reason: "The server sent an unexpected answer.".into(),
    }
}

/// The edge has no such route (it predates it): `Ok(false)`.
fn supported(answer: Result<Value, EdgeError>, path: &str) -> Result<bool, CoreError> {
    match answer {
        Err(error) if error.code() == "not_found" => {
            info!(path, "not on this server yet");
            Ok(false)
        }
        answer => answer.map(|_| true).map_err(refused),
    }
}

/// Gmail access the server may use to read the sign-in code while no phone answers (the
/// student's own Google client, from OTP auto-fill).
#[derive(uniffi::Record)]
pub struct GmailGrant {
    pub refresh_token: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub delete_after_reading: bool,
}

/// Server-page calls; each takes the caller's flow id first.
#[uniffi::export]
impl Core {
    /// Everything the server keeps for this student and this phone: the account and its options,
    /// phones, keys, saved session and password, sign-ins, the forwarding address and this
    /// install's registration.
    pub async fn server_status(&self, flow: String) -> Result<ServerStatus, CoreError> {
        traced("server_status", &flow, async {
            let cookies = self.live_cookies().await?;
            self.status_with(&self.forwarding_server(), &flow, &cookies)
                .await
        })
        .await
    }

    /// Removes a linked phone (an id from [`ServerStatus::phones`]).
    pub async fn remove_phone(&self, flow: String, id: String) -> Result<(), CoreError> {
        let path = "/v1/account/remove-phone";
        self.account_op("remove_phone", flow, path, json!({ "id": id }))
            .await
            .map(drop)
    }

    /// Tells the server the semester picked here, so connected apps read the same one. Does
    /// nothing while no semester is picked; fails with `not_linked` before a key is made.
    pub async fn send_server_semester(&self, flow: String) -> Result<(), CoreError> {
        let Some(semester) = self.read::<String>(crate::SELECTED) else {
            return Ok(());
        };
        let path = "/v1/account/semester";
        self.account_op(
            "send_server_semester",
            flow,
            path,
            json!({ "semesterId": semester }),
        )
        .await
        .map(drop)
    }

    /// Hands the server this phone's VTOP password, sealed, so it signs in for connected apps
    /// while no phone answers. It needs the sign-in code too: the server keeps the password only
    /// with a forwarding address or [`GmailGrant`], and says so otherwise.
    pub async fn save_server_password(
        &self,
        flow: String,
        gmail: Option<GmailGrant>,
    ) -> Result<(), CoreError> {
        traced("save_server_password", &flow, async {
            let cookies = self.live_cookies().await?;
            self.save_password_with(&self.forwarding_server(), &flow, &cookies, gmail)
                .await
        })
        .await
    }

    /// Sends the password again with the OTP auto-fill now in use ([`GmailGrant`] for Gmail,
    /// none for forwarding), only when the server already keeps one; false when it keeps none.
    pub async fn resend_server_password(
        &self,
        flow: String,
        gmail: Option<GmailGrant>,
    ) -> Result<bool, CoreError> {
        traced("resend_server_password", &flow, async {
            let cookies = self.live_cookies().await?;
            let server = self.forwarding_server();
            let body = json!({ "session": { "cookies": cookies } });
            let account = self
                .edge_post(&server, "/v1/account", &flow, &body)
                .await
                .map_err(refused)?;
            if account["savedCredentials"] != json!(true) {
                return Ok(false);
            }
            self.save_password_with(&server, &flow, &cookies, gmail)
                .await
                .map(|()| true)
        })
        .await
    }

    /// Drops the sealed password; the server can then no longer sign in by itself.
    pub async fn forget_server_password(&self, flow: String) -> Result<(), CoreError> {
        let path = "/v1/account/forget-credentials";
        self.account_op("forget_server_password", flow, path, json!({}))
            .await
            .map(drop)
    }

    /// Deletes the account on the server: phones, keys, saved session and password, options and
    /// sign-ins. Not the forwarding address, nor this phone's registration.
    pub async fn delete_server_account(&self, flow: String) -> Result<(), CoreError> {
        let path = "/v1/account/delete";
        self.account_op("delete_server_account", flow, path, json!({}))
            .await
            .map(drop)
    }

    /// Sets how the server serves sessions (see [`ServerOptions`]).
    pub async fn set_server_options(
        &self,
        flow: String,
        options: ServerOptions,
    ) -> Result<(), CoreError> {
        let settings = json!({ "settings": {
            "alwaysUseVault": options.always_use_vault,
            "phoneWaitSecs": options.phone_wait_secs,
            "vaultTtlSecs": options.vault_ttl_secs,
        } });
        let path = "/v1/account/settings";
        self.account_op("set_server_options", flow, path, settings)
            .await
            .map(drop)
    }

    /// Drops the sign-in history; false when the server doesn't offer it yet.
    pub async fn clear_server_sign_ins(&self, flow: String) -> Result<bool, CoreError> {
        let path = "/v1/account/clear-sign-ins";
        self.account_op("clear_server_sign_ins", flow, path, json!({}))
            .await
    }

    /// Drops the sealed VTOP session the server keeps for the extension and MCP.
    pub async fn clear_server_session(&self, flow: String) -> Result<(), CoreError> {
        traced("clear_server_session", &flow, async {
            let regno = self
                .saved_account(flow.clone())
                .and_then(|account| account.registration_number)
                .ok_or(CoreError::NotSignedIn)?;
            let body = json!({ "registrationNumber": regno });
            self.edge_post(
                &self.forwarding_server(),
                "/v1/app/session/clear",
                &flow,
                &body,
            )
            .await
            .map_err(refused)?;
            info!("server session cleared");
            Ok(())
        })
        .await
    }

    /// Takes another phone (a [`MailPhone::id`]) off this phone's forwarding address.
    pub async fn remove_forwarding_phone(&self, flow: String, id: String) -> Result<(), CoreError> {
        traced("remove_forwarding_phone", &flow, async {
            let (server, mailbox, _) = self.saved_mailbox().ok_or_else(|| CoreError::Other {
                reason: "Mail forwarding is off.".into(),
            })?;
            let body = json!({ "id": mailbox, "removeDevice": id });
            self.edge_post(&server, "/v1/mail-otp/devices/remove", &flow, &body)
                .await?;
            info!("phone removed from the address");
            Ok(())
        })
        .await
    }

    /// Deletes this phone's forwarding address for every phone on it.
    pub async fn delete_forwarding_everywhere(&self, flow: String) -> Result<(), CoreError> {
        traced(
            "delete_forwarding_everywhere",
            &flow,
            self.leave_forwarding(&flow, true),
        )
        .await
    }

    /// Deletes this install on the server (keys, push token, events), unlinks it from the account
    /// and takes it off its forwarding address there, then forgets its id, so the next call
    /// registers afresh. False when the server doesn't offer it yet.
    pub async fn delete_server_device(&self, flow: String) -> Result<bool, CoreError> {
        traced("delete_server_device", &flow, async {
            let server = self.forwarding_server();
            if self.registration(&server).0.is_none() {
                return Ok(true);
            }
            let mut body = json!({});
            if let Some(regno) = self
                .saved_account(flow.clone())
                .and_then(|account| account.registration_number)
            {
                body["registrationNumber"] = json!(regno);
            }
            let mailbox = self
                .saved_mailbox()
                .filter(|(on, ..)| *on == server)
                .map(|(_, id, _)| id);
            if let Some(id) = &mailbox {
                body["mailbox"] = json!(id);
            }
            let path = "/v1/devices/delete";
            if !supported(self.edge_post(&server, path, &flow, &body).await, path)? {
                return Ok(false);
            }
            self.forget_device_id(&server);
            if let Some(id) = mailbox {
                self.forget_forwarding(&id);
            }
            info!("device deleted on the server");
            Ok(true)
        })
        .await
    }
}

impl Core {
    /// `/v1/account`, `/v1/devices/info`, `/v1/app/status` once this phone is linked, and
    /// `/v1/mail-otp/devices` when it has an address.
    async fn status_with(
        &self,
        server: &str,
        flow: &str,
        cookies: &str,
    ) -> Result<ServerStatus, CoreError> {
        let body = json!({ "session": { "cookies": cookies } });
        let account = self
            .edge_post(server, "/v1/account", flow, &body)
            .await
            .map_err(refused)?;
        let account: AccountView = serde_json::from_value(account).map_err(unexpected)?;
        // Older servers lack it: the phone's own view then.
        let device: DeviceView = self
            .edge_post(server, "/v1/devices/info", flow, &json!({}))
            .await
            .inspect_err(|error| info!(code = error.code(), "no device info"))
            .ok()
            .and_then(|info| serde_json::from_value(info).ok())
            .unwrap_or_default();
        let status = if account.this_phone {
            let body = json!({ "registrationNumber": account.registration_number });
            let status = self
                .edge_post(server, "/v1/app/status", flow, &body)
                .await
                .map_err(refused)?;
            serde_json::from_value(status).map_err(unexpected)?
        } else {
            StatusView::default()
        };
        let mailbox = match self.saved_mailbox() {
            None => None,
            Some((on, id, address)) => {
                let body = json!({ "id": id });
                match self
                    .edge_post(&on, "/v1/mail-otp/devices", flow, &body)
                    .await
                {
                    Ok(list) => {
                        let phones: Vec<MailPhoneView> =
                            serde_json::from_value(list["devices"].clone()).map_err(unexpected)?;
                        let phones = phones
                            .into_iter()
                            .map(|p| MailPhone {
                                id: p.device_id,
                                label: p.label,
                                added_at: p.added_at,
                                seen_at: p.seen_at,
                                this_phone: p.this_device,
                            })
                            .collect();
                        Some(ServerMailbox { address, phones })
                    }
                    Err(error) if error.code() == "mailbox_not_found" => {
                        self.forget_forwarding(&id);
                        None
                    }
                    Err(error) => return Err(error.into()),
                }
            }
        };
        let (id, push_token) = self.registration(server);
        info!(
            linked = account.this_phone,
            phones = account.phones.len(),
            keys = account.keys.len(),
            mailbox = mailbox.is_some(),
            "server status"
        );
        Ok(ServerStatus {
            linked: account.this_phone,
            registration_number: account.registration_number,
            created_at: account.created_at,
            semester_id: account.semester_id,
            options: account.settings,
            forwarding: account.forwarding,
            mailbox,
            device: ServerDevice {
                id: id.map(|id| id.chars().take(8).collect()),
                label: device.label.unwrap_or_else(|| label().to_string()),
                push_token: device.push_token || push_token,
                registered_at: device.created_at,
                events: device.events,
            },
            session_saved_at: status.session_saved_at,
            session_source: status.session_source,
            vault_saved_at: status.vault_saved_at,
            vault_expires_at: status.vault_expires_at,
            vault_gmail: status.vault_gmail,
            credentials_rejected_at: account.credentials_rejected_at,
            sign_ins: status
                .sign_ins
                .into_iter()
                .map(|s| ServerSignIn {
                    at: s.at,
                    by_server: s.by == "server",
                    ok: s.ok,
                    reason: s.reason,
                })
                .collect(),
            phones: account
                .phones
                .into_iter()
                .map(|p| LinkedPhone {
                    id: p.id,
                    label: p.label,
                    added_at: p.added_at,
                    seen_at: p.seen_at,
                    this_phone: p.this_phone,
                })
                .collect(),
            keys: account.keys.into_iter().map(AccessKey::from).collect(),
        })
    }

    /// A `/v1/account/*` call with the live session added to `body`; false when the server has
    /// no such route yet.
    async fn account_op(
        &self,
        name: &str,
        flow: String,
        path: &str,
        mut body: Value,
    ) -> Result<bool, CoreError> {
        traced(name, &flow, async {
            body["session"] = json!({ "cookies": self.live_cookies().await? });
            let answer = self
                .edge_post(&self.forwarding_server(), path, &flow, &body)
                .await;
            let done = supported(answer, path)?;
            if done {
                info!(path, "server account changed");
            }
            Ok(done)
        })
        .await
    }
}

impl Core {
    /// [`Core::save_server_password`] against `server` with `cookies`.
    async fn save_password_with(
        &self,
        server: &str,
        flow: &str,
        cookies: &str,
        gmail: Option<GmailGrant>,
    ) -> Result<(), CoreError> {
        let account = self
            .read::<crate::Account>(crate::ACCOUNT)
            .ok_or(CoreError::NotSignedIn)?;
        let otp = if gmail.is_some() {
            "gmail"
        } else {
            "forwarding"
        };
        let mut credentials =
            json!({ "username": account.username, "password": account.password, "otp": otp });
        if let Some(grant) = gmail {
            credentials["gmail"] = json!({
                "refreshToken": grant.refresh_token,
                "clientId": grant.client_id,
                "clientSecret": grant.client_secret,
                "deleteAfterReading": grant.delete_after_reading,
            });
        }
        let body = json!({
            "session": { "cookies": cookies },
            "label": label(),
            "credentials": credentials,
        });
        let answer = self
            .edge_post(server, "/v1/link", flow, &body)
            .await
            .map_err(refused)?;
        if answer["savedCredentials"] != json!(true) {
            return Err(CoreError::Other {
                reason: "The server keeps a password only with a forwarding address or \
                             Gmail OTP auto-fill set up."
                    .into(),
            });
        }
        info!("password sealed on the server");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::forwarding::tests::{DEVICE_ID, Edge, serve};

    fn core(name: &str) -> Core {
        let dir =
            std::env::temp_dir().join(format!("vtop-ffi-server-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Core {
            cache: dir.clone(),
            dir,
            slot: Mutex::new(None),
        }
    }

    fn can(edge: &Arc<Mutex<Edge>>, path: &str, status: u16, body: Value) {
        let mut edge = edge.lock().unwrap();
        edge.canned
            .entry(path.into())
            .or_default()
            .push_back((status, body));
    }

    fn paths(edge: &Arc<Mutex<Edge>>) -> Vec<String> {
        let edge = edge.lock().unwrap();
        edge.requests.iter().map(|r| r.path.clone()).collect()
    }

    /// This phone's forwarding address on `server`, as `forwarding.json` holds it.
    fn mailbox(core: &Core, server: &str) {
        let saved = json!({ "server": server, "id": "k3f9x2ab7q", "address": "otp+k3f9x2ab7q@vmate.aster0.dev" });
        fs::write(core.at(crate::forwarding::FILE), saved.to_string()).unwrap();
    }

    #[tokio::test]
    async fn status_reads_everything_the_server_keeps() {
        let core = core("status");
        let edge = Arc::new(Mutex::new(Edge::default()));
        let server = serve(edge.clone()).await;
        mailbox(&core, &server);
        let account = json!({
            "registrationNumber": "22BCE0001", "createdAt": 4, "semesterId": "AP2025264", "forwarding": true,
            "settings": { "alwaysUseVault": false, "phoneWaitSecs": 45, "vaultTtlSecs": 86400 },
            "credentialsRejectedAt": 3, "thisPhone": true,
            "keys": [{ "id": "ab12cd34", "label": "MCP", "createdAt": 6, "lastUsedAt": null, "phoneId": "p1", "outings": false }],
            "phones": [
                { "id": "p1", "label": "OnePlus CPH2573", "addedAt": 1, "seenAt": 5, "thisPhone": true },
                { "id": "p2", "label": null, "addedAt": 2, "seenAt": 2, "thisPhone": false },
            ],
        });
        let status = json!({
            "sessionSavedAt": 9, "sessionSource": "phone", "vaultSavedAt": null, "vaultExpiresAt": null,
            "vaultGmail": false, "credentialsRejectedAt": 3, "forwarding": null, "phones": 2, "keys": [],
            "signIns": [{ "at": 8, "by": "server", "ok": false, "reason": "VTOP down" }],
        });
        let info = json!({ "deviceId": DEVICE_ID, "label": "Pixel", "platform": "android", "pushToken": true,
            "createdAt": 7, "events": 2, "latest": 9 });
        let devices = json!({ "devices": [
            { "deviceId": DEVICE_ID, "label": "Pixel", "addedAt": 1, "seenAt": 2, "thisDevice": true },
            { "deviceId": "ff", "label": null, "addedAt": 3, "seenAt": 3, "thisDevice": false },
        ] });
        can(&edge, "/v1/account", 200, account);
        can(&edge, "/v1/devices/info", 200, info);
        can(&edge, "/v1/app/status", 200, status);
        can(&edge, "/v1/mail-otp/devices", 200, devices);
        let status = core
            .status_with(&server, "f-s", "JSESSIONID=c")
            .await
            .unwrap();
        assert!(status.linked && status.forwarding);
        assert_eq!(
            (
                status.registration_number.as_str(),
                status.created_at,
                status.semester_id.as_deref()
            ),
            ("22BCE0001", Some(4), Some("AP2025264"))
        );
        assert_eq!(
            status.options,
            ServerOptions {
                always_use_vault: false,
                phone_wait_secs: 45,
                vault_ttl_secs: Some(86400)
            }
        );
        assert_eq!(
            (status.session_saved_at, status.session_source.as_deref()),
            (Some(9), Some("phone"))
        );
        assert_eq!(
            (status.vault_saved_at, status.credentials_rejected_at),
            (None, Some(3))
        );
        assert_eq!(
            status.sign_ins,
            [ServerSignIn {
                at: 8,
                by_server: true,
                ok: false,
                reason: Some("VTOP down".into())
            }]
        );
        assert_eq!(status.phones.len(), 2);
        assert!(status.phones[0].this_phone && !status.phones[1].this_phone);
        assert_eq!(
            (
                status.keys.len(),
                status.keys[0].id.as_str(),
                status.keys[0].created_at
            ),
            (1, "ab12cd34", 6)
        );
        assert_eq!(
            status.device,
            ServerDevice {
                id: Some(DEVICE_ID[..8].into()),
                label: "Pixel".into(),
                push_token: true,
                registered_at: Some(7),
                events: Some(2),
            }
        );
        let mailbox = status.mailbox.unwrap();
        assert_eq!(mailbox.address, "otp+k3f9x2ab7q@vmate.aster0.dev");
        assert!(mailbox.phones[0].this_phone && mailbox.phones[1].id == "ff");
        assert_eq!(
            paths(&edge),
            [
                "/v1/devices/register",
                "/v1/account",
                "/v1/devices/info",
                "/v1/app/status",
                "/v1/mail-otp/devices"
            ]
        );
        let edge = edge.lock().unwrap();
        assert_eq!(
            edge.requests[3].body,
            json!({ "registrationNumber": "22BCE0001" })
        );
        assert_eq!(edge.requests[4].body, json!({ "id": "k3f9x2ab7q" }));
        assert!(edge.requests.iter().all(|r| r.signed && r.flow == "f-s"));
    }

    #[tokio::test]
    async fn an_older_server_and_an_unlinked_phone_still_show_what_applies() {
        let core = core("older");
        let edge = Arc::new(Mutex::new(Edge::default()));
        let server = serve(edge.clone()).await;
        // No createdAt, settings or /v1/devices/info (the fake answers 404 not_found).
        can(
            &edge,
            "/v1/account",
            200,
            json!({ "registrationNumber": "22BCE0001", "thisPhone": false }),
        );
        let status = core.status_with(&server, "f", "c=1").await.unwrap();
        assert!(!status.linked && status.mailbox.is_none());
        assert_eq!(
            (status.created_at, status.options.clone()),
            (None, ServerOptions::default())
        );
        assert_eq!(
            (
                status.device.id.as_deref(),
                status.device.events,
                status.device.label.as_str()
            ),
            (Some(&DEVICE_ID[..8]), None, "vitapmate")
        );
        assert_eq!(
            paths(&edge),
            ["/v1/devices/register", "/v1/account", "/v1/devices/info"]
        );
    }

    #[tokio::test]
    async fn deleting_this_device_forgets_its_id_unless_the_server_lacks_the_route() {
        let core = core("delete");
        let edge = Arc::new(Mutex::new(Edge::default()));
        let server = serve(edge.clone()).await;
        fs::write(core.at(crate::forwarding::SERVER_FILE), &server).unwrap();
        // Never registered: nothing to delete.
        assert_eq!(core.delete_server_device("f0".into()).await, Ok(true));
        assert!(paths(&edge).is_empty());

        core.device_id(&server, "f").await.unwrap();
        mailbox(&core, &server);
        // An older server: 404 not_found, nothing forgotten.
        assert_eq!(core.delete_server_device("f1".into()).await, Ok(false));
        assert!(core.registration(&server).0.is_some() && core.saved_mailbox().is_some());

        can(&edge, "/v1/devices/delete", 204, Value::Null);
        assert_eq!(core.delete_server_device("f2".into()).await, Ok(true));
        assert!(core.registration(&server).0.is_none() && core.saved_mailbox().is_none());
        let edge = edge.lock().unwrap();
        let last = edge.requests.last().unwrap();
        assert_eq!(
            (last.path.as_str(), &last.body),
            ("/v1/devices/delete", &json!({ "mailbox": "k3f9x2ab7q" }))
        );
    }

    #[tokio::test]
    async fn the_password_goes_sealed_only_when_the_server_can_get_the_code() {
        let core = core("password");
        let edge = Arc::new(Mutex::new(Edge::default()));
        let server = serve(edge.clone()).await;
        fs::write(
            core.at(crate::ACCOUNT),
            r#"{"username":"22BCE0001","password":"pw"}"#,
        )
        .unwrap();
        can(
            &edge,
            "/v1/link",
            200,
            json!({ "savedCredentials": true, "forwarding": false }),
        );
        can(
            &edge,
            "/v1/link",
            200,
            json!({ "savedCredentials": false, "forwarding": false }),
        );
        let gmail = GmailGrant {
            refresh_token: "r".into(),
            client_id: "c.apps.googleusercontent.com".into(),
            client_secret: None,
            delete_after_reading: true,
        };
        core.save_password_with(&server, "f", "c=1", Some(gmail))
            .await
            .unwrap();
        assert!(
            core.save_password_with(&server, "f", "c=1", None)
                .await
                .is_err()
        );
        let edge = edge.lock().unwrap();
        let sent = &edge
            .requests
            .iter()
            .find(|r| r.path == "/v1/link")
            .unwrap()
            .body;
        assert_eq!(sent["credentials"]["username"], "22BCE0001");
        assert_eq!(sent["credentials"]["otp"], "gmail");
        assert_eq!(
            sent["credentials"]["gmail"]["clientId"],
            "c.apps.googleusercontent.com"
        );
        assert_eq!(sent["session"]["cookies"], "c=1");
    }
}
