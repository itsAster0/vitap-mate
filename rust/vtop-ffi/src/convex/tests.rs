//! The client against a fake WebSocket and a fake clock: the test plays the server, with frames
//! built and read through convex_sync_types' JSON conversions.

use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use convex_sync_types::{
    AuthenticationToken, ClientMessage, LogLinesMessage, QueryId, QuerySetModification,
    ServerMessage, StateModification, StateVersion, Timestamp,
};
use futures::FutureExt;
use serde_json::{Value, json};
use tracing::field::{Field, Visit};
use tracing::span::Attributes;
use tracing::{Event, Id, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

use super::client::{CallError, Clock, ConvexClient, Served};
use super::value::{self, ConvexValue};
use super::{ConvexNetwork, ConvexSocket, ConvexSocketListener};

const SILENCE: u64 = 30_000;

#[derive(Default)]
struct FakeClock(AtomicU64);

impl Clock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }
    fn jitter(&self, ms: u64) -> u64 {
        ms
    }
}

#[derive(Default)]
struct FakeSocket {
    sent: Mutex<Vec<String>>,
    closed: AtomicBool,
    /// Set on a reentrant network's sockets: `disconnect` and a failed `send` call it at once.
    listener: Mutex<Option<Arc<dyn ConvexSocketListener>>>,
    fail_sends: AtomicBool,
}

impl FakeSocket {
    fn call_on_closed(&self, reason: &str) {
        let listener = self.listener.lock().unwrap().clone();
        if let Some(listener) = listener {
            listener.on_closed(reason.into());
        }
    }
}

impl ConvexSocket for FakeSocket {
    fn send(&self, text: String) -> bool {
        if self.fail_sends.load(Ordering::SeqCst) {
            self.call_on_closed("send failed");
            return false;
        }
        self.sent.lock().unwrap().push(text);
        true
    }
    fn disconnect(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.call_on_closed("closed");
    }
}

struct Opened {
    at: u64,
    url: String,
    socket: Arc<FakeSocket>,
    listener: Arc<dyn ConvexSocketListener>,
}

#[derive(Default)]
struct FakeNet {
    clock: Arc<FakeClock>,
    opens: Mutex<Vec<Opened>>,
    /// Calls the listener from inside `open`, `send` and `disconnect`, against the contract.
    reentrant: bool,
}

impl ConvexNetwork for FakeNet {
    fn open(&self, url: String, listener: Arc<dyn ConvexSocketListener>) -> Arc<dyn ConvexSocket> {
        let socket = Arc::new(FakeSocket::default());
        if self.reentrant {
            *socket.listener.lock().unwrap() = Some(listener.clone());
        }
        self.opens.lock().unwrap().push(Opened {
            at: self.clock.now_ms(),
            url,
            socket: socket.clone(),
            listener: listener.clone(),
        });
        if self.reentrant {
            listener.on_open();
        }
        socket
    }
}

struct Harness {
    clock: Arc<FakeClock>,
    net: Arc<FakeNet>,
    auth_errors: Arc<AtomicUsize>,
    client: Arc<ConvexClient>,
}

impl Harness {
    fn new() -> Self {
        Self::build(false)
    }

    /// A client whose first tick opened a socket.
    fn build(reentrant: bool) -> Self {
        let clock = Arc::new(FakeClock::default());
        let net = Arc::new(FakeNet {
            clock: clock.clone(),
            reentrant,
            ..FakeNet::default()
        });
        let auth_errors = Arc::new(AtomicUsize::new(0));
        let seen = auth_errors.clone();
        let client = ConvexClient::with_clock(
            "https://convex.example",
            net.clone(),
            Box::new(move || {
                seen.fetch_add(1, Ordering::SeqCst);
            }),
            clock.clone(),
            false,
        );
        client.tick();
        Self {
            clock,
            net,
            auth_errors,
            client,
        }
    }

    fn opens(&self) -> usize {
        self.net.opens.lock().unwrap().len()
    }

    fn listener(&self, index: usize) -> Arc<dyn ConvexSocketListener> {
        self.net.opens.lock().unwrap()[index].listener.clone()
    }

