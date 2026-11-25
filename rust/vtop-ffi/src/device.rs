//! This install as the server knows it: an Ed25519 seed that signs `signed` calls
//! (`vtop_shared::device_sig`; the server's id for it is derived from its key), the X25519 key
//! forwarded OTPs are sealed to, the refresh token `auth:token` gave it, and the FCM token. All in
//! `device.json`; nothing of it is ever logged. Calls go through [`crate::remote::backend`].

use std::sync::{MutexGuard, PoisonError};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::info;
use vtop_shared::{device_sig, mail_seal};

use crate::remote::backend::{Kind, Refresh};
use crate::remote::server_code::ServerCode;
use crate::{Core, CoreError, traced, unix_now};

const FILE: &str = "device.json";

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
    Status { code: ServerCode, message: String },
}

impl EdgeError {
    /// The code as the server spelt it ("offline" with no answer), for the logs.
    pub(crate) fn code(&self) -> &str {
        match self {
            Self::Offline => "offline",
            Self::Status { code, .. } => code.as_str(),
        }
    }

    /// Whether the server answered with `code`.
    pub(crate) fn is(&self, code: &ServerCode) -> bool {
        matches!(self, Self::Status { code: got, .. } if got == code)
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
        let mut kept = self.kept_device();
        if let Some(device) = &*kept {
            return device.clone();
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
        *kept = Some(device.clone());
        device
    }

    /// `device.json` as last read or written, read on first use; the guard holds the file's lock.
    fn kept_device(&self) -> MutexGuard<'_, Option<Device>> {
        let mut kept = self
            .shared
            .locks
            .device
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if kept.is_none() {
            *kept = self.read::<Device>(FILE);
        }
        kept
    }

    fn update_device(&self, change: impl FnOnce(&mut Device)) {
        let mut kept = self.kept_device();
        if let Some(device) = &mut *kept {
            change(device);
            self.write_plain(FILE, &*device);
        }
    }

    /// This install's id on the server (derived from its key) and whether it has an FCM token.
    pub(crate) fn registration(&self) -> (String, bool) {
        let device = self.device();
        let key = device_sig::public_key(&device.seed).unwrap_or_default();
        let id = device_sig::device_id(&key).unwrap_or_default();
        (id, device.push_token.is_some())
    }

    /// Keeps (or, None, drops) the refresh token. Clearing it does not create `device.json` if
    /// none exists yet.
    pub(crate) fn keep_refresh(&self, refresh: Option<Refresh>) {
        if refresh.is_some() {
            self.device();
        }
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
                Err(error) if error.is(&ServerCode::Unauthenticated) => Ok(()),
                sent => sent.map(drop).map_err(CoreError::from),
            }
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("vtop-ffi-device-{name}-{}", std::process::id()));
        // A leftover from an earlier run, if any.
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn device_json_keeps_its_format() {
        let file = r#"{"seed":"c2VlZA","seal_secret":"c2Vj","seal_key":"a2V5","refresh":{"server":"https://vmate.aster0.dev","token":"tok","regno":"22BCE0001"},"push_token":"fcm","clock_offset":-3}"#;
        let device: Device = serde_json::from_str(file).unwrap();
        assert_eq!(serde_json::to_string(&device).unwrap(), file);
        // From before the refresh token, the FCM token and the clock offset.
        let old: Device =
            serde_json::from_str(r#"{"seed":"s","seal_secret":"x","seal_key":"k"}"#).unwrap();
        assert_eq!(
            serde_json::to_string(&old).unwrap(),
            r#"{"seed":"s","seal_secret":"x","seal_key":"k","refresh":null,"push_token":null,"clock_offset":0}"#
        );
    }

    #[test]
    fn the_device_is_read_once_and_kept_per_core() {
        let dir = dir("kept");
        let core = Core::for_test(&dir);
        let seed = core.device().seed;
        core.set_clock_offset(42);
        // Every signed call asks for the clock: it comes from memory, not the file.
        fs::write(dir.join(FILE), "not json").unwrap();
        assert_eq!(core.server_now() - unix_now(), 42);
        assert_eq!(core.device().seed, seed);
        core.set_clock_offset(7);

        // Another core reads what this one wrote.
        let other = Core::for_test(&dir);
        let device = other.device();
        assert!(device.seed == seed && device.clock_offset == 7);
        let _ = fs::remove_dir_all(&dir);
    }
}
