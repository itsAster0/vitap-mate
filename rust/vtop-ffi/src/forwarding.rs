//! Mail forwarding (the Flutter app's `mail_forwarding_service.dart`): Gmail forwards VTOP's OTP
//! mail to `otp+<id>@<domain>`; the edge Worker reads it, seals the code to this install's
//! X25519 seal key (`device.json`, see [`crate::device`]; `vtop_shared::mail_seal`, the mailbox
//! id as associated data) and keeps the latest box for 5 minutes, which the app pulls from
//! `/v1/mail-otp/inbox`. Every call is signed as this install ([`Core::edge_post`]); the edge
//! knows the mailbox's members by device id, so `forwarding.json` holds no secret. Codes and
//! mail contents are never logged.
//!
//! Only two kinds of mail are relayed: VTOP's OTP and Gmail's forwarding confirmation (sent once,
//! when the student adds the address in Gmail). That confirmation reaching the phone proves the
//! address works.

use std::fs;
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{debug, info, warn};
use vtop_core::{VtopClient, VtopError};
use vtop_shared::mail_seal;

use crate::{Core, CoreError, traced, traced_sync};

pub(crate) const FILE: &str = "forwarding.json";
/// Optional one-line server URL (e.g. `http://127.0.0.1:8787` for `wrangler dev` behind
/// `adb reverse`), read when an address is set up; the address keeps the server it was made on.
pub(crate) const SERVER_FILE: &str = "forwarding_server";
/// The edge (`rust/vtop-edge`).
const DEFAULT_SERVER: &str = "https://vmate.aster0.dev";
/// VTOP's OTP sender: the Gmail filter's From.
pub const VTOP_SENDER: &str = "noreply.sdc@vitap.ac.in";
/// VTOP's clock and the mail's `Date:` may differ a little.
const CLOCK_SKEW_SECS: i64 = 60;

/// Serialises read-modify-write of [`FILE`].
pub(crate) static FILE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum ForwardingState {
    /// No address on this phone.
    Off,
    /// An address, but no mail has reached it yet (Gmail not set up).
    Waiting,
    /// Mail reached the address (or it was joined from the student's other phone).
    On,
}

/// Gmail's "confirm forwarding" mail: the code to type in Gmail, or the link to open.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct GmailConfirmation {
    pub code: Option<String>,
    pub link: Option<String>,
    /// The Gmail address asking to forward.
    pub requester: Option<String>,
    /// Unix seconds.
    pub sent_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Forwarding {
    pub state: ForwardingState,
    pub address: Option<String>,
    /// What the Gmail filter matches in From.
    pub sender: String,
    /// The latest Gmail confirmation that arrived, while setting up.
    pub confirmation: Option<GmailConfirmation>,
    /// Unix seconds of the latest forwarded mail of either kind.
    pub last_mail_at: Option<i64>,
    /// Unix seconds of the latest forwarded OTP.
    pub last_otp_at: Option<i64>,
}

/// This phone's address. A file from before signed requests (it held a mailbox secret and its
/// own key) does not parse, so it reads as no address.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    /// The edge the address lives on.
    server: String,
    id: String,
    address: String,
    #[serde(default)]
    owner_claimed: bool,
    /// Mail arrived, or the address was joined (Gmail already forwards to it).
    #[serde(default)]
    ready: bool,
    #[serde(default)]
    last_mail_at: Option<i64>,
    #[serde(default)]
    last_otp_at: Option<i64>,
    #[serde(default)]
    confirmation: Option<GmailConfirmation>,
}

/// What a box holds (the edge's `Payload` in `vtop-edge/src/mail.ts`).
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Payload {
    Otp {
        code: String,
        #[serde(rename = "sentAt")]
        sent_at: i64,
    },
    GmailConfirm {
        code: Option<String>,
        link: Option<String>,
        requester: Option<String>,
        #[serde(rename = "sentAt")]
        sent_at: i64,
    },
}

