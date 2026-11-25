//! A stand-in server for tests: a fake Convex backend answering the functions the app calls by
//! name, checking `signed` calls' signatures and that `jwt` calls go with a JWT, as the server
//! does; plus a local HTTP server for VTOP's session check and the public HTTP actions. A core
//! whose server (`SERVER_FILE`) is one [`serve`] made uses its fake backend.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use futures::FutureExt;
use futures::future::BoxFuture;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use vtop_shared::device_sig;

use crate::Core;
use crate::convex::CallError;
use crate::remote::backend::{Backend, Kind, Refresh};
use crate::remote::server_code::ServerCode;

/// One call the fake server got.
pub(crate) struct Request {
    /// The function (`mail:inbox`), or an HTTP path.
    pub(crate) path: String,
    pub(crate) flow: String,
    /// The arguments without `flow`; a signed call's payload.
    pub(crate) body: Value,
    /// A signed call: its signature verified. Any other: it went with a JWT.
    pub(crate) signed: bool,
}

/// A stand-in server: answers the functions the app calls, from `Edge`.
#[derive(Default)]
pub(crate) struct Edge {
    pub(crate) requests: Vec<Request>,
    /// Answers by function (or HTTP path), taken in order before the built-in ones; a status
    /// of 300 or more throws the body as a `ConvexError`.
    pub(crate) canned: HashMap<String, VecDeque<(u16, Value)>>,
    /// The `sealKey` the last `auth:token` sent.
    pub(crate) seal_key: Option<String>,
    pub(crate) sealed: Option<String>,
    pub(crate) gone: bool,
    /// Where `mail:join` finds the student's address once this install's went (`gone`); none:
    /// the student has no address.
    pub(crate) moved_to: Option<String>,
    /// The JWT the client goes with.
    pub(crate) auth: Option<String>,
    /// JWTs issued so far.
    pub(crate) issued: u32,
    /// Unreachable: every call fails as a closed socket does, and none is recorded.
    pub(crate) offline: bool,
}

pub(crate) const ID: &str = "k3f9x2ab7q";
/// The ticket `auth:proveStart` hands out.
pub(crate) const CHALLENGE: &str = "00112233445566778899aabbccddeeff";
/// The refresh token [`keep_token`] keeps.
pub(crate) const REFRESH: &str = "vrt_kept";

/// Functions that take a signature, not a JWT.
const SIGNED: [&str; 5] = [
    "auth:proveStart",
    "auth:token",
    "vtop:login",
    "vtop:loginOtp",
    "vtop:loginResend",
];
/// Functions anyone may call.
const PUBLIC: [&str; 4] = [
    "udp:keys",
    "guides:demo",
    "guides:updates",
    "guides:forwarding",
];

/// The fake server behind every core whose server is one [`serve`] made.
static EDGES: Mutex<Vec<(String, Arc<Mutex<Edge>>)>> = Mutex::new(Vec::new());

/// The fake backend of `server`, if [`serve`] made it.
pub(crate) fn backend_for(server: &str) -> Option<Arc<dyn Backend>> {
    let edges = EDGES.lock().unwrap();
    let edge = edges.iter().find(|(url, _)| url == server)?.1.clone();
    Some(Arc::new(Fake(edge)))
}

struct Fake(Arc<Mutex<Edge>>);

impl Backend for Fake {
    fn run<'a>(
        &'a self,
        _kind: Kind,
        path: &'a str,
        mut args: Value,
    ) -> BoxFuture<'a, Result<Value, CallError>> {
        let mut edge = self.0.lock().unwrap();
        if edge.offline {
            return Box::pin(async { Err(CallError::Closed) });
        }
        let flow = args
            .as_object_mut()
            .and_then(|args| args.remove("flow"))
            .and_then(|flow| flow.as_str().map(str::to_string))
            .unwrap_or_default();
        let (body, signed) = match args.get("signed") {
            Some(signed) => {
                let payload = args["payload"].as_str().unwrap_or_default().to_string();
                let text = |key: &str| signed[key].as_str().unwrap_or_default().to_string();
                let time = signed["time"].as_i64().unwrap_or_default();
                if (time - now()).abs() > 120 {
                    let stale = json!({ "code": "stale_time", "serverTime": now() });
                    edge.requests.push(Request {
                        path: path.to_string(),
                        flow,
                        body: Value::Null,
                        signed: false,
                    });
                    return Box::pin(async move { Err(convex(stale)) });
                }
                let ok = device_sig::verify(
                    &text("key"),
                    &text("sig"),
                    "CALL",
                    &format!("/{path}"),
                    time,
                    payload.as_bytes(),
                );
                (serde_json::from_str(&payload).unwrap_or(Value::Null), ok)
            }
            None => (args, edge.auth.is_some()),
        };
        let reply = if PUBLIC.contains(&path) {
            answer(&mut edge, path, &body)
        } else if !signed && SIGNED.contains(&path) {
            Err(convex(
                json!({ "code": "bad_signature", "error": "Bad signature." }),
            ))
        } else if !signed {
            Err(convex(
                json!({ "code": "unauthenticated", "error": "Sign in first." }),
            ))
        } else {
            answer(&mut edge, path, &body)
        };
        edge.requests.push(Request {
            path: path.to_string(),
            flow,
            body,
            signed,
        });
        Box::pin(async move { reply })
    }

    fn set_auth(&self, jwt: Option<String>) {
        self.0.lock().unwrap().auth = jwt;
    }

    /// Answers once, at once, as the query's first result.
    fn watch(&self, path: &str, args: Value, on_update: crate::remote::backend::OnUpdate) -> u64 {
        let answer = self.run(Kind::Query, path, args).now_or_never();
        on_update(answer.unwrap_or(Err(CallError::Closed)));
        0
    }

    fn unwatch(&self, _id: u64) {}
}

