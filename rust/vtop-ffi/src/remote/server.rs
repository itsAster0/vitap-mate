//! The Server page (Settings → Account): what the server keeps for this student and the phones
//! linked to the account (the contract's `account:*`, `devices:*`, `app:*` and `mail:*`). Every
//! call goes as the student this install proved to be ([`Core::account_call`]); the student
//! comes from the JWT, never from the arguments. Cookies are never logged.

use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

use crate::device::EdgeError;
use crate::remote::backend::Kind;
use crate::remote::consent::{self, CookieKind};
use crate::remote::keys::{AccessKey, KeyView, refused};
use crate::remote::server_code::ServerCode;
use crate::{Core, CoreError, traced};

/// This phone's name on the server: its model, or `vitapmate` when the app gave none.
pub(crate) fn label(shared: &crate::Shared) -> &str {
    shared.label.as_deref().unwrap_or("vitapmate")
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
    /// The short id the server removes it by.
    pub id: String,
    pub label: Option<String>,
    pub added_at: i64,
    pub seen_at: i64,
    pub this_phone: bool,
}

/// How the server serves sessions to connected apps (`account:setSettings`).
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
    /// The first 8 characters of its id; None while it has no identity on the server.
    pub id: Option<String>,
    pub label: String,
    pub push_token: bool,
    /// From the server; None when it doesn't say (before `devices:info`).
    pub registered_at: Option<i64>,
}

/// Who saved the server's sealed VTOP session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SessionSource {
    Phone,
    Server,
}

/// The sealed VTOP session the server keeps for connected apps.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ServerSession {
    None,
    Saved { at: i64, by: SessionSource },
}

/// The sealed VTOP password the server keeps to sign in by itself.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ServerVault {
    None,
    /// `expires_at` None: kept until VTOP rejects it or the student forgets it.
    Saved {
        at: i64,
        expires_at: Option<i64>,
        gmail: bool,
    },
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
    pub session: ServerSession,
    pub vault: ServerVault,
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

impl StatusView {
    fn session(&self) -> ServerSession {
        match self.session_saved_at {
            None => ServerSession::None,
            Some(at) => ServerSession::Saved {
                at,
                // A server that doesn't say: the phone's, as most are.
                by: match self.session_source.as_deref() {
                    Some("server") => SessionSource::Server,
                    _ => SessionSource::Phone,
                },
            },
        }
    }

    fn vault(&self) -> ServerVault {
        match self.vault_saved_at {
            None => ServerVault::None,
            Some(at) => ServerVault::Saved {
                at,
                expires_at: self.vault_expires_at,
                gmail: self.vault_gmail,
            },
        }
    }
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

/// The server has no such function (it predates it): `Ok(false)`.
fn supported(answer: Result<Value, EdgeError>, path: &str) -> Result<bool, CoreError> {
    match answer {
        Err(error) if error.is(&ServerCode::NotFound) => {
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
            self.status_with(&flow, action).await
        })
        .await
    }

    /// Removes a linked phone (an id from [`ServerStatus::phones`]).
    pub async fn remove_phone(&self, flow: String, id: String) -> Result<(), CoreError> {
        let path = "account:removeDevice";
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
    /// nothing while no semester is picked, or (it runs unasked, never proving) before this
    /// install has a server identity; the identity sends it once it comes
    /// ([`Core::semester_to_server`]).
    pub async fn send_server_semester(&self, flow: String) -> Result<(), CoreError> {
        let Some(semester) = self.read_kept::<String>(crate::SELECTED) else {
            return Ok(());
        };
        let path = "account:setSemester";
        let body = json!({ "semesterId": semester });
        match self
            .account_op("send_server_semester", None, flow, path, body)
            .await
        {
            Err(CoreError::ConsentDenied | CoreError::RegnoNotShared) => Ok(()),
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
            self.save_password_with(&flow, action, true, gmail).await
        })
        .await
    }

    /// Sends the password again with the OTP auto-fill now in use ([`GmailGrant`] for Gmail,
    /// none for forwarding), only when the server already keeps one: the student agreed to that
    /// when saving it, so it isn't asked again. False when it keeps none, or there's no identity
    /// to send it with (no proof is started for it).
    pub async fn resend_server_password(
        &self,
        flow: String,
        gmail: Option<GmailGrant>,
    ) -> Result<bool, CoreError> {
        traced("resend_server_password", &flow, async {
            let action = "Send your password to the server again";
            let account = self
                .account_call(&flow, Kind::Query, "account:view", action, false, json!({}))
                .await;
            let account = match account {
                Err(CoreError::ConsentDenied | CoreError::RegnoNotShared) => return Ok(false),
                account => account?.map_err(refused)?,
            };
            if account.get("savedCredentials") != Some(&json!(true)) {
                return Ok(false);
            }
            match self.save_password_with(&flow, action, true, gmail).await {
                Err(CoreError::ConsentDenied | CoreError::RegnoNotShared) => Ok(false),
                saved => saved.map(|()| true),
            }
        })
        .await
    }

