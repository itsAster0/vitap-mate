//! The student's server account as the app shows it (Account, Connected apps, the OTP auto-fill
//! sheet, Developer): who the server knows, the phones linked to the account, what it stores for
//! connected apps ([`ServerStorage`]), its sign-ins, this install's registration and the phones on
//! the forwarding address (the contract's `account:*`, `devices:*`, `app:*` and `mail:*`). Every
//! call goes as the student this install proved to be ([`Core::account_call`]); the student
//! comes from the JWT, never from the arguments. Answers are parsed once here into the records
//! below. Cookies are never logged.

use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

use crate::device::EdgeError;
use crate::remote::backend::Kind;
use crate::remote::consent::{self, CookieKind};
use crate::remote::keys::{KeyView, refused};
use crate::remote::server_code::ServerCode;
use crate::{Core, CoreError, ForwardingOffKind, traced};

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
    pub outcome: SignInOutcome,
}

/// How a [`ServerSignIn`] went.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum SignInOutcome {
    SignedIn,
    /// With VTOP's reason when the server kept one.
    Failed {
        reason: Option<String>,
    },
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

/// How the server uses a saved password (`account:setSettings`); only with one saved.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct VaultOptions {
    /// Sign in with the saved password at once, without asking a phone first.
    pub always_use_vault: bool,
    /// Otherwise, how long a phone gets to answer first: 10, 20 or 45.
    pub phone_wait_secs: u32,
    /// How long the saved password is kept: 86400, 172800, or None (until VTOP rejects it).
    pub vault_ttl_secs: Option<i64>,
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

/// This install as the server knows it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ServerDevice {
    /// No identity on the server yet (never proved, or removed): nothing to show or remove.
    Unregistered { label: String },
    Registered {
        /// The first 8 characters of its id.
        id: String,
        label: String,
        /// The server holds a push token, so it can wake this phone for a connected app.
        push_token: bool,
        /// None from a server that doesn't say.
        registered_at: Option<i64>,
    },
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
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SavedVault {
    pub at: i64,
    /// None: kept until VTOP rejects it or the student forgets it.
    pub expires_at: Option<i64>,
    /// It carries Gmail access to read the sign-in code.
    pub gmail: bool,
}

/// What the server stores of the student's VTOP for connected apps (`settings.cacheSession`
/// and what it keeps under it). Off by default: a connected app then gets a session only while
/// a phone answers, and the server keeps no copy.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ServerStorage {
    Off,
    /// Sessions phones hand over are kept (sealed) for a week, for connected apps.
    SessionOnly {
        session: ServerSession,
    },
    /// A saved password too, so the server signs in when no phone answers.
    OfflineSignIn {
        session: ServerSession,
        vault: SavedVault,
        options: VaultOptions,
    },
}

/// Whether this phone is one of the account's phones (asked for sessions, sees the others).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum ServerLink {
    /// Not one of them (after a reinstall or a removal): the server shows it no phones or keys.
    Unlinked,
    Linked {
        phones: Vec<LinkedPhone>,
    },
}

/// The student's account on the server (`account:view`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ServerAccount {
    pub registration_number: String,
    /// When the server made the account (None from servers that don't say).
    pub created_at: Option<i64>,
    /// The semester connected apps read; None is the newest.
    pub semester_id: Option<String>,
    pub link: ServerLink,
    pub storage: ServerStorage,
    /// The student has a forwarding address, on this phone or another.
    pub forwarding: bool,
    /// When VTOP last rejected the saved password on the server.
    pub credentials_rejected_at: Option<i64>,
}

/// `account:view`'s answer as the server sends it (older servers leave fields out), read once
/// into [`ServerAccount`] here and into Connected apps' keys in `keys.rs`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AccountWire {
    #[serde(default)]
    registration_number: String,
    created_at: Option<i64>,
    pub(crate) semester_id: Option<String>,
    #[serde(default)]
    settings: SettingsWire,
    vault_saved_at: Option<i64>,
    vault_expires_at: Option<i64>,
    #[serde(default)]
    vault_gmail: bool,
    session_saved_at: Option<i64>,
    session_source: Option<String>,
    #[serde(default)]
    forwarding: bool,
    credentials_rejected_at: Option<i64>,
    #[serde(default)]
    pub(crate) this_phone: bool,
    #[serde(default)]
    pub(crate) phones: Vec<PhoneWire>,
    #[serde(default)]
    pub(crate) keys: Vec<KeyView>,
}

