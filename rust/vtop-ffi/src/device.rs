//! This install as the edge knows it: an Ed25519 seed that signs every request
//! (`vtop_shared::device_sig`), the X25519 key forwarded OTPs are sealed to, the id each server
//! gave it, and the FCM token. All in `device.json`; nothing of it is ever logged. Every call to
//! the edge goes through [`Core::edge_post`].

use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use tracing::{info, warn};
use vtop_core::client::Hop;
use vtop_shared::{device_sig, mail_seal, page_dictionary};

use crate::{Core, CoreError, traced, transport, unix_now};

const FILE: &str = "device.json";
const TIMEOUT: Duration = Duration::from_secs(10);
const VTOP_TIMEOUT: Duration = Duration::from_secs(35);

/// Serialises read-modify-write of [`FILE`].
static LOCK: Mutex<()> = Mutex::new(());

/// No `Debug`: it holds the private keys.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Device {
    pub(crate) seed: String,
    pub(crate) seal_secret: String,
    pub(crate) seal_key: String,
    /// Server URL → this install's id there.
    #[serde(default)]
    ids: BTreeMap<String, String>,
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
}

impl From<EdgeError> for CoreError {
    fn from(error: EdgeError) -> Self {
        match error {
            EdgeError::Offline => Self::Offline,
            EdgeError::Status { message, .. } => Self::Other { reason: message },
        }
    }
}

pub(crate) struct Answer {
    status: u16,
    pub body: Value,
    /// A page the engine packed with the page dictionary (`body` is then null).
    pub packed: Option<Vec<u8>>,
}

impl Answer {
    fn text(&self, key: &str) -> Option<String> {
        self.body[key].as_str().map(str::to_string)
    }

    fn error(&self) -> EdgeError {
        let status = self.status;
        EdgeError::Status {
            code: self
                .text("code")
                .unwrap_or_else(|| format!("http_{status}")),
            message: self
                .text("error")
                .unwrap_or_else(|| format!("The server answered HTTP {status}.")),
        }
    }
}

/// POSTs JSON through the app's transport, with the flow as `x-request-id` and `headers`.
async fn send(
    server: &str,
    path: &str,
    flow: &str,
    body: &[u8],
    headers: Vec<(String, String)>,
) -> Result<Answer, EdgeError> {
    let offline = |error: &dyn std::fmt::Display| {
        warn!(path, error = %error, "edge unreachable");
        EdgeError::Offline
    };
    let transport = transport::current().map_err(|error| offline(&error))?;
    let mut all = vec![
        ("content-type".into(), "application/json".into()),
        ("x-request-id".into(), flow.into()),
    ];
    all.extend(headers);
    // The engine signs in or fetches from VTOP behind `/v1/vtop/` (the edge gives up at 32 s).
    let timeout = if path.starts_with("/v1/vtop/") {
        VTOP_TIMEOUT
    } else {
        TIMEOUT
    };
    let hop = Hop {
        method: "POST".into(),
        url: format!("{}{path}", server.trim_end_matches('/')),
        headers: all,
        body: body.to_vec(),
        timeout,
        connect_timeout: TIMEOUT,
        download_to: None,
    };
    let response = transport.send(hop).await.map_err(|error| offline(&error))?;
    let status = response.status;
    let packed = response.header("content-type") == Some(page_dictionary::CONTENT_TYPE);
    let bytes = response.bytes().await.ok();
    if packed {
        return Ok(Answer {
            status,
            body: Value::Null,
            packed: bytes,
        });
    }
    // 204 has no body.
    let body = bytes
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(Value::Null);
    Ok(Answer {
        status,
        body,
        packed: None,
    })
}