fn convex(data: Value) -> CallError {
    CallError::Convex {
        message: "Uncaught ConvexError".into(),
        data,
    }
}

fn answer(edge: &mut Edge, path: &str, body: &Value) -> Result<Value, CallError> {
    if let Some(canned) = edge.canned.get_mut(path).and_then(VecDeque::pop_front) {
        return match canned {
            (status, body) if status >= 300 => Err(convex(body)),
            (_, body) => Ok(body),
        };
    }
    let gone = || {
        Err(convex(
            json!({ "code": ServerCode::MailboxNotFound.as_str(), "error": "No such forwarding address." }),
        ))
    };
    match path {
        "auth:proveStart" => Ok(json!({ "ticket": CHALLENGE, "expiresAt": 0 })),
        "auth:token" => {
            if let Some(seal) = body["device"]["sealKey"].as_str() {
                edge.seal_key = Some(seal.to_string());
            }
            edge.issued += 1;
            let regno = if body["demo"] == json!(true) {
                "DEMO2026"
            } else {
                "22BCE0001"
            };
            Ok(json!({
                "jwt": format!("jwt-{}", edge.issued),
                "jwtExpiresAt": (now() + 900) * 1000,
                "refreshToken": format!("vrt_{}", edge.issued),
                "regno": regno,
                "account": "acct1",
            }))
        }
        "mail:register" => {
            let address = format!("otp+{ID}@vmate.aster0.dev");
            Ok(json!({ "id": ID, "address": address }))
        }
        "mail:join" if edge.gone => Ok(match &edge.moved_to {
            Some(id) => json!({ "id": id, "address": format!("otp+{id}@vmate.aster0.dev") }),
            None => Value::Null,
        }),
        _ if edge.gone && path.starts_with("mail:") => gone(),
        "mail:inbox" => match &edge.sealed {
            Some(sealed) => Ok(json!({ "sealed": sealed, "receivedAt": now() })),
            None => Ok(json!({ "sealed": null })),
        },
        "mail:remove" | "mail:leave" => Ok(Value::Null),
        _ => Err(CallError::Message(format!(
            "Could not find public function for '{path}'"
        ))),
    }
}

/// A fake server: its URL, which also answers VTOP's session check and the public HTTP
/// actions (from `canned`, by path).
pub(crate) async fn serve(edge: Arc<Mutex<Edge>>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    EDGES.lock().unwrap().push((url.clone(), edge.clone()));
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
            let (status, reply) = {
                let mut edge = edge.lock().unwrap();
                let reply = edge
                    .canned
                    .get_mut(&path)
                    .and_then(VecDeque::pop_front)
                    .unwrap_or((
                        404,
                        json!({ "code": "not_found", "error": "No such route." }),
                    ));
                edge.requests.push(Request {
                    path,
                    flow,
                    body: json,
                    signed: false,
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

fn header<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    head.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.eq_ignore_ascii_case(name).then(|| value.trim())
    })
}

/// A core signed in to VTOP at `base` (the mock server also answers VTOP's session check),
/// whose server is `base` too.
pub(crate) fn signed_in(dir: &std::path::Path, base: String) -> Core {
    use vtop_core::sync::RwLock;
    use vtop_core::{SessionState, VtopClient, VtopConfig};
    std::fs::write(dir.join(crate::remote::edge::SERVER_FILE), &base).unwrap();
    let client = VtopClient::builder()
        .config(VtopConfig {
            base_url: base,
            ..VtopConfig::default()
        })
        .with_session(&SessionState {
            cookies: format!("JSESSIONID={}", vtop_tlsn::fake::SID),
            csrf_token: Some("csrf-1".into()),
            registration_number: Some("22BCE0001".into()),
            otp_issued_at: None,
            logged_in_at: None,
        })
        .unwrap();
    let session = crate::Session::for_test(
        client,
        crate::Account {
            username: "22BCE0001".into(),
            password: "pw".into(),
            registration_number: None,
        },
    );
    let core = Core::for_test(dir);
    core.install(Some(Arc::new(RwLock::new(session))));
    // The student said Share; `remote::regno`'s own tests start from Unasked.
    crate::remote::regno::tests::shared(&core, "22BCE0001");
    core
}

/// Keeps refresh token [`REFRESH`] for 22BCE0001 on `server`, as a proof would, so jwt calls
/// go without proving.
pub(crate) fn keep_token(core: &Core, server: &str) {
    core.keep_refresh(Some(Refresh {
        server: server.into(),
        token: REFRESH.into(),
        regno: "22BCE0001".into(),
    }));
}

/// The functions (and paths) the fake server got, in order.
pub(crate) fn paths(edge: &Arc<Mutex<Edge>>) -> Vec<String> {
    let edge = edge.lock().unwrap();
    edge.requests.iter().map(|r| r.path.clone()).collect()
}

pub(crate) fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