impl Payload {
    fn sent_at(&self) -> i64 {
        match self {
            Self::Otp { sent_at, .. } | Self::GmailConfirm { sent_at, .. } => *sent_at,
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Self::Otp { .. } => "otp",
            Self::GmailConfirm { .. } => "gmail_confirm",
        }
    }

    /// A six-digit OTP sent no earlier than `since` (minus the clock skew).
    fn otp_since(&self, since: i64) -> Option<&str> {
        match self {
            Self::Otp { code, sent_at }
                if code.len() == 6
                    && code.bytes().all(|b| b.is_ascii_digit())
                    && *sent_at >= since - CLOCK_SKEW_SECS =>
            {
                Some(code)
            }
            _ => None,
        }
    }
}

/// Opens a box sealed to this install's seal key for mailbox `id`; the reason it couldn't is
/// logged, never the content.
fn open(seal_secret: &str, id: &str, sealed: &str) -> Option<Payload> {
    let plaintext = mail_seal::open(seal_secret, id, sealed)
        .inspect_err(|error| warn!(?error, "forwarded box not opened"))
        .ok()?;
    serde_json::from_slice(&plaintext)
        .inspect_err(|_| warn!("forwarded box holds an unknown payload"))
        .ok()
}

fn string(body: &Value, key: &str) -> Result<String, CoreError> {
    body[key]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| CoreError::Other {
            reason: "The server sent an unexpected answer.".into(),
        })
}

/// The edge no longer knows this address (deleted, or this install is no longer on it).
const GONE: &str = "mailbox_not_found";

/// Mail forwarding calls; each takes the caller's flow id first.
#[uniffi::export]
impl Core {
    /// This phone's forwarding address and how far its setup got. No network.
    pub fn forwarding_status(&self, flow: String) -> Forwarding {
        traced_sync("forwarding_status", &flow, || Ok(self.forwarding_view()))
            .unwrap_or_else(|_| view(None))
    }

    /// Gives this phone a forwarding address. A student who already has one (made on another
    /// phone; the edge knows it by their VTOP session) joins it, so their Gmail filter keeps
    /// working; otherwise a new address is made and tied to them. Does nothing when this phone
    /// already has one.
    pub async fn set_up_forwarding(&self, flow: String) -> Result<Forwarding, CoreError> {
        traced("set_up_forwarding", &flow, async {
            if self.read::<Saved>(FILE).is_some() {
                info!("already set up");
                return Ok(self.forwarding_view());
            }
            let server = self.forwarding_server();
            let cookies = match self.session_cookies().await {
                Ok(cookies) => cookies,
                Err(CoreError::NotSignedIn) => None,
                Err(error) => return Err(error),
            };
            let mut joined = None;
            if let Some(cookies) = &cookies {
                let reclaim = json!({ "session": { "cookies": cookies } });
                match self
                    .edge_post(&server, "/v1/mail-otp/reclaim", &flow, &reclaim)
                    .await
                {
                    Ok(body) => joined = Some(body),
                    Err(error) if error.code() == GONE => info!("no address yet: making one"),
                    Err(error) => return Err(error.into()),
                }
            }
            let reclaimed = joined.is_some();
            let body = match joined {
                Some(body) => body,
                None => {
                    self.edge_post(&server, "/v1/mail-otp/register", &flow, &json!({}))
                        .await?
                }
            };
            let mut saved = Saved {
                server: server.clone(),
                id: string(&body, "id")?,
                address: string(&body, "address")?,
                owner_claimed: reclaimed,
                ready: reclaimed,
                last_mail_at: None,
                last_otp_at: None,
                confirmation: None,
            };
            self.write(FILE, &saved);
            info!(reclaimed, "forwarding address set up");
            // Best effort: lets the student's next phone join it, and the server sign in with it.
            if let (false, Some(cookies)) = (reclaimed, cookies) {
                let owner = json!({ "id": saved.id, "session": { "cookies": cookies } });
                match self
                    .edge_post(&server, "/v1/mail-otp/owner", &flow, &owner)
                    .await
                {
                    Ok(_) => {
                        saved.owner_claimed = true;
                        self.write(FILE, &saved);
                    }
                    Err(error) => warn!(code = error.code(), "owner not claimed"),
                }
            }
            Ok(self.forwarding_view())
        })
        .await
    }