    fn socket(&self, index: usize) -> Arc<FakeSocket> {
        self.net.opens.lock().unwrap()[index].socket.clone()
    }

    /// The newest socket opens.
    fn connect(&self) {
        self.listener(self.opens() - 1).on_open();
    }

    /// What the client sent on socket `index`.
    fn sent(&self, index: usize) -> Vec<ClientMessage> {
        let socket = self.socket(index);
        let sent = socket.sent.lock().unwrap();
        sent.iter()
            .map(|text| ClientMessage::try_from(serde_json::from_str::<Value>(text).unwrap()))
            .collect::<anyhow::Result<_>>()
            .unwrap()
    }

    fn last_sent(&self) -> ClientMessage {
        self.sent(self.opens() - 1).pop().unwrap()
    }

    /// The server sends `message` on the newest socket.
    fn push(&self, message: ServerMessage<ConvexValue>) {
        self.listener(self.opens() - 1)
            .on_text(Value::from(message).to_string());
    }

    fn advance(&self, ms: u64) {
        self.clock.0.fetch_add(ms, Ordering::SeqCst);
        self.client.tick();
    }

    /// Steps 100 ms at a time until there are `count` sockets.
    fn run_until_opens(&self, count: usize) {
        for _ in 0..1200 {
            if self.opens() >= count {
                return;
            }
            self.advance(100);
        }
        panic!("no socket {count} after 120 s");
    }
}

fn version(query_set: u32, identity: u32, ts: u64) -> StateVersion {
    StateVersion {
        query_set,
        identity,
        ts: Timestamp::try_from(ts).unwrap(),
    }
}

fn transition(
    start: StateVersion,
    end: StateVersion,
    modifications: Vec<StateModification<ConvexValue>>,
) -> ServerMessage<ConvexValue> {
    ServerMessage::Transition {
        start_version: start,
        end_version: end,
        modifications,
        client_clock_skew: None,
        server_ts: None,
    }
}

fn updated(query_id: u32, value: Value) -> StateModification<ConvexValue> {
    StateModification::QueryUpdated {
        query_id: QueryId::new(query_id),
        value: ConvexValue(value),
        log_lines: LogLinesMessage(Vec::new()),
        journal: None,
    }
}

fn mutation_response(request_id: u32, value: Value, ts: u64) -> ServerMessage<ConvexValue> {
    ServerMessage::MutationResponse {
        request_id,
        result: Ok(ConvexValue(value)),
        ts: Some(Timestamp::try_from(ts).unwrap()),
        log_lines: LogLinesMessage(Vec::new()),
    }
}

fn auth_error() -> ServerMessage<ConvexValue> {
    ServerMessage::AuthError {
        error_message: "token rejected".into(),
        base_version: Some(0),
        auth_update_attempted: Some(false),
    }
}

type Updates = Arc<Mutex<Vec<Result<Value, CallError>>>>;
type OnUpdate = Box<dyn Fn(Result<Value, CallError>) + Send + Sync>;

fn recorder() -> (Updates, OnUpdate) {
    let updates = Updates::default();
    let sink = updates.clone();
    (
        updates,
        Box::new(move |update| sink.lock().unwrap().push(update)),
    )
}

fn mutations(messages: &[ClientMessage]) -> Vec<u32> {
    messages
        .iter()
        .filter_map(|message| match message {
            ClientMessage::Mutation { request_id, .. } => Some(*request_id),
            _ => None,
        })
        .collect()
}

#[test]
fn connect_sends_connect_then_authenticate() {
    let harness = Harness::new();
    harness.client.set_auth(Some("jwt-1".into()));
    assert_eq!(harness.opens(), 1);
    assert_eq!(
        harness.net.opens.lock().unwrap()[0].url,
        "wss://convex.example/api/sync"
    );
    assert!(harness.sent(0).is_empty(), "nothing before on_open");
    harness.connect();
    let sent = harness.sent(0);
    match &sent[0] {
        ClientMessage::Connect {
            connection_count,
            last_close_reason,
            max_observed_timestamp,
            ..
        } => {
            assert_eq!(*connection_count, 0);
            assert_eq!(last_close_reason, "InitialConnect");
            assert_eq!(*max_observed_timestamp, None);
        }
        other => panic!("first frame {other:?}"),
    }
    assert_eq!(
        sent[1],
        ClientMessage::Authenticate {
            base_version: 0,
            token: AuthenticationToken::User("jwt-1".into()),
        }
    );
    assert!(matches!(
        sent[2],
        ClientMessage::ModifyQuerySet {
            base_version: 0,
            new_version: 1,
            ..
        }
    ));
}

