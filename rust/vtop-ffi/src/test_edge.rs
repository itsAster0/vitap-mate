//! A stand-in edge for tests: a local HTTP server answering the routes the app calls (and
//! VTOP's session check), checking each request's device signature as the edge does.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use vtop_shared::device_sig;

use crate::Core;
use crate::remote::forwarding::GONE;

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
    pub(crate) sign_key: Option<String>,
    pub(crate) seal_key: Option<String>,
    pub(crate) sealed: Option<String>,
    pub(crate) gone: bool,
    /// An account token to issue on answers to calls that send the session.
    pub(crate) issue: Option<String>,
}

pub(crate) const ID: &str = "k3f9x2ab7q";
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
        "/v1/mail-otp/reclaim" => gone,
        "/v1/mail-otp/owner" => (204, Value::Null),
        "/v1/mail-otp/register" => {
            let address = format!("otp+{ID}@vmate.aster0.dev");
            (201, json!({ "id": ID, "address": address }))
        }
        _ if edge.gone => gone,
        "/v1/mail-otp/inbox" => match &edge.sealed {
            Some(sealed) => (200, json!({ "sealed": sealed, "receivedAt": now() })),
            None => (200, json!({ "sealed": null })),
        },
        "/v1/mail-otp/delete" | "/v1/mail-otp/leave" => (204, Value::Null),
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
            if path.starts_with("/vtop/") {
                // VTOP itself, for the session check: signed in.
                let page = r#"<html><input name="_csrf" value="csrf-1"><input type="hidden" name="authorizedIDX" value="22BCE0001"></html>"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: text/html\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{page}",
                    page.len()
                );
                socket.write_all(response.as_bytes()).await.unwrap();
                continue;
            }
            let flow = header(&head, "x-request-id")
                .unwrap_or_default()
                .to_string();
            let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            let (status, reply, issued) = {
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
                let issued = match &edge.issue {
                    Some(token) if json.get("session").is_some() && reply.0 < 300 => {
                        let at = (now() + 14 * 86_400) * 1000;
                        format!(
                            "x-vm-account-token: {token}\r\nx-vm-account-token-expires-at: {at}\r\n"
                        )
                    }
                    _ => String::new(),
                };
                edge.requests.push(Request {
                    path,
                    flow,
                    body: json,
                    signed,
                });
                (reply.0, reply.1, issued)
            };
            let text = if reply.is_null() {
                String::new()
            } else {
                reply.to_string()
            };
            let response = format!(
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\n{issued}content-length: {}\r\nconnection: close\r\n\r\n{text}",
                text.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });
    url
}

/// A core signed in to VTOP at `base` (the mock edge also answers VTOP's session check).
pub(crate) fn signed_in(dir: &std::path::Path, base: String) -> Core {
    use vtop_core::sync::RwLock;
    use vtop_core::{SessionState, VtopClient, VtopConfig};
    let client = VtopClient::builder()
        .config(VtopConfig {
            base_url: base,
            ..VtopConfig::default()
        })
        .with_session(&SessionState {
            cookies: "JSESSIONID=abc".into(),
            csrf_token: Some("csrf-1".into()),
            registration_number: Some("22BCE0001".into()),
            otp_issued_at: None,
            logged_in_at: None,
        })
        .unwrap();
    let session = crate::Session {
        client,
        cookies: crate::store::cookies::SessionCookies::new(),
        account: crate::Account {
            username: "22BCE0001".into(),
            password: "pw".into(),
            rejected: false,
            registration_number: None,
        },
        logins: 0,
        otp: None,
        server_otp: false,
    };
    let core = Core::for_test(dir);
    core.install(Some(Arc::new(RwLock::new(session))));
    core
}

pub(crate) fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