    /// Pulls the latest forwarded mail (the edge keeps it 5 minutes): Gmail's confirmation while
    /// setting up, or an OTP. Either one proves the address works.
    pub async fn check_forwarding(&self, flow: String) -> Result<Forwarding, CoreError> {
        traced("check_forwarding", &flow, async {
            self.pull(&flow).await?;
            Ok(self.forwarding_view())
        })
        .await
    }

    /// The forwarded OTP sent since `since` (unix seconds), if one is in the inbox; for the OTP
    /// prompt, which calls it every few seconds. Never logs the code.
    pub async fn fetch_forwarded_otp(
        &self,
        flow: String,
        since: i64,
    ) -> Result<Option<String>, CoreError> {
        traced("fetch_forwarded_otp", &flow, async {
            let payload = self.pull(&flow).await?;
            let code = payload.as_ref().and_then(|p| p.otp_since(since));
            info!(found = code.is_some(), "forwarded otp");
            Ok(code.map(str::to_string))
        })
        .await
    }

    /// Takes this phone off its address and forgets it. The edge deletes the address when no
    /// other phone uses it; mail forwarded to it is rejected from then on.
    pub async fn turn_off_forwarding(&self, flow: String) -> Result<(), CoreError> {
        traced(
            "turn_off_forwarding",
            &flow,
            self.leave_forwarding(&flow, false),
        )
        .await
    }
}

impl Core {
    /// This phone's address as `(server, mailbox id, address)`, if it has one.
    pub(crate) fn saved_mailbox(&self) -> Option<(String, String, String)> {
        self.read::<Saved>(FILE)
            .map(|saved| (saved.server, saved.id, saved.address))
    }

    /// Takes this phone off its address (deleting it for every phone with `everywhere`) and
    /// forgets it here.
    pub(crate) async fn leave_forwarding(
        &self,
        flow: &str,
        everywhere: bool,
    ) -> Result<(), CoreError> {
        let Some(saved) = self.read::<Saved>(FILE) else {
            return Ok(());
        };
        let body = json!({ "id": saved.id, "everywhere": everywhere });
        match self
            .edge_post(&saved.server, "/v1/mail-otp/delete", flow, &body)
            .await
        {
            Ok(_) => info!(everywhere, "left the address"),
            Err(error) if error.code() == GONE => info!("address already gone"),
            Err(error) => return Err(error.into()),
        }
        self.forget_forwarding(&saved.id);
        Ok(())
    }

    fn forwarding_view(&self) -> Forwarding {
        view(self.read::<Saved>(FILE).as_ref())
    }