/// The vm1 headers; `id` is None for registration, which the new key signs on its own.
fn signed(
    seed: &str,
    id: Option<&str>,
    path: &str,
    time: i64,
    body: &[u8],
) -> Vec<(String, String)> {
    let sig = device_sig::sign(seed, "POST", path, time, body).unwrap_or_default();
    let mut headers = vec![
        (device_sig::TIME.into(), time.to_string()),
        (device_sig::SIG.into(), sig),
    ];
    if let Some(id) = id {
        headers.push((device_sig::DEVICE.into(), id.into()));
    }
    headers
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
            ids: BTreeMap::new(),
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

    /// This install's id on `server` (None before it registered there), and whether it has an
    /// FCM token.
    pub(crate) fn registration(&self, server: &str) -> (Option<String>, bool) {
        let device = self.device();
        (device.ids.get(server).cloned(), device.push_token.is_some())
    }

    /// Forgets this install's id on `server` (the edge deleted it); the next call registers
    /// afresh.
    pub(crate) fn forget_device_id(&self, server: &str) {
        self.update_device(|d| {
            d.ids.remove(server);
        });
    }

    /// This install's id on `server`, registering it there first if needed.
    pub(crate) async fn device_id(&self, server: &str, flow: &str) -> Result<String, EdgeError> {
        let device = self.device();
        if let Some(id) = device.ids.get(server) {
            return Ok(id.clone());
        }
        let path = "/v1/devices/register";
        let sign_key = device_sig::public_key(&device.seed).unwrap_or_default();
        let mut body = json!({
            "signKey": sign_key,
            "sealKey": device.seal_key,
            "label": crate::server::label(),
            "platform": "android",
        });
        if let Some(token) = &device.push_token {
            body["fcmToken"] = json!(token);
        }
        let bytes = serde_json::to_vec(&body).unwrap_or_default();
        let answer = self.signed_post(server, path, flow, &bytes, None).await?;
        if !(200..300).contains(&answer.status) {
            return Err(answer.error());
        }
        let id = answer.text("deviceId").ok_or_else(|| EdgeError::Status {
            code: "bad_answer".into(),
            message: "The server sent an unexpected answer.".into(),
        })?;
        self.update_device(|d| {
            d.ids.insert(server.to_string(), id.clone());
        });
        info!("device registered");
        Ok(id)
    }

    /// A signed POST to the edge as this install. A `stale_time` answer corrects the clock offset
    /// and retries once; `unknown_device` registers again and retries once.
    pub(crate) async fn edge_post(
        &self,
        server: &str,
        path: &str,
        flow: &str,
        body: &Value,
    ) -> Result<Value, EdgeError> {
        Ok(self.edge_answer(server, path, flow, body).await?.body)
    }

    /// As [`Self::edge_post`], with a packed page kept as it came.
    pub(crate) async fn edge_answer(
        &self,
        server: &str,
        path: &str,
        flow: &str,
        body: &Value,
    ) -> Result<Answer, EdgeError> {
        let bytes = serde_json::to_vec(body).unwrap_or_default();
        for attempt in 0..2 {
            let id = self.device_id(server, flow).await?;
            let answer = self
                .signed_post(server, path, flow, &bytes, Some(&id))
                .await?;
            if (200..300).contains(&answer.status) {
                return Ok(answer);
            }
            let error = answer.error();
            if error.code() == "unknown_device" && attempt == 0 {
                self.update_device(|d| {
                    d.ids.remove(server);
                });
                continue;
            }
            return Err(error);
        }
        Err(EdgeError::Offline)
    }

    /// Sends `bytes` signed with this install's seed, as `id` (None: registering). A
    /// `stale_time` answer sets the clock offset from the edge's `serverTime` and retries once.
    async fn signed_post(
        &self,
        server: &str,
        path: &str,
        flow: &str,
        bytes: &[u8],
        id: Option<&str>,
    ) -> Result<Answer, EdgeError> {
        let mut retried = false;
        loop {
            let device = self.device();
            let time = unix_now() + device.clock_offset;
            let headers = signed(&device.seed, id, path, time, bytes);
            let answer = send(server, path, flow, bytes, headers).await?;
            if (200..300).contains(&answer.status) {
                return Ok(answer);
            }
            let code = answer.text("code").unwrap_or_default();
            info!(
                path,
                status = answer.status,
                code = code.as_str(),
                "edge refused"
            );
            match answer.body["serverTime"].as_i64() {
                Some(server_time) if code == "stale_time" && !retried => {
                    self.update_device(|d| d.clock_offset = server_time - unix_now());
                    retried = true;
                }
                _ => return Ok(answer),
            }
        }
    }
}

#[uniffi::export]
impl Core {
    /// Saves the FCM token and sends it to every server this install is registered with.
    pub async fn set_push_token(&self, flow: String, token: String) -> Result<(), CoreError> {
        traced("set_push_token", &flow, async {
            self.device();
            self.update_device(|d| d.push_token = Some(token.clone()));
            let servers: Vec<String> = self.device().ids.keys().cloned().collect();
            for server in servers {
                self.edge_post(
                    &server,
                    "/v1/devices/update",
                    &flow,
                    &json!({ "fcmToken": token }),
                )
                .await?;
            }
            Ok(())
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// Against the deployed edge: `cargo test -p vtop-ffi live_edge -- --ignored`. Registers a
    /// throwaway install (no mailbox, no mail).
    #[tokio::test]
    #[ignore = "needs the network and the live edge"]
    async fn live_edge() {
        let server = "https://vmate.aster0.dev";
        let dir = std::env::temp_dir().join(format!("vtop-ffi-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let core = Core {
            cache: dir.clone(),
            dir,
            slot: Mutex::new(None),
        };
        let id = core.device_id(server, "live").await.unwrap();
        assert!(id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()));
        let label = json!({ "label": "vitapmate-live-test" });
        let updated = core
            .edge_post(server, "/v1/devices/update", "live", &label)
            .await;
        assert_eq!(updated.unwrap(), Value::Null);
        let inbox = core
            .edge_post(
                server,
                "/v1/mail-otp/inbox",
                "live",
                &json!({ "id": "zzzzzzzzzz" }),
            )
            .await;
        assert_eq!(inbox.unwrap_err().code(), "mailbox_not_found");
        // Unsigned, and signed with the wrong device id.
        let bytes = label.to_string().into_bytes();
        let unsigned = send(server, "/v1/devices/update", "live", &bytes, vec![])
            .await
            .unwrap();
        assert_eq!(unsigned.status, 401);
        assert_eq!(unsigned.text("code").as_deref(), Some("bad_signature"));
        let seed = core.device().seed;
        let headers = signed(
            &seed,
            Some(&"0".repeat(32)),
            "/v1/devices/update",
            unix_now(),
            &bytes,
        );
        let stranger = send(server, "/v1/devices/update", "live", &bytes, headers)
            .await
            .unwrap();
        assert_eq!(stranger.status, 404);
        // A phone clock 15 minutes slow: corrected from `serverTime`, then accepted.
        core.update_device(|d| d.clock_offset = -900);
        let updated = core
            .edge_post(server, "/v1/devices/update", "live", &label)
            .await;
        assert_eq!(updated.unwrap(), Value::Null);
        assert!(core.device().clock_offset.abs() < 60);
    }
}