#[test]
fn subscription_receives_transition_update() {
    let harness = Harness::new();
    harness.connect();
    let (updates, on_update) = recorder();
    harness
        .client
        .subscribe("messages:list", json!({ "limit": 50.0 }), on_update);
    match harness.last_sent() {
        ClientMessage::ModifyQuerySet {
            base_version: 1,
            new_version: 2,
            modifications,
        } => match &modifications[..] {
            [QuerySetModification::Add(query)] => {
                assert_eq!(query.query_id, QueryId::new(0));
                assert_eq!(String::from(query.udf_path.clone()), "messages:list");
            }
            other => panic!("modifications {other:?}"),
        },
        other => panic!("last frame {other:?}"),
    }
    let page = json!([{ "body": "hi" }]);
    harness.push(transition(
        version(0, 0, 0),
        version(2, 1, 10),
        vec![updated(0, page.clone())],
    ));
    assert_eq!(*updates.lock().unwrap(), vec![Ok(page.clone())]);
    // A transition that leaves the query alone is not an update.
    harness.push(transition(version(2, 1, 10), version(2, 1, 11), vec![]));
    assert_eq!(updates.lock().unwrap().len(), 1);
}

#[test]
fn mutation_resolves_after_transition_reaches_its_ts() {
    let harness = Harness::new();
    harness.connect();
    harness.push(transition(version(0, 0, 0), version(1, 1, 5), vec![]));
    let mut sending = Box::pin(
        harness
            .client
            .mutation("messages:send", json!({ "body": "x" })),
    );
    assert!(sending.as_mut().now_or_never().is_none());
    assert_eq!(mutations(&harness.sent(0)), vec![0]);
    harness.push(mutation_response(0, json!("id1"), 20));
    assert!(sending.as_mut().now_or_never().is_none(), "waits for ts 20");
    harness.push(transition(version(1, 1, 5), version(1, 1, 20), vec![]));
    assert_eq!(sending.as_mut().now_or_never(), Some(Ok(json!("id1"))));
}

#[test]
fn resend_after_reconnect_does_not_duplicate() {
    let harness = Harness::new();
    harness.connect();
    harness.push(transition(version(0, 0, 0), version(1, 1, 5), vec![]));
    let mut sending = Box::pin(
        harness
            .client
            .mutation("messages:send", json!({ "body": "x" })),
    );
    assert!(sending.as_mut().now_or_never().is_none());
    harness.listener(0).on_closed("gone".into());
    harness.run_until_opens(2);
    harness.connect();
    let sent = harness.sent(1);
    match &sent[0] {
        ClientMessage::Connect {
            connection_count,
            last_close_reason,
            max_observed_timestamp,
            ..
        } => {
            assert_eq!(*connection_count, 1);
            assert_eq!(last_close_reason, "gone");
            assert_eq!(
                *max_observed_timestamp,
                Some(Timestamp::try_from(5u64).unwrap())
            );
        }
        other => panic!("first frame {other:?}"),
    }
    assert_eq!(mutations(&harness.sent(0)), vec![0]);
    assert_eq!(mutations(&sent), vec![0]);
    // The dropped socket's late frames change nothing.
    harness
        .listener(0)
        .on_text(Value::from(mutation_response(0, json!("stale"), 6)).to_string());
    assert!(sending.as_mut().now_or_never().is_none());
    harness.push(mutation_response(0, json!("id1"), 30));
    harness.push(transition(version(0, 0, 0), version(1, 1, 30), vec![]));
    assert_eq!(sending.as_mut().now_or_never(), Some(Ok(json!("id1"))));
    assert_eq!(mutations(&harness.sent(1)), vec![0]);
}