    pub(crate) fn forwarding_server(&self) -> String {
        fs::read_to_string(self.dir.join(SERVER_FILE))
            .ok()
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| DEFAULT_SERVER.to_string())
    }

    /// The signed-in VTOP session's cookies, after VTOP confirmed it (signing in again when it
    /// had expired), so the edge can tell whose session it is.
    pub(crate) async fn session_cookies(&self) -> Result<Option<String>, CoreError> {
        let cookies = |client: &VtopClient| client.export_persisted_session(0).cookies;
        signed_in!(self, |client| async {
            client.verify_session().await.map(|_| cookies(client))
        })
    }

    /// The latest box in this phone's inbox, opened and recorded; None without an address, a
    /// box, or one that opens. An address the edge no longer knows is forgotten here.
    async fn pull(&self, flow: &str) -> Result<Option<Payload>, CoreError> {
        let Some(saved) = self.read::<Saved>(FILE) else {
            return Err(CoreError::Other {
                reason: "Mail forwarding is off.".into(),
            });
        };
        let body = json!({ "id": saved.id });
        let inbox = match self
            .edge_post(&saved.server, "/v1/mail-otp/inbox", flow, &body)
            .await
        {
            Ok(inbox) => inbox,
            Err(error) if error.code() == GONE => {
                self.forget_forwarding(&saved.id);
                return Err(CoreError::Other {
                    reason: "Your forwarding address no longer exists. Set it up again.".into(),
                });
            }
            Err(error) => return Err(error.into()),
        };
        let Some(sealed) = inbox["sealed"].as_str().filter(|s| !s.is_empty()) else {
            debug!("inbox empty");
            return Ok(None);
        };
        let Some(payload) = open(&self.device().seal_secret, &saved.id, sealed) else {
            return Ok(None);
        };
        info!(
            kind = payload.kind(),
            sent_at = payload.sent_at(),
            "forwarded mail"
        );
        self.record(&saved.id, &payload);
        Ok(Some(payload))
    }

    /// Notes that `payload` reached the address `id` (if it is still this phone's).
    fn record(&self, id: &str, payload: &Payload) {
        let _lock = FILE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(mut saved) = self.read::<Saved>(FILE).filter(|saved| saved.id == id) else {
            return;
        };
        let before = saved.clone();
        let sent_at = payload.sent_at();
        saved.ready = true;
        saved.last_mail_at = saved.last_mail_at.max(Some(sent_at));
        match payload {
            Payload::Otp { .. } => saved.last_otp_at = saved.last_otp_at.max(Some(sent_at)),
            Payload::GmailConfirm {
                code,
                link,
                requester,
                sent_at,
            } => {
                saved.confirmation = Some(GmailConfirmation {
                    code: code.clone(),
                    link: link.clone(),
                    requester: requester.clone(),
                    sent_at: *sent_at,
                });
            }
        }
        if saved != before {
            self.write(FILE, &saved);
        }
    }

    pub(crate) fn forget_forwarding(&self, id: &str) {
        let _lock = FILE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        if self.read::<Saved>(FILE).is_some_and(|saved| saved.id == id) {
            let _ = fs::remove_file(self.dir.join(FILE));
            info!("forwarding forgotten");
        }
    }
}