/// `settings` in [`AccountWire`]; `cacheSession` is absent from servers before storage became a
/// choice.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SettingsWire {
    cache_session: Option<bool>,
    always_use_vault: bool,
    phone_wait_secs: u32,
    vault_ttl_secs: Option<i64>,
}

impl Default for SettingsWire {
    fn default() -> Self {
        Self {
            cache_session: None,
            always_use_vault: true,
            phone_wait_secs: 20,
            vault_ttl_secs: None,
        }
    }
}

impl SettingsWire {
    fn options(&self) -> VaultOptions {
        VaultOptions {
            always_use_vault: self.always_use_vault,
            phone_wait_secs: self.phone_wait_secs,
            vault_ttl_secs: self.vault_ttl_secs,
        }
    }
}

/// One of the account's phones in [`AccountWire`].
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PhoneWire {
    pub(crate) id: String,
    pub(crate) label: Option<String>,
    #[serde(default)]
    added_at: i64,
    pub(crate) seen_at: i64,
    #[serde(default)]
    pub(crate) this_phone: bool,
}

impl AccountWire {
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

    fn vault(&self) -> Option<SavedVault> {
        self.vault_saved_at.map(|at| SavedVault {
            at,
            expires_at: self.vault_expires_at,
            gmail: self.vault_gmail,
        })
    }

    pub(crate) fn storage(&self) -> ServerStorage {
        let session = self.session();
        let vault = self.vault();
        // A server from before the choice stored whatever it had: on when it keeps something.
        let on = self
            .settings
            .cache_session
            .unwrap_or(vault.is_some() || session != ServerSession::None);
        match (on, vault) {
            (false, _) => ServerStorage::Off,
            (true, None) => ServerStorage::SessionOnly { session },
            (true, Some(vault)) => ServerStorage::OfflineSignIn {
                session,
                vault,
                options: self.settings.options(),
            },
        }
    }

    fn into_account(self) -> ServerAccount {
        let storage = self.storage();
        let link = if self.this_phone {
            ServerLink::Linked {
                phones: self
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
            }
        } else {
            ServerLink::Unlinked
        };
        ServerAccount {
            registration_number: self.registration_number,
            created_at: self.created_at,
            semester_id: self.semester_id,
            link,
            storage,
            forwarding: self.forwarding,
            credentials_rejected_at: self.credentials_rejected_at,
        }
    }
}

/// `account:setSettings`' body; without `cache_session` the server keeps what it has (and an
/// older server, which doesn't know the field, still takes it).
fn settings_body(cache_session: Option<bool>, options: &VaultOptions) -> Value {
    let mut settings = json!({
        "alwaysUseVault": options.always_use_vault,
        "phoneWaitSecs": options.phone_wait_secs,
        "vaultTtlSecs": options.vault_ttl_secs,
    });
    if let Some(on) = cache_session {
        crate::json_set(&mut settings, "cacheSession", json!(on));
    }
    json!({ "settings": settings })
}

#[derive(Deserialize)]
struct SignInView {
    at: i64,
    by: String,
    ok: bool,
    reason: Option<String>,
}

impl From<SignInView> for ServerSignIn {
    fn from(view: SignInView) -> Self {
        Self {
            at: view.at,
            by_server: view.by == "server",
            outcome: if view.ok {
                SignInOutcome::SignedIn
            } else {
                SignInOutcome::Failed {
                    reason: view.reason,
                }
            },
        }
    }
}

/// `devices:info`'s answer.
#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct DeviceView {
    label: Option<String>,
    push_token: bool,
    created_at: Option<i64>,
}

