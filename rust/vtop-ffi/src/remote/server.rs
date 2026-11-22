//! The Server page (Settings → Account): what the edge keeps for this student and the phones
//! linked to the account (`edge/API.md`, "Accounts — phones"). `/v1/account` sends the
//! account token or the live VTOP session ([`crate::remote::account_token`]); `/v1/app/*` names the
//! account by registration number. Every call is signed as this install ([`Core::edge_post`]);
//! cookies are never logged.

use std::sync::OnceLock;

use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

use crate::device::EdgeError;
use crate::remote::keys::{AccessKey, KeyView, refused};
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
    #[expect(
        clippy::let_underscore_must_use,
        reason = "set once; a later call keeps the first label"
    )]
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
            let action = "Load what the server keeps for you";
            self.status_with(&self.forwarding_server(), &flow, action)
                .await
        })
        .await
    }

    /// Removes a linked phone (an id from [`ServerStatus::phones`]).
    pub async fn remove_phone(&self, flow: String, id: String) -> Result<(), CoreError> {
        let path = "/v1/account/remove-phone";
        let action = "Remove a phone from your account";
        self.account_op(
            "remove_phone",
            Some(action),
            flow,
            path,
            json!({ "id": id }),
        )
        .await
        .map(drop)
    }

    /// Tells the server the semester picked here, so connected apps read the same one. Does
    /// nothing while no semester is picked, or (it runs unasked) until the student ticked
    /// "don't ask again" for [`CookieKind::Account`]; fails with `not_linked` before a key is made.
    pub async fn send_server_semester(&self, flow: String) -> Result<(), CoreError> {
        let Some(semester) = self.read_kept::<String>(crate::SELECTED) else {
            return Ok(());
        };
        let path = "/v1/account/semester";
        let body = json!({ "semesterId": semester });
        match self
            .account_op("send_server_semester", None, flow, path, body)
            .await
        {
            Err(CoreError::ConsentDenied) => Ok(()),
            result => result.map(drop),
        }
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
            let action = "Save your password on the server (it gets your password too)";
            let server = self.forwarding_server();
            self.save_password_with(&server, &flow, action, true, gmail)
                .await
        })
        .await
    }

    /// Sends the password again with the OTP auto-fill now in use ([`GmailGrant`] for Gmail,
    /// none for forwarding), only when the server already keeps one; false when it keeps none,
    /// or (it runs unasked) the student hasn't ticked "don't ask again" for [`CookieKind::Account`].
    pub async fn resend_server_password(
        &self,
        flow: String,
        gmail: Option<GmailGrant>,
    ) -> Result<bool, CoreError> {
        traced("resend_server_password", &flow, async {
            let action = "Send your password to the server again";
            let server = self.forwarding_server();
            let account = self
                .account_post(&server, "/v1/account", &flow, action, false, json!({}))
                .await;
            let account = match account {
                Err(CoreError::ConsentDenied) => return Ok(false),
                account => account?.map_err(refused)?,
            };
            if account.get("savedCredentials") != Some(&json!(true)) {
                return Ok(false);
            }
            match self
                .save_password_with(&server, &flow, action, false, gmail)
                .await
            {
                Err(CoreError::ConsentDenied) => Ok(false),
                saved => saved.map(|()| true),
            }
        })
        .await
    }

    /// Drops the sealed password; the server can then no longer sign in by itself.
    pub async fn forget_server_password(&self, flow: String) -> Result<(), CoreError> {
        let path = "/v1/account/forget-credentials";
        let action = "Drop your password from the server";
        self.account_op(
            "forget_server_password",
            Some(action),
            flow,
            path,
            json!({}),
        )
        .await
        .map(drop)
    }

    /// Deletes the account on the server: phones, keys, saved session and password, options and
    /// sign-ins. Not the forwarding address, nor this phone's registration.
    pub async fn delete_server_account(&self, flow: String) -> Result<(), CoreError> {
        let path = "/v1/account/delete";
        let action = "Delete your account on the server";
        let deleted = self.account_op("delete_server_account", Some(action), flow, path, json!({}));
        if deleted.await? {
            self.forget_account_token();
        }
        Ok(())
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
        let action = "Change the server's options";
        self.account_op("set_server_options", Some(action), flow, path, settings)
            .await
            .map(drop)
    }

    /// Drops the sign-in history; false when the server doesn't offer it yet.
    pub async fn clear_server_sign_ins(&self, flow: String) -> Result<bool, CoreError> {
        let path = "/v1/account/clear-sign-ins";
        let action = "Clear the server's sign-in history";
        self.account_op("clear_server_sign_ins", Some(action), flow, path, json!({}))
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
                crate::json_set(&mut body, "registrationNumber", json!(regno));
            }
            let mailbox = self
                .saved_mailbox()
                .filter(|(on, ..)| *on == server)
                .map(|(_, id, _)| id);
            if let Some(id) = &mailbox {
                crate::json_set(&mut body, "mailbox", json!(id));
            }
            let path = "/v1/devices/delete";
            if !supported(self.edge_post(&server, path, &flow, &body).await, path)? {
                return Ok(false);
            }
            self.forget_device_id(&server);
            self.forget_account_token();
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
        action: &str,
    ) -> Result<ServerStatus, CoreError> {
        let account = self
            .account_post(server, "/v1/account", flow, action, true, json!({}))
            .await?
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
                        let phones: Vec<MailPhoneView> = serde_json::from_value(
                            list.get("devices").cloned().unwrap_or_default(),
                        )
                        .map_err(unexpected)?;
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

    /// A `/v1/account/*` call with the account token or the live session added to `body`, the
    /// session once the student allowed it for `action` (asked: `Some`, else only when
    /// remembered); false when the server has no such route yet.
    async fn account_op(
        &self,
        name: &str,
        action: Option<&str>,
        flow: String,
        path: &str,
        body: Value,
    ) -> Result<bool, CoreError> {
        traced(name, &flow, async {
            let (why, ask) = (action.unwrap_or(name), action.is_some());
            let server = self.forwarding_server();
            let answer = self.account_post(&server, path, &flow, why, ask, body);
            let done = supported(answer.await?, path)?;
            if done {
                info!(path, "server account changed");
            }
            Ok(done)
        })
        .await
    }
}

impl Core {
    /// [`Core::save_server_password`] against `server`, as an account call for `action` (the
    /// token, else the session once allowed, asking as `ask` says).
    async fn save_password_with(
        &self,
        server: &str,
        flow: &str,
        action: &str,
        ask: bool,
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
            crate::json_set(
                &mut credentials,
                "gmail",
                json!({
                    "refreshToken": grant.refresh_token,
                    "clientId": grant.client_id,
                    "clientSecret": grant.client_secret,
                    "deleteAfterReading": grant.delete_after_reading,
                }),
            );
        }
        let body = json!({ "label": label(), "credentials": credentials });
        let answer = self
            .account_post(server, "/v1/link", flow, action, ask, body)
            .await?
            .map_err(refused)?;
        if answer.get("savedCredentials") != Some(&json!(true)) {
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
mod tests;
