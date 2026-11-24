//! This install as the server knows it: an Ed25519 seed that signs `signed` calls
//! (`vtop_shared::device_sig`; the server's id for it is derived from its key), the X25519 key
//! forwarded OTPs are sealed to, the refresh token `auth:token` gave it, and the FCM token. All in
//! `device.json`; nothing of it is ever logged. Calls go through [`crate::remote::backend`].

use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::info;
use vtop_shared::{device_sig, mail_seal};

use crate::remote::backend::{Kind, Refresh};
use crate::{Core, CoreError, traced, unix_now};

const FILE: &str = "device.json";

/// Serialises read-modify-write of [`FILE`].
static LOCK: Mutex<()> = Mutex::new(());

/// No `Debug`: it holds the private keys.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Device {
    pub(crate) seed: String,
    pub(crate) seal_secret: String,
    pub(crate) seal_key: String,
    #[serde(default)]
    pub(crate) refresh: Option<Refresh>,
    #[serde(default)]
    push_token: Option<String>,
    /// Seconds to add to this phone's clock, learnt from a `stale_time` answer.
    #[serde(default)]
    pub(crate) clock_offset: i64,
}

#[derive(Debug)]
pub(crate) enum EdgeError {
    Offline,
    Status { code: String, message: String },
}

impl EdgeError {
    pub(crate) fn code(&self) -> &str {
        match self {
            Self::Offline => "offline",
            Self::Status { code, .. } => code,
        }
    }

    /// What the server said, for the student.
    pub(crate) fn message(&self) -> &str {
        match self {
            Self::Offline => "The server can't be reached.",
            Self::Status { message, .. } => message,
        }
    }
}

impl From<EdgeError> for CoreError {
    fn from(error: EdgeError) -> Self {
        match error {
            EdgeError::Offline => Self::Offline,
            EdgeError::Status { message, .. } => Self::Other { reason: message },
        }
    }
}

impl Core {
    /// This install's keys, made on first use.
    pub(crate) fn device(&self) -> Device {
        let _guard = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(device) = self.read::<Device>(FILE) {
            return device;
        }
        let (seal_secret, seal_key) = mail_seal::generate_keypair();
        let device = Device {
            seed: device_sig::generate_seed(),
            seal_secret,
            seal_key,
            refresh: None,
            push_token: None,
            clock_offset: 0,
        };
        self.write_plain(FILE, &device);
        info!("device keys made");
        device
    }

    fn update_device(&self, change: impl FnOnce(&mut Device)) {
        let _guard = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(mut device) = self.read::<Device>(FILE) {
            change(&mut device);
            self.write_plain(FILE, &device);
        }
    }

    /// This install's id on the server (derived from its key) and whether it has an FCM token.
    pub(crate) fn registration(&self) -> (String, bool) {
        let device = self.device();
        let key = device_sig::public_key(&device.seed).unwrap_or_default();
        let id = device_sig::device_id(&key).unwrap_or_default();
        (id, device.push_token.is_some())
    }

    /// Keeps (or, None, drops) the refresh token.
    pub(crate) fn keep_refresh(&self, refresh: Option<Refresh>) {
        self.device();
        self.update_device(|d| d.refresh = refresh);
    }

    /// Current unix time in seconds adjusted for the server's clock (`unix_now() + clock_offset`).
    pub(crate) fn server_now(&self) -> i64 {
        unix_now().saturating_add(self.device().clock_offset)
    }

    pub(crate) fn set_clock_offset(&self, offset: i64) {
        self.device();
        self.update_device(|d| d.clock_offset = offset);
    }

    /// `device` for `auth:token`: what the server keeps about this install.
    pub(crate) fn device_info(&self) -> Value {
        let device = self.device();
        let mut info = json!({
            "sealKey": device.seal_key,
            "platform": "android",
            "label": crate::remote::server::label(&self.shared),
        });
        if let Some(token) = &device.push_token {
            crate::json_set(&mut info, "fcmToken", json!(token));
        }
        info
    }
}

#[uniffi::export]
impl Core {
    /// Saves the FCM token and sends it to the server once this install has an identity there.
    pub async fn set_push_token(&self, flow: String, token: String) -> Result<(), CoreError> {
        traced("set_push_token", &flow, async {
            self.device();
            self.update_device(|d| d.push_token = Some(token.clone()));
            let body = json!({ "fcmToken": token });
            match self
                .device_call(&flow, Kind::Mutation, "devices:update", body)
                .await
            {
                // Sent with the device at the next `auth:token`.
                Err(error) if error.code() == "unauthenticated" => Ok(()),
                sent => sent.map(drop).map_err(CoreError::from),
            }
        })
        .await
    }
}