/// `mail:devices`' answer.
#[derive(Deserialize)]
struct MailDevicesView {
    devices: Vec<MailPhoneView>,
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

pub(crate) fn unexpected(_: serde_json::Error) -> CoreError {
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

/// Server account calls; each takes the caller's flow id first.
#[uniffi::export]
impl Core {
    /// The student's account on the server: whether this phone is linked and the other phones,
    /// and what the server stores for connected apps. Proves first when there is no identity.
    pub async fn server_account(&self, flow: String) -> Result<ServerAccount, CoreError> {
        traced("server_account", &flow, async {
            let account = self.account_view(&flow, "Load your server account").await?;
            info!(
                linked = account.this_phone,
                phones = account.phones.len(),
                "server account"
            );
            Ok(account.into_account())
        })
        .await
    }

    /// The account's last VTOP sign-ins (phones and the server), newest first, at most 10;
    /// none from a server that doesn't list them yet.
    pub async fn server_sign_ins(&self, flow: String) -> Result<Vec<ServerSignIn>, CoreError> {
        traced("server_sign_ins", &flow, async {
            let (path, action) = ("account:signIns", "Load your recent sign-ins");
            let answer = self
                .account_call(&flow, Kind::Query, path, action, true, json!({}))
                .await?;
            let views: Vec<SignInView> = match answer {
                Err(error) if error.is(&ServerCode::NotFound) => {
                    info!(path, "not on this server yet");
                    Vec::new()
                }
                answer => serde_json::from_value(answer.map_err(refused)?).map_err(unexpected)?,
            };
            info!(count = views.len(), "server sign-ins");
            Ok(views.into_iter().map(ServerSignIn::from).collect())
        })
        .await
    }

    /// This install as the server knows it (Developer). Never proves: with no identity it is
    /// [`ServerDevice::Unregistered`] and nothing is sent.
    pub async fn server_device(&self, flow: String) -> Result<ServerDevice, CoreError> {
        traced("server_device", &flow, async {
            let fallback = label(&self.shared).to_string();
            if self.device().refresh.is_none() {
                return Ok(ServerDevice::Unregistered { label: fallback });
            }
            let answer = self
                .device_call(&flow, Kind::Query, "devices:info", json!({}))
                .await;
            let view: DeviceView = match answer {
                Err(error) if error.is(&ServerCode::Unauthenticated) => {
                    return Ok(ServerDevice::Unregistered { label: fallback });
                }
                answer => serde_json::from_value(answer.map_err(refused)?).map_err(unexpected)?,
            };
            let (id, _) = self.registration();
            Ok(ServerDevice::Registered {
                id: id.chars().take(8).collect(),
                label: view.label.unwrap_or(fallback),
                // What the server holds: a token the phone has but never sent can't wake it.
                push_token: view.push_token,
                registered_at: view.created_at,
            })
        })
        .await
    }

    /// The phones on this phone's forwarding address (at most 5). An address the server no
    /// longer has is forgotten here and answers `ForwardingOff`.
    pub async fn forwarding_phones(&self, flow: String) -> Result<Vec<MailPhone>, CoreError> {
        traced("forwarding_phones", &flow, async {
            let (_, id, _) = self
                .saved_mailbox()
                .ok_or_else(|| CoreError::ForwardingOff {
                    kind: ForwardingOffKind::NotSetUp,
                    reason: "Mail forwarding is off.".into(),
                })?;
            let rejoining = self.forwarding_rejoining();
            let answer = self
                .device_call(&flow, Kind::Query, "mail:devices", json!({ "id": id }))
                .await;
            let view: MailDevicesView = match answer {
                // Kept to rejoin: the server doesn't know this install on it yet, and the
                // rejoin (`keep_forwarding`, a check) tells whether it still exists.
                Err(error)
                    if error.is(&ServerCode::MailboxNotFound)
                        && (rejoining || self.forwarding_rejoining()) =>
                {
                    return Err(CoreError::ForwardingOff {
                        kind: ForwardingOffKind::NeedsSignIn,
                        reason: "Mail forwarding reconnects once you're signed in.".into(),
                    });
                }
                Err(error) if error.is(&ServerCode::MailboxNotFound) => {
                    self.forget_forwarding(&id);
                    return Err(CoreError::ForwardingOff {
                        kind: ForwardingOffKind::AddressDeleted,
                        reason: "The forwarding address was deleted.".into(),
                    });
                }
                answer => serde_json::from_value(answer.map_err(refused)?).map_err(unexpected)?,
            };
            if rejoining && view.devices.iter().any(|p| p.this_device) {
                self.promote_joined(&id);
            }
            info!(count = view.devices.len(), "forwarding phones");
            Ok(view
                .devices
                .into_iter()
                .map(|p| MailPhone {
                    id: p.device_id,
                    label: p.label,
                    added_at: p.added_at,
                    seen_at: p.seen_at,
                    this_phone: p.this_device,
                })
                .collect())
        })
        .await
    }

    /// Removes a linked phone (an id from [`ServerLink::Linked`]).
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
    /// ([`Core::semester_to_server`]). The demo account has no semester there to set.
    pub async fn send_server_semester(&self, flow: String) -> Result<(), CoreError> {
        if self.is_demo() {
            return Ok(());
        }
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

    /// Turns the server's storage for connected apps on or off (`settings.cacheSession`). Off
    /// deletes on the server what it keeps (the session, the password and its sign-in logins)
    /// and its MCP copies; on keeps the sessions phones hand over from then on.
    pub async fn set_server_storage(&self, flow: String, enabled: bool) -> Result<(), CoreError> {
        traced("set_server_storage", &flow, async {
            let action = if enabled {
                "Let the server keep your VTOP session for connected apps"
            } else {
                "Delete what the server keeps of your VTOP"
            };
            // The server takes the settings whole: the password options stay as they are.
            let options = self.account_view(&flow, action).await?.settings.options();
            let body = settings_body(Some(enabled), &options);
            let path = "account:setSettings";
            self.account_call(&flow, Kind::Mutation, path, action, true, body)
                .await?
                .map_err(refused)?;
            info!(enabled, "server storage");
            Ok(())
        })
        .await
    }

    /// Sets how the server uses the saved password (see [`VaultOptions`]); storage stays as it is.
    pub async fn set_vault_options(
        &self,
        flow: String,
        options: VaultOptions,
    ) -> Result<(), CoreError> {
        let path = "account:setSettings";
        let action = "Change how the server uses your password";
        let body = settings_body(None, &options);
        self.account_op("set_vault_options", Some(action), flow, path, body)
            .await
            .map(drop)
    }

    /// Hands the server this phone's VTOP password, sealed, so it signs in for connected apps
    /// while no phone answers (it turns storage on). It needs the sign-in code too: the server
    /// keeps the password only with a forwarding address or [`GmailGrant`], and says so
    /// otherwise.
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

    /// Signs this install out of the server, before [`Core::sign_out`]: stops its live watches,
    /// takes it off its forwarding address and the ones queued to be left (leaving needs the identity), then `auth:signOut`
    /// (for every phone on the account with `everywhere`), then forgets the identity here. Best
    /// effort: offline, an address stays queued and the sign-out (with `everywhere`) is kept for
    /// later, the install listed on the account until it goes.
    pub async fn sign_out_of_server(
        &self,
        flow: String,
        everywhere: bool,
    ) -> Result<(), CoreError> {
        traced("sign_out_of_server", &flow, async {
            self.stop_live_watches();
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
                Err(error) if crate::remote::backend::identity::signed_out_already(&error) => {
                    info!(code = error.code(), "already signed out of the server");
                    self.forget_identity();
                }
                // Offline: the refresh token is kept aside with `everywhere`, and the next
                // leave_old_forwarding (or identity) signs this install out with it, so it stops
                // getting this student's OTPs and session requests.
                Err(error) => {
                    info!(
                        code = error.code(),
                        everywhere, "server sign-out not sent: kept for later"
                    );
                    self.set_aside_sign_out(everywhere);
                }
            }
            Ok(())
        })
        .await
    }

    /// Deletes the account on the server: phones, keys, saved session and password, options,
    /// sign-ins and the forwarding address (it belongs to the account). Not this phone's
    /// registration. Sharing the registration number stops and a server route goes back to
    /// Direct, so the next call doesn't make a new account unasked.
    pub async fn delete_server_account(&self, flow: String) -> Result<(), CoreError> {
        let path = "account:remove";
        let action = "Delete your account on the server";
        let deleted = self.account_op(
            "delete_server_account",
            Some(action),
            flow.clone(),
            path,
            json!({}),
        );
        if deleted.await? {
            self.forget_identity();
            self.forget_account_token();
            self.forget_any_forwarding();
            self.stop_sharing_after_delete(&flow).await;
        }
        Ok(())
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
                    kind: ForwardingOffKind::NotSetUp,
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

impl Core {
    /// `account:view`, read once into [`AccountWire`] (proving first when there's no identity).
    pub(crate) async fn account_view(
        &self,
        flow: &str,
        action: &str,
    ) -> Result<AccountWire, CoreError> {
        let view = self
            .account_call(flow, Kind::Query, "account:view", action, true, json!({}))
            .await?
            .map_err(refused)?;
        serde_json::from_value(view).map_err(unexpected)
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
    /// server identity never reached it. Best effort; skipped for the demo account, which the
    /// server refuses.
    pub(crate) async fn semester_to_server(&self, flow: &str) {
        if self.is_demo() {
            return;
        }
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
        // neither of which asks). Asked, Connected apps' own "Save your password on the
        // server?" is the consent; unasked, only a remembered yes sends.
        if !ask {
            let host = consent::host(&self.forwarding_server());
            if !consent::allowed(
                &self.shared,
                flow,
                CookieKind::Account,
                consent::CookiePayload::CookieAndPassword,
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