    /// Drops the sealed password; the server can then no longer sign in by itself.
    pub async fn forget_server_password(&self, flow: String) -> Result<(), CoreError> {
        let path = "account:forgetCredentials";
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

    /// Signs this install out of the server, before [`Core::sign_out`]: off its forwarding
    /// address and the ones queued to be left (leaving needs the identity), then `auth:signOut`
    /// (for every phone on the account with `everywhere`), then forgets the identity here. Best
    /// effort: offline, an address stays queued and the install stays listed in Server.
    pub async fn sign_out_of_server(
        &self,
        flow: String,
        everywhere: bool,
    ) -> Result<(), CoreError> {
        traced("sign_out_of_server", &flow, async {
            self.leave_with_identity(&flow).await;
            if let Some((_, id, _)) = self.saved_mailbox() {
                let body = json!({ "id": id });
                match self
                    .device_call(&flow, Kind::Mutation, "mail:leave", body)
                    .await
                {
                    Ok(_) => self.forget_forwarding(&id),
                    Err(error) if error.is(&ServerCode::MailboxNotFound) => {
                        self.forget_forwarding(&id);
                    }
                    Err(error) => info!(code = error.code(), "address left at sign-out: queued"),
                }
            }
            let body = json!({ "everywhere": everywhere });
            match self
                .device_call(&flow, Kind::Mutation, "auth:signOut", body)
                .await
            {
                Ok(_) => {
                    info!(everywhere, "signed out of the server");
                    self.forget_identity();
                }
                Err(error) if crate::remote::backend::signed_out_already(&error) => {
                    info!(code = error.code(), "already signed out of the server");
                    self.forget_identity();
                }
                // Offline: the refresh token is kept aside, and the next leave_old_forwarding
                // signs this install out with it, so it stops getting this student's OTPs and
                // session requests.
                Err(error) => {
                    info!(
                        code = error.code(),
                        "server sign-out not sent: kept for later"
                    );
                    self.set_aside_identity();
                }
            }
            Ok(())
        })
        .await
    }

    /// Deletes the account on the server: phones, keys, saved session and password, options,
    /// sign-ins and the forwarding address (it belongs to the account). Not this phone's
    /// registration.
    pub async fn delete_server_account(&self, flow: String) -> Result<(), CoreError> {
        let path = "account:remove";
        let action = "Delete your account on the server";
        let deleted = self.account_op("delete_server_account", Some(action), flow, path, json!({}));
        if deleted.await? {
            self.forget_identity();
            self.forget_any_forwarding();
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
        let path = "account:setSettings";
        let action = "Change the server's options";
        self.account_op("set_server_options", Some(action), flow, path, settings)
            .await
            .map(drop)
    }

    /// Drops the sign-in history; false when the server doesn't offer it yet.
    pub async fn clear_server_sign_ins(&self, flow: String) -> Result<bool, CoreError> {
        let path = "account:clearSignIns";
        let action = "Clear the server's sign-in history";
        self.account_op("clear_server_sign_ins", Some(action), flow, path, json!({}))
            .await
    }

    /// Drops the sealed VTOP session the server keeps for the extension and MCP.
    pub async fn clear_server_session(&self, flow: String) -> Result<(), CoreError> {
        traced("clear_server_session", &flow, async {
            let action = "Drop the session the server keeps";
            let path = "app:sessionClear";
            self.account_call(&flow, Kind::Mutation, path, action, true, json!({}))
                .await?
                .map_err(refused)?;
            info!("server session cleared");
            Ok(())
        })
        .await
    }

    /// Takes another phone (a [`MailPhone::id`]) off this phone's forwarding address.
    pub async fn remove_forwarding_phone(&self, flow: String, id: String) -> Result<(), CoreError> {
        traced("remove_forwarding_phone", &flow, async {
            let (_, mailbox, _) = self
                .saved_mailbox()
                .ok_or_else(|| CoreError::ForwardingOff {
                    reason: "Mail forwarding is off.".into(),
                })?;
            let body = json!({ "id": mailbox, "removeDevice": id });
            self.device_call(&flow, Kind::Mutation, "mail:removeDevice", body)
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

    /// Deletes this install on the server (keys, push token), unlinks it from the account
    /// and takes it off its forwarding address there, then forgets its identity, so the next call
    /// proves afresh. False when the server doesn't offer it yet.
    pub async fn delete_server_device(&self, flow: String) -> Result<bool, CoreError> {
        traced("delete_server_device", &flow, async {
            let server = self.forwarding_server();
            if self.device().refresh.is_none() {
                return Ok(true);
            }
            let mut body = json!({});
            let mailbox = self
                .saved_mailbox()
                .filter(|(on, ..)| *on == server)
                .map(|(_, id, _)| id);
            if let Some(id) = &mailbox {
                crate::json_set(&mut body, "mailbox", json!(id));
            }
            let path = "devices:remove";
            let removed = self.device_call(&flow, Kind::Mutation, path, body).await;
            if !supported(removed, path)? {
                return Ok(false);
            }
            self.forget_identity();
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

/// `screen:view`'s arguments: the same for the read and the live watch, so the read is answered
/// from the watch.
pub(crate) fn screen_args(mailbox: Option<&str>) -> Value {
    mailbox.map_or_else(|| json!({}), |id| json!({ "mailbox": id }))
}

impl Core {
    /// `screen:view`: the account, this install, what the server keeps once this phone is
    /// linked, and the phones on its address when it has one; one read, which the Server
    /// screen's live watch ([`LiveTopic::Server`]) answers locally.
    async fn status_with(&self, flow: &str, action: &str) -> Result<ServerStatus, CoreError> {
        let saved = self.saved_mailbox();
        let screen = self
            .account_call(
                flow,
                Kind::Query,
                "screen:view",
                action,
                true,
                screen_args(saved.as_ref().map(|(_, id, _)| id.as_str())),
            )
            .await?
            .map_err(refused)?;
        let part = |key: &str| screen.get(key).cloned().unwrap_or(Value::Null);
        let account: AccountView = serde_json::from_value(part("account")).map_err(unexpected)?;
        let device: DeviceView = serde_json::from_value(part("device")).unwrap_or_default();
        let status: StatusView = match part("status") {
            Value::Null => StatusView::default(),
            status => serde_json::from_value(status).map_err(unexpected)?,
        };
        let on = part("mailbox");
        let mailbox = match saved {
            None => None,
            Some((_, id, _)) if on.get("gone") == Some(&json!(true)) => {
                self.forget_forwarding(&id);
                None
            }
            Some((_, _, address)) => {
                let phones: Vec<MailPhoneView> =
                    serde_json::from_value(on.get("devices").cloned().unwrap_or_default())
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
        };
        let (id, _) = self.registration();
        let id = self.device().refresh.map(|_| id);
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
                label: device
                    .label
                    .unwrap_or_else(|| label(&self.shared).to_string()),
                // What the server holds: a token the phone has but never sent can't wake it.
                push_token: device.push_token,
                registered_at: device.created_at,
            },
            session: status.session(),
            vault: status.vault(),
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

    /// An `account:*` mutation as the student, proving first only when asked (`action` is
    /// `Some`); false when the server has no such function yet.
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
            let answer = self.account_call(&flow, Kind::Mutation, path, why, ask, body);
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
    /// The picked semester, sent with a JWT just taken: a pick made before this install had a
    /// server identity never reached it. Best effort; the demo account refuses it.
    pub(crate) async fn semester_to_server(&self, flow: &str) {
        let Some(semester) = self.read_kept::<String>(crate::SELECTED) else {
            return;
        };
        let body = json!({ "semesterId": semester });
        if let Err(error) = self
            .jwt_call(flow, Kind::Mutation, "account:setSemester", body)
            .await
        {
            info!(code = error.code(), "semester not sent");
        }
    }

    /// [`Core::save_server_password`], as an account call for `action` (proving first only
    /// when `ask`).
    async fn save_password_with(
        &self,
        flow: &str,
        action: &str,
        ask: bool,
        gmail: Option<GmailGrant>,
    ) -> Result<(), CoreError> {
        // The password leaves the phone whatever carries the call (a kept JWT or a proof,
        // neither of which asks). Asked, the Server screen's own "Let the server sign in for
        // you?" is the consent; unasked, only a remembered yes sends.
        if !ask {
            let host = consent::host(&self.forwarding_server());
            if !consent::allowed(
                &self.shared,
                flow,
                CookieKind::Account,
                action,
                &host,
                false,
            )
            .await
            {
                return Err(CoreError::ConsentDenied);
            }
        }
        let account = (self.read_account())
            .and_then(crate::store::account::Saved::usable)
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
        let body = json!({ "label": label(&self.shared), "credentials": credentials });
        let path = "account:saveCredentials";
        let answer = self
            .account_call(flow, Kind::Action, path, action, ask, body)
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