#[test]
fn reconnect_backoff_doubles_to_30s() {
    let harness = Harness::new();
    for count in 2..=9 {
        harness.listener(count - 2).on_closed("refused".into());
        harness.run_until_opens(count);
    }
    let opens = harness.net.opens.lock().unwrap();
    let gaps: Vec<u64> = opens
        .windows(2)
        .map(|pair| pair[1].at - pair[0].at)
        .collect();
    assert_eq!(gaps, vec![500, 1000, 2000, 4000, 8000, 16000, 30000, 30000]);
}

#[test]
fn silence_for_30s_reconnects() {
    let harness = Harness::new();
    harness.connect();
    harness.advance(20_000);
    harness.push(ServerMessage::Ping);
    harness.advance(29_900);
    assert!(!harness.socket(0).closed.load(Ordering::SeqCst));
    harness.advance(100);
    assert!(harness.socket(0).closed.load(Ordering::SeqCst));
    harness.run_until_opens(2);
    harness.connect();
    match &harness.sent(1)[0] {
        ClientMessage::Connect {
            last_close_reason, ..
        } => assert_eq!(last_close_reason, "InactiveServer"),
        other => panic!("first frame {other:?}"),
    }
}

#[test]
fn unsubscribe_sends_query_removal() {
    let harness = Harness::new();
    harness.connect();
    let (_updates, on_update) = recorder();
    let id = harness
        .client
        .subscribe("messages:list", json!({}), on_update);
    harness.client.unsubscribe(id);
    assert_eq!(
        harness.last_sent(),
        ClientMessage::ModifyQuerySet {
            base_version: 2,
            new_version: 3,
            modifications: vec![QuerySetModification::Remove {
                query_id: QueryId::new(0)
            }],
        }
    );
}

#[test]
fn action_resolves_on_its_response() {
    let harness = Harness::new();
    harness.connect();
    let mut acting = Box::pin(harness.client.action("mail:send", json!({ "to": "a" })));
    assert!(acting.as_mut().now_or_never().is_none());
    assert!(matches!(
        harness.last_sent(),
        ClientMessage::Action { request_id: 0, ref udf_path, .. } if udf_path.to_string() == "mail:send"
    ));
    // No transition needed: an action has no state to wait for.
    harness.push(ServerMessage::ActionResponse {
        request_id: 0,
        result: Ok(ConvexValue(json!({ "sent": true }))),
        log_lines: LogLinesMessage(Vec::new()),
    });
    assert_eq!(
        acting.as_mut().now_or_never(),
        Some(Ok(json!({ "sent": true })))
    );
}

#[test]
fn auth_error_drops_the_token_reconnects_and_tells_the_owner() {
    let harness = Harness::new();
    harness.client.set_auth(Some("jwt-1".into()));
    harness.connect();
    let mut sending = Box::pin(harness.client.mutation("account:view", json!({})));
    assert!(sending.as_mut().now_or_never().is_none());
    harness.push(auth_error());
    assert!(harness.socket(0).closed.load(Ordering::SeqCst));
    assert_eq!(harness.auth_errors.load(Ordering::SeqCst), 1);
    harness.client.tick();
    assert_eq!(harness.opens(), 2, "reconnects at once");
    harness.connect();
    let sent = harness.sent(1);
    assert!(
        !sent
            .iter()
            .any(|message| matches!(message, ClientMessage::Authenticate { .. })),
        "the rejected token is not sent again"
    );
    assert_eq!(mutations(&sent), vec![0], "the call is sent again");
    harness.client.set_auth(Some("jwt-2".into()));
    assert!(matches!(
        harness.last_sent(),
        ClientMessage::Authenticate { token: AuthenticationToken::User(ref jwt), .. } if jwt == "jwt-2"
    ));
}