fn view(saved: Option<&Saved>) -> Forwarding {
    Forwarding {
        state: match saved {
            None => ForwardingState::Off,
            Some(saved) if saved.ready => ForwardingState::On,
            Some(_) => ForwardingState::Waiting,
        },
        address: saved.map(|saved| saved.address.clone()),
        sender: VTOP_SENDER.to_string(),
        confirmation: saved.and_then(|saved| saved.confirmation.clone()),
        last_mail_at: saved.and_then(|saved| saved.last_mail_at),
        last_otp_at: saved.and_then(|saved| saved.last_otp_at),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::{HashMap, VecDeque};
    use std::sync::Arc;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use vtop_shared::device_sig;

    use super::*;

    /// One request the fake edge got.
    pub(crate) struct Request {
        pub(crate) path: String,
        /// `x-request-id`.
        pub(crate) flow: String,
        pub(crate) body: Value,
        /// Whether `x-vm-*` verified against the registered key (or, for registration, the key
        /// it registers), with `x-vm-device` only off registration.
        pub(crate) signed: bool,
    }

    /// A stand-in edge: answers the routes the app calls, from `Edge`.
    #[derive(Default)]
    pub(crate) struct Edge {
        pub(crate) requests: Vec<Request>,
        /// Answers for other routes (path with query), taken in order before the built-in ones.
        pub(crate) canned: HashMap<String, VecDeque<(u16, Value)>>,
        /// The registered `signKey` and `sealKey`.
        sign_key: Option<String>,
        seal_key: Option<String>,
        sealed: Option<String>,
        gone: bool,
    }

    const ID: &str = "k3f9x2ab7q";
    pub(crate) const DEVICE_ID: &str = "0123456789abcdef0123456789abcdef";

    fn header<'a>(head: &'a str, name: &str) -> Option<&'a str> {
        head.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name).then(|| value.trim())
        })
    }

    /// Checks the vm1 headers the way the edge does.
    fn verify(edge: &Edge, path: &str, head: &str, body: &[u8], json: &Value) -> bool {
        let register = path == "/v1/devices/register";
        let key = if register {
            json["signKey"].as_str()
        } else {
            edge.sign_key.as_deref()
        };
        let device_ok = match header(head, device_sig::DEVICE) {
            None => register,
            Some(id) => !register && id == DEVICE_ID,
        };
        let time = header(head, device_sig::TIME).and_then(|t| t.parse::<i64>().ok());
        let fresh = time.is_some_and(|t| (t - now()).abs() <= 120);
        match (key, header(head, device_sig::SIG), time) {
            (Some(key), Some(sig), Some(time)) => {
                device_ok && fresh && device_sig::verify(key, sig, "POST", path, time, body)
            }
            _ => false,
        }
    }

    fn answer(edge: &mut Edge, path: &str, body: &Value) -> (u16, Value) {
        let gone = (
            404,
            json!({ "code": GONE, "error": "No such forwarding address." }),
        );
        match path {
            _ if edge.canned.get(path).is_some_and(|c| !c.is_empty()) => {
                edge.canned.get_mut(path).unwrap().pop_front().unwrap()
            }
            "/v1/devices/register" => {
                edge.sign_key = body["signKey"].as_str().map(str::to_string);
                edge.seal_key = body["sealKey"].as_str().map(str::to_string);
                (201, json!({ "deviceId": DEVICE_ID }))
            }
            "/v1/mail-otp/register" => {
                let address = format!("otp+{ID}@vmate.aster0.dev");
                (201, json!({ "id": ID, "address": address }))
            }
            _ if edge.gone => gone,
            "/v1/mail-otp/inbox" => match &edge.sealed {
                Some(sealed) => (200, json!({ "sealed": sealed, "receivedAt": now() })),
                None => (200, json!({ "sealed": null })),
            },
            "/v1/mail-otp/delete" => (204, Value::Null),
            _ => (
                404,
                json!({ "code": "not_found", "error": "No such route." }),
            ),
        }
    }

    pub(crate) async fn serve(edge: Arc<Mutex<Edge>>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut raw = Vec::new();
                let mut buf = [0u8; 4096];
                let (head, body) = loop {
                    let n = socket.read(&mut buf).await.unwrap();
                    raw.extend_from_slice(&buf[..n]);
                    let Some(end) = raw.windows(4).position(|w| w == b"\r\n\r\n") else {
                        continue;
                    };
                    let head = String::from_utf8_lossy(&raw[..end]).to_string();
                    let length = header(&head, "content-length")
                        .and_then(|v| v.parse::<usize>().ok())
                        .unwrap_or(0);
                    if raw.len() >= end + 4 + length {
                        break (head, raw[end + 4..end + 4 + length].to_vec());
                    }
                };
                let path = head.split(' ').nth(1).unwrap().to_string();
                let flow = header(&head, "x-request-id")
                    .unwrap_or_default()
                    .to_string();
                let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
                let (status, reply) = {
                    let mut edge = edge.lock().unwrap();
                    let signed = verify(&edge, &path, &head, &body, &json);
                    let reply = if signed {
                        answer(&mut edge, &path, &json)
                    } else {
                        (
                            401,
                            json!({ "code": "bad_signature", "error": "Bad signature." }),
                        )
                    };
                    edge.requests.push(Request {
                        path,
                        flow,
                        body: json,
                        signed,
                    });
                    reply
                };
                let text = if reply.is_null() {
                    String::new()
                } else {
                    reply.to_string()
                };
                let response = format!(
                    "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}",
                    text.len()
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });
        url
    }

    /// Seals `payload` the way the edge does: to the registered seal key, for mailbox [`ID`].
    fn seal(edge: &Arc<Mutex<Edge>>, payload: Value) {
        let mut edge = edge.lock().unwrap();
        let key = edge.seal_key.clone().unwrap();
        edge.sealed = Some(mail_seal::seal(&key, ID, payload.to_string().as_bytes()).unwrap());
    }

    fn now() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }

    #[tokio::test]
    async fn sets_up_checks_fetches_and_turns_off() {
        let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let edge = Arc::new(Mutex::new(Edge::default()));
        fs::write(dir.join(SERVER_FILE), serve(edge.clone()).await + "\n").unwrap();
        // Not signed in: a new address, no owner claim.
        let core = Core {
            dir: dir.clone(),
            slot: Mutex::new(None),
        };
        assert_eq!(
            core.forwarding_status("f".into()).state,
            ForwardingState::Off
        );
        let fetched = core.fetch_forwarded_otp("f".into(), 0).await;
        assert!(matches!(fetched, Err(CoreError::Other { .. })));

        let set_up = core.set_up_forwarding("f-setup".into()).await.unwrap();
        assert_eq!(set_up.state, ForwardingState::Waiting);
        assert_eq!(
            set_up.address.as_deref(),
            Some("otp+k3f9x2ab7q@vmate.aster0.dev")
        );
        assert_eq!(set_up.sender, VTOP_SENDER);
        {
            let edge = edge.lock().unwrap();
            let paths: Vec<&str> = edge.requests.iter().map(|r| r.path.as_str()).collect();
            assert_eq!(paths, ["/v1/devices/register", "/v1/mail-otp/register"]);
            assert!(edge.requests.iter().all(|r| r.flow == "f-setup"));
            let register = &edge.requests[0].body;
            assert_eq!(
                (&register["label"], &register["platform"]),
                (&json!(crate::server::label()), &json!("android"))
            );
            assert!(mail_seal::is_public_key(
                register["sealKey"].as_str().unwrap()
            ));
            assert_eq!(edge.requests[1].body, json!({}));
        }
        // Set up already: nothing is sent.
        core.set_up_forwarding("f".into()).await.unwrap();
        assert_eq!(edge.lock().unwrap().requests.len(), 2);

        // Nothing forwarded yet.
        let checked = core.check_forwarding("f".into()).await.unwrap();
        assert_eq!(checked.state, ForwardingState::Waiting);
        assert_eq!(
            edge.lock().unwrap().requests.last().unwrap().body,
            json!({ "id": ID })
        );

        // Gmail's confirmation proves the address works.
        seal(
            &edge,
            json!({ "kind": "gmail_confirm", "code": "981234567", "link": null, "requester": "me@vitapstudent.ac.in", "sentAt": 1_700_000_000 }),
        );
        let checked = core.check_forwarding("f".into()).await.unwrap();
        assert_eq!(checked.state, ForwardingState::On);
        let confirmation = checked.confirmation.unwrap();
        assert_eq!(confirmation.code.as_deref(), Some("981234567"));
        assert_eq!(checked.last_mail_at, Some(1_700_000_000));
        assert_eq!(checked.last_otp_at, None);
        // Not an OTP.
        assert_eq!(core.fetch_forwarded_otp("f".into(), 0).await, Ok(None));

        let sent = now();
        seal(
            &edge,
            json!({ "kind": "otp", "code": "401295", "sentAt": sent }),
        );
        let code = core.fetch_forwarded_otp("f".into(), sent - 30).await;
        assert_eq!(code, Ok(Some("401295".into())));
        // An OTP from before this prompt.
        let code = core.fetch_forwarded_otp("f".into(), sent + 120).await;
        assert_eq!(code, Ok(None));
        let status = core.forwarding_status("f".into());
        assert_eq!(status.last_otp_at, Some(sent));
        // No key or secret in forwarding.json.
        let saved = fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(!saved.contains("secret") && !saved.contains("seed"));

        core.turn_off_forwarding("f-off".into()).await.unwrap();
        {
            let edge = edge.lock().unwrap();
            let last = edge.requests.last().unwrap();
            assert_eq!(
                (last.path.as_str(), last.flow.as_str()),
                ("/v1/mail-otp/delete", "f-off")
            );
            assert_eq!(last.body, json!({ "id": ID, "everywhere": false }));
        }
        assert_eq!(
            core.forwarding_status("f".into()).state,
            ForwardingState::Off
        );
        assert!(!dir.join(FILE).exists());

        // An address the edge lost is forgotten here.
        core.set_up_forwarding("f".into()).await.unwrap();
        edge.lock().unwrap().gone = true;
        let checked = core.check_forwarding("f".into()).await;
        assert!(matches!(checked, Err(CoreError::Other { .. })));
        assert_eq!(
            core.forwarding_status("f".into()).state,
            ForwardingState::Off
        );

        // Registered once; every request was signed as this install.
        let edge = edge.lock().unwrap();
        let registers = edge
            .requests
            .iter()
            .filter(|r| r.path == "/v1/devices/register")
            .count();
        assert_eq!(registers, 1);
        assert!(edge.requests.iter().all(|r| r.signed));
    }

    #[tokio::test]
    async fn requests_carry_valid_signatures() {
        let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-sig-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let edge = Arc::new(Mutex::new(Edge::default()));
        let server = serve(edge.clone()).await;
        let core = Core {
            dir: dir.clone(),
            slot: Mutex::new(None),
        };
        let body = json!({ "id": ID });
        core.edge_post(&server, "/v1/mail-otp/inbox", "f", &body)
            .await
            .unwrap();
        let edge = edge.lock().unwrap();
        assert_eq!(edge.requests.len(), 2);
        assert!(edge.requests.iter().all(|r| r.signed));
        // The key the edge saw is this install's: a signature by another seed does not verify.
        let other = device_sig::generate_seed();
        let sig = device_sig::sign(&other, "POST", "/v1/mail-otp/inbox", now(), b"{}").unwrap();
        let key = edge.sign_key.as_deref().unwrap();
        assert!(!device_sig::verify(
            key,
            &sig,
            "POST",
            "/v1/mail-otp/inbox",
            now(),
            b"{}"
        ));
    }

    #[tokio::test]
    async fn old_file_reads_as_no_address() {
        let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-old-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let old = json!({
            "server": "https://vmate-api.aster0.dev", "id": ID, "address": "a@b",
            "secret": "ms_x", "private_key": "pk", "device_id": "d", "device_secret": "ds",
            "ready": true,
        });
        fs::write(dir.join(FILE), old.to_string()).unwrap();
        let core = Core {
            dir: dir.clone(),
            slot: Mutex::new(None),
        };
        assert_eq!(
            core.forwarding_status("f".into()).state,
            ForwardingState::Off
        );
    }

    #[tokio::test]
    async fn offline_keeps_the_address() {
        let dir = std::env::temp_dir().join(format!("vtop-ffi-fwd-off-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let core = Core {
            dir: dir.clone(),
            slot: Mutex::new(None),
        };
        let saved = Saved {
            // Nothing listens on port 9.
            server: "http://127.0.0.1:9".into(),
            id: ID.into(),
            address: "a@b".into(),
            owner_claimed: false,
            ready: false,
            last_mail_at: None,
            last_otp_at: None,
            confirmation: None,
        };
        core.write(FILE, &saved);
        assert_eq!(
            core.check_forwarding("f".into()).await,
            Err(CoreError::Offline)
        );
        assert_eq!(
            core.turn_off_forwarding("f".into()).await,
            Err(CoreError::Offline)
        );
        assert_eq!(
            core.forwarding_status("f".into()).state,
            ForwardingState::Waiting
        );
    }

    #[test]
    fn otps_are_six_digits_and_recent() {
        let otp = |text: &str| serde_json::from_str::<Payload>(text).unwrap();
        let fresh = otp(r#"{"kind":"otp","code":"401295","sentAt":1000}"#);
        assert_eq!(fresh.otp_since(1000), Some("401295"));
        assert_eq!(fresh.otp_since(1000 + CLOCK_SKEW_SECS), Some("401295"));
        assert_eq!(fresh.otp_since(1001 + CLOCK_SKEW_SECS), None);
        assert_eq!(
            otp(r#"{"kind":"otp","code":"40129a","sentAt":1000}"#).otp_since(0),
            None
        );
        let confirm = otp(
            r#"{"kind":"gmail_confirm","code":null,"link":"https://mail-settings.google.com/mail/vf-x","requester":null,"sentAt":5}"#,
        );
        assert_eq!(
            (confirm.kind(), confirm.otp_since(0)),
            ("gmail_confirm", None)
        );
        assert!(serde_json::from_str::<Payload>(r#"{"kind":"other","sentAt":1}"#).is_err());
    }
}