#[test]
fn server_error_code_reaches_the_caller() {
    let harness = Harness::new();
    harness.connect();
    let mut sending = Box::pin(harness.client.mutation("users:setNickname", json!({})));
    assert!(sending.as_mut().now_or_never().is_none());
    harness.push(ServerMessage::MutationResponse {
        request_id: 0,
        result: Err(convex_sync_types::ErrorPayload::ErrorData {
            message: "Uncaught ConvexError".into(),
            data: ConvexValue(json!({ "code": "nickname_taken" })),
        }),
        ts: None,
        log_lines: LogLinesMessage(Vec::new()),
    });
    assert_eq!(
        sending.as_mut().now_or_never(),
        Some(Err(CallError::Convex {
            message: "Uncaught ConvexError".into(),
            data: json!({ "code": "nickname_taken" }),
        }))
    );
}

#[test]
fn value_refuses_tagged_forms_but_bytes() {
    assert!(ConvexValue::try_from(json!({ "a": [1.5, "x", null] })).is_ok());
    let read = ConvexValue::try_from(json!({ "n": 1.0, "m": [-2.0, 1.5] })).unwrap();
    assert_eq!(read.0, json!({ "n": 1, "m": [-2, 1.5] }));
    assert_eq!(
        serde_json::from_value::<u32>(read.0["n"].clone()).unwrap(),
        1
    );
    assert!(ConvexValue::try_from(json!({ "n": { "$integer": "AQAAAAAAAAA=" } })).is_err());
    let packed = json!({ "$bytes": "AQID" });
    assert!(ConvexValue::try_from(packed.clone()).is_ok());
    assert_eq!(value::bytes(&packed), Some(vec![1, 2, 3]));
    assert_eq!(value::bytes(&json!({ "data": 1 })), None);
}

#[test]
fn query_answers_its_first_result_then_unsubscribes() {
    let harness = Harness::new();
    harness.connect();
    let mut asking = Box::pin(harness.client.query("udp:keys", json!({})));
    assert!(asking.as_mut().now_or_never().is_none());
    harness.push(transition(
        version(0, 0, 0),
        version(1, 0, 10),
        vec![updated(0, json!({ "host": "h" }))],
    ));
    assert_eq!(
        asking.as_mut().now_or_never(),
        Some((Ok(json!({ "host": "h" })), Served::Server))
    );
    assert!(matches!(
        harness.last_sent(),
        ClientMessage::ModifyQuerySet { ref modifications, .. }
            if matches!(modifications[..], [QuerySetModification::Remove { .. }])
    ));
}

#[test]
fn a_query_of_a_watched_query_is_answered_here() {
    let harness = Harness::new();
    harness.connect();
    let (_updates, on_update) = recorder();
    let args = json!({ "id": "kxkjq926sc" });
    harness
        .client
        .subscribe("mail:inbox", args.clone(), on_update);
    harness.push(transition(
        version(0, 0, 0),
        version(2, 1, 10),
        vec![updated(0, json!({ "sealed": "s" }))],
    ));
    let frames = harness.sent(harness.opens() - 1).len();
    let mut asking = Box::pin(harness.client.query("mail:inbox", args));
    assert_eq!(
        asking.as_mut().now_or_never(),
        Some((Ok(json!({ "sealed": "s" })), Served::Local))
    );
    // Nothing went to the server: not the query, nor its removal (the watch still holds it).
    assert_eq!(harness.sent(harness.opens() - 1).len(), frames);
}

#[test]
fn close_fails_pending_calls() {
    let harness = Harness::new();
    harness.connect();
    let mut sending = Box::pin(harness.client.mutation("auth:token", json!({})));
    assert!(sending.as_mut().now_or_never().is_none());
    harness.client.close();
    assert_eq!(
        sending.as_mut().now_or_never(),
        Some(Err(CallError::Closed))
    );
    assert!(harness.client.closed());
    let later = harness
        .client
        .action("vtop:fetch", json!({}))
        .now_or_never();
    assert_eq!(later, Some(Err(CallError::Closed)));
}

/// Every event and span field, as text.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<String>>);

impl Visit for Captured {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let _ = writeln!(self.0.lock().unwrap(), "{}={value:?}", field.name());
    }
}

impl<S: Subscriber> Layer<S> for Captured {
    fn on_new_span(&self, attrs: &Attributes<'_>, _id: &Id, _ctx: Context<'_, S>) {
        attrs.record(&mut self.clone());
    }
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        event.record(&mut self.clone());
    }
}

#[test]
fn logs_never_contain_token() {
    let captured = Captured::default();
    // A second dispatcher, as in consent's test: callsites then ask each one, so other tests'
    // threads can't leave them cached as "never".
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let subscriber = tracing_subscriber::registry().with(captured.clone());
    tracing::subscriber::with_default(subscriber, || {
        let harness = Harness::new();
        harness.client.set_auth(Some("secret-jwt-one".into()));
        harness.connect();
        let mut sending = Box::pin(
            harness
                .client
                .mutation("auth:token", json!({ "payload": "private body" })),
        );
        assert!(sending.as_mut().now_or_never().is_none());
        harness.push(mutation_response(0, json!({ "jwt": "secret-jwt-two" }), 9));
        harness.push(transition(version(0, 0, 0), version(1, 1, 9), vec![]));
        harness.push(auth_error());
        harness.client.tick();
        harness.connect();
        harness.listener(1).on_closed("gone".into());
        harness.run_until_opens(3);
        harness.client.close();
    });
    let logs = captured.0.lock().unwrap();
    assert!(!logs.is_empty(), "something was logged");
    assert!(!logs.contains("secret-jwt"), "{logs}");
    assert!(!logs.contains("private body"), "{logs}");
}

#[test]
fn socket_calling_listener_synchronously_does_not_deadlock() {
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let harness = Harness::build(true);
        // `open` called on_open from inside: Connect went out once the socket was stored.
        assert!(matches!(harness.sent(0)[0], ClientMessage::Connect { .. }));
        harness.connect();
        assert_eq!(
            harness
                .sent(0)
                .iter()
                .filter(|message| matches!(message, ClientMessage::Connect { .. }))
                .count(),
            1,
            "a second on_open is ignored"
        );

        // A failed send calls on_closed from inside, then close() does too.
        harness.socket(0).fail_sends.store(true, Ordering::SeqCst);
        let (_updates, on_update) = recorder();
        harness
            .client
            .subscribe("messages:list", json!({}), on_update);
        assert!(harness.socket(0).closed.load(Ordering::SeqCst));
        harness.run_until_opens(2);
        let sent = harness.sent(1);
        match &sent[0] {
            ClientMessage::Connect {
                last_close_reason, ..
            } => assert_eq!(last_close_reason, "send failed"),
            other => panic!("first frame {other:?}"),
        }
        assert!(sent.iter().any(|message| matches!(
            message,
            ClientMessage::ModifyQuerySet { modifications, .. } if !modifications.is_empty()
        )));

        // The silence watchdog's close() calls on_closed from inside as well.
        harness.advance(SILENCE);
        assert!(harness.socket(1).closed.load(Ordering::SeqCst));
        harness.run_until_opens(3);
        harness.client.close();
        done.send(()).unwrap();
    });
    finished
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("no deadlock");
}

#[test]
fn an_idle_socket_closes_and_the_next_call_opens_one() {
    let harness = Harness::new();
    harness.connect();
    for _ in 0..2 {
        harness.advance(20_000);
        harness.push(ServerMessage::Ping);
    }
    harness.advance(19_900);
    assert!(!harness.socket(0).closed.load(Ordering::SeqCst));
    harness.advance(100);
    assert!(
        harness.socket(0).closed.load(Ordering::SeqCst),
        "idle for 60 s"
    );
    harness.advance(120_000);
    assert_eq!(harness.opens(), 1, "no socket while idle");
    let mut sending = Box::pin(harness.client.mutation("auth:token", json!({})));
    assert!(sending.as_mut().now_or_never().is_none());
    harness.client.tick();
    assert_eq!(harness.opens(), 2, "a call opens one");
    harness.connect();
    assert_eq!(mutations(&harness.sent(1)), vec![0]);
    // A call waiting for its answer keeps the socket.
    for _ in 0..4 {
        harness.advance(20_000);
        harness.push(ServerMessage::Ping);
    }
    assert!(!harness.socket(1).closed.load(Ordering::SeqCst));
}

fn failed(query_id: u32, code: &str) -> StateModification<ConvexValue> {
    StateModification::QueryFailed {
        query_id: QueryId::new(query_id),
        error_message: "Uncaught ConvexError".into(),
        log_lines: LogLinesMessage(Vec::new()),
        journal: None,
        error_data: Some(ConvexValue(json!({ "code": code }))),
    }
}

#[test]
fn a_query_given_up_on_unsubscribes() {
    let harness = Harness::new();
    harness.connect();
    let mut asking = Box::pin(harness.client.query("udp:keys", json!({})));
    assert!(asking.as_mut().now_or_never().is_none());
    drop(asking);
    assert!(matches!(
        harness.last_sent(),
        ClientMessage::ModifyQuerySet { ref modifications, .. }
            if matches!(modifications[..], [QuerySetModification::Remove { .. }])
    ));
}

#[test]
fn a_mutation_given_up_on_is_not_sent_again() {
    let harness = Harness::new();
    harness.connect();
    let mut sending = Box::pin(harness.client.mutation("mail:remove", json!({})));
    assert!(sending.as_mut().now_or_never().is_none());
    drop(sending);
    harness.listener(0).on_closed("gone".into());
    harness.run_until_opens(2);
    harness.connect();
    assert_eq!(mutations(&harness.sent(0)), vec![0]);
    assert_eq!(mutations(&harness.sent(1)), Vec::<u32>::new());
}

#[test]
fn a_failed_watch_hears_its_error_again_on_the_next_socket() {
    let harness = Harness::new();
    harness.connect();
    let (updates, on_update) = recorder();
    harness
        .client
        .subscribe("account:view", json!({}), on_update);
    harness.push(transition(
        version(0, 0, 0),
        version(1, 0, 10),
        vec![failed(0, "unauthenticated")],
    ));
    assert_eq!(updates.lock().unwrap().len(), 1);
    harness.listener(0).on_closed("offline".into());
    harness.run_until_opens(2);
    harness.connect();
    harness.push(transition(
        version(0, 0, 0),
        version(1, 0, 20),
        vec![failed(0, "unauthenticated")],
    ));
    assert_eq!(
        updates.lock().unwrap().len(),
        2,
        "heard again, so its owner renews"
    );
}

#[test]
fn a_failure_cached_before_a_new_jwt_waits_for_the_server() {
    let harness = Harness::new();
    harness.connect();
    let (watched, on_watch) = recorder();
    harness
        .client
        .subscribe("account:view", json!({}), on_watch);
    harness.push(transition(
        version(0, 0, 0),
        version(1, 0, 10),
        vec![failed(0, "unauthenticated")],
    ));
    assert_eq!(watched.lock().unwrap().len(), 1);
    harness.client.set_auth(Some("new-jwt".into()));
    // The same query read again: the expired JWT's failure isn't its answer.
    let (read, on_read) = recorder();
    harness.client.subscribe("account:view", json!({}), on_read);
    assert!(read.lock().unwrap().is_empty());
    harness.push(transition(
        version(1, 0, 10),
        version(2, 1, 20),
        vec![updated(0, json!({ "regno": "23ABC0001" }))],
    ));
    let read = read.lock().unwrap();
    assert_eq!(read.len(), 1);
    assert!(read[0].is_ok(), "{read:?}");
}

#[test]
fn auth_error_reconnects_immediately_without_inflating_backoff() {
    let harness = Harness::new();
    harness.connect();
    // Repeated AuthErrors reconnect immediately and must not double backoff_ms.
    for i in 0..3 {
        harness.push(auth_error());
        harness.client.tick();
        assert_eq!(harness.opens(), i + 2);
        harness.connect();
    }
    // The next real disconnect still starts at INITIAL_BACKOFF_MS (500 ms), not 4000 ms.
    harness.listener(3).on_closed("network".into());
    harness.advance(499);
    assert_eq!(harness.opens(), 4);
    harness.advance(1);
    assert_eq!(harness.opens(), 5);
}
