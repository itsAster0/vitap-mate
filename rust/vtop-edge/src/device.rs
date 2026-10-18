//! One Durable Object per app install: its FCM token, its event log and,
//! while the app is in front, its WebSocket. Every event gets the next
//! sequence number and goes out over the socket and FCM; the app applies
//! each number once and reads what it missed from `/v1/events?after=`.

use std::cell::Cell;
use std::time::Duration;

use serde::Deserialize;
use serde_json::json;
use vtop_shared::events::{Event, EventBody, ServerFrame, EVENT_TTL_SECS};
use vtop_shared::tokens;
use worker::{
    durable_object, DurableObject, Env, Method, Request, Response, Result, SqlStorage, State,
    WebSocket, WebSocketIncomingMessage, WebSocketPair,
};

use crate::fcm::{self, SendOutcome};
use crate::sql;
use crate::state::DeviceDoc;
use crate::{error, now};

/// The most events one catch-up returns; the app asks again from the last.
const PAGE: usize = 100;

#[durable_object]
pub struct Device {
    state: State,
    env: Env,
    ready: Cell<bool>,
}

const DEVICE_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS device (
    id TEXT PRIMARY KEY,
    secret_hash TEXT NOT NULL,
    fcm_token TEXT NOT NULL,
    seq INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS events (
    seq INTEGER PRIMARY KEY,
    at INTEGER NOT NULL,
    kind TEXT NOT NULL,
    event TEXT NOT NULL
)";

fn write_device(db: &SqlStorage, doc: &DeviceDoc) -> Result<()> {
    sql::run(db, "DELETE FROM device", vec![])?;
    sql::run(
        db,
        "INSERT INTO device (id, secret_hash, fcm_token, seq, created_at) VALUES (?, ?, ?, ?, ?)",
        vec![
            sql::v_text(&doc.id),
            sql::v_text(&doc.secret_hash),
            sql::v_text(&doc.fcm_token),
            sql::v_int(doc.seq),
            sql::v_int(doc.created_at),
        ],
    )
}

#[derive(Deserialize)]
struct InitBody {
    id: String,
    secret_hash: String,
    fcm_token: String,
}

#[derive(Deserialize)]
struct FcmBody {
    fcm_token: String,
}

#[derive(Deserialize)]
struct AppendBody {
    body: EventBody,
}

#[derive(Deserialize, Default)]
struct AfterQuery {
    #[serde(default)]
    after: u64,
}

impl Device {
    async fn db(&self) -> Result<SqlStorage> {
        let db = self.state.storage().sql();
        if !self.ready.get() {
            if sql::needs_schema(&db)? {
                // The install is kept, so the app needs no new one; its old
                // events are dropped.
                let old: Option<DeviceDoc> = self.state.storage().get("doc").await.ok().flatten();
                self.state.storage().delete_all().await?;
                sql::create(&db, DEVICE_SCHEMA)?;
                if let Some(doc) = old {
                    write_device(&db, &doc)?;
                }
            }
            self.ready.set(true);
        }
        Ok(db)
    }

    async fn doc(&self) -> Result<Option<DeviceDoc>> {
        let db = self.db().await?;
        Ok(
            sql::first(&db, "SELECT * FROM device LIMIT 1", vec![])?.and_then(|row| {
                Some(DeviceDoc {
                    id: sql::text(&row, "id")?,
                    secret_hash: sql::text(&row, "secret_hash")?,
                    fcm_token: sql::text(&row, "fcm_token")?,
                    seq: sql::int(&row, "seq")?,
                    created_at: sql::int(&row, "created_at")?,
                })
            }),
        )
    }

    async fn save(&self, doc: &DeviceDoc) -> Result<()> {
        write_device(&self.db().await?, doc)
    }

    /// The bearer secret on a client request matches this install's.
    fn authorized(req: &Request, doc: &DeviceDoc) -> bool {
        let presented = req
            .headers()
            .get("authorization")
            .ok()
            .flatten()
            .and_then(|value| {
                value
                    .strip_prefix("Bearer ")
                    .map(str::trim)
                    .map(str::to_string)
            })
            .unwrap_or_default();
        tokens::hashes_equal(&doc.secret_hash, &tokens::hash(&presented))
    }

    async fn events_after(&self, after: u64) -> Result<Vec<Event>> {
        let db = self.db().await?;
        Ok(sql::rows(
            &db,
            "SELECT event FROM events WHERE seq > ? ORDER BY seq LIMIT ?",
            vec![sql::v_int(after), sql::v_int(PAGE as u64)],
        )?
        .iter()
        .filter_map(|row| serde_json::from_str(&sql::text(row, "event")?).ok())
        .collect())
    }

    async fn append(&self, body: EventBody) -> Result<Event> {
        let db = self.db().await?;
        // Numbered in SQL, so appends running at the same time never share
        // a number.
        let seq = sql::first(
            &db,
            "UPDATE device SET seq = seq + 1 RETURNING seq",
            vec![],
        )?
        .and_then(|row| sql::int(&row, "seq"))
        .ok_or("no device")?;
        let doc = self.doc().await?.ok_or("no device")?;
        let event = Event {
            seq,
            at: now(),
            body,
        };
        let value = serde_json::to_value(&event)?;
        let kind = value
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        sql::run(
            &self.db().await?,
            "INSERT OR REPLACE INTO events (seq, at, kind, event) VALUES (?, ?, ?, ?)",
            vec![
                sql::v_int(event.seq),
                sql::v_int(event.at),
                sql::v_text(&kind),
                sql::v_text(&value.to_string()),
            ],
        )?;
        if self.state.storage().get_alarm().await?.is_none() {
            self.state
                .storage()
                .set_alarm(Duration::from_secs(EVENT_TTL_SECS))
                .await?;
        }

        let frame = serde_json::to_string(&ServerFrame::Event {
            event: event.clone(),
        })?;
        for socket in self.state.get_websockets() {
            // A dead socket is closed by the runtime; the app catches up.
            let _ = socket.send_with_str(&frame);
        }
        if !doc.fcm_token.is_empty() && self.push(&doc, &event).await {
            // Google says the token is dead: stop sending to it until the
            // app sends a new one (unless it already has, meanwhile).
            sql::run(
                &self.db().await?,
                "UPDATE device SET fcm_token = '' WHERE fcm_token = ?",
                vec![sql::v_text(&doc.fcm_token)],
            )?;
        }
        Ok(event)
    }

    /// FCM goes out for every event, sockets or not: a socket may be dead
    /// without anyone knowing yet, and the app drops duplicates by `seq`.
    /// True when Google reports the token gone.
    async fn push(&self, doc: &DeviceDoc, event: &Event) -> bool {
        let Ok(serde_json::Value::Object(mut data)) = serde_json::to_value(event) else {
            return false;
        };
        // Older app builds read these names.
        if let Some(kind) = data.remove("kind") {
            data.insert("type".into(), kind);
        }
        data.insert("receivedAt".into(), json!(event.at));
        match fcm::send(&self.env, &doc.fcm_token, &data).await {
            Ok(SendOutcome::Sent) => false,
            Ok(SendOutcome::TokenInvalid) => {
                worker::console_log!("device: FCM token is gone, cleared");
                true
            }
            Ok(SendOutcome::Failed) => {
                worker::console_log!("device: FCM send failed");
                false
            }
            Err(e) => {
                worker::console_log!("device: FCM error: {e}");
                false
            }
        }
    }

    async fn client_events(&self, req: Request) -> Result<Response> {
        let Some(doc) = self.doc().await? else {
            return error(404, "unknown_device", "This install is not registered.");
        };
        if !Self::authorized(&req, &doc) {
            return error(404, "unknown_device", "This install is not registered.");
        }
        let after = req.query::<AfterQuery>().unwrap_or_default().after;
        let upgrade = req
            .headers()
            .get("upgrade")
            .ok()
            .flatten()
            .is_some_and(|value| value.eq_ignore_ascii_case("websocket"));
        if !upgrade {
            let events = self.events_after(after).await?;
            return Response::from_json(&json!({ "latest": doc.seq, "events": events }));
        }

        let pair = WebSocketPair::new()?;
        self.state.accept_web_socket(&pair.server);
        let hello = ServerFrame::Hello { latest: doc.seq };
        pair.server.send_with_str(serde_json::to_string(&hello)?)?;
        for event in self.events_after(after).await? {
            pair.server
                .send_with_str(serde_json::to_string(&ServerFrame::Event { event })?)?;
        }
        Response::from_websocket(pair.client)
    }
}

impl DurableObject for Device {
    fn new(state: State, env: Env) -> Self {
        // Pings are answered without waking the object.
        if let Ok(pair) = worker::worker_sys::WebSocketRequestResponsePair::new("ping", "pong") {
            state.set_websocket_auto_response(&pair);
        }
        Self {
            state,
            env,
            ready: Cell::new(false),
        }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        let path = req.path();
        match (req.method(), path.as_str()) {
            (Method::Get, "/v1/events") => self.client_events(req).await,
            (Method::Post, "/init") => {
                if self.doc().await?.is_some() {
                    return error(409, "exists", "Device exists.");
                }
                let body: InitBody = req.json().await?;
                let doc = DeviceDoc {
                    id: body.id,
                    secret_hash: body.secret_hash,
                    fcm_token: body.fcm_token,
                    seq: 0,
                    created_at: now(),
                };
                self.save(&doc).await?;
                Ok(Response::empty()?.with_status(201))
            }
            (Method::Post, "/auth") => {
                #[derive(Deserialize)]
                struct AuthBody {
                    secret_hash: String,
                }
                let body: AuthBody = req.json().await?;
                match self.doc().await? {
                    Some(doc) if tokens::hashes_equal(&doc.secret_hash, &body.secret_hash) => {
                        Ok(Response::empty()?.with_status(204))
                    }
                    _ => error(404, "unknown_device", "This install is not registered."),
                }
            }
            (Method::Post, "/fcm") => {
                let body: FcmBody = req.json().await?;
                let mut doc = self.doc().await?.ok_or("no device")?;
                doc.fcm_token = body.fcm_token;
                self.save(&doc).await?;
                Ok(Response::empty()?.with_status(204))
            }
            (Method::Post, "/append") => {
                let body: AppendBody = req.json().await?;
                let event = self.append(body.body).await?;
                Response::from_json(&json!({ "seq": event.seq }))
            }
            (Method::Post, "/delete") => {
                for socket in self.state.get_websockets() {
                    let _ = socket.close(Some(1000), Some("deleted"));
                }
                self.state.storage().delete_all().await?;
                self.ready.set(false);
                Ok(Response::empty()?.with_status(204))
            }
            _ => error(404, "not_found", "No such route."),
        }
    }

    /// Drops events older than a day.
    async fn alarm(&self) -> Result<Response> {
        let cutoff = now().saturating_sub(EVENT_TTL_SECS);
        let db = self.db().await?;
        sql::run(&db, "DELETE FROM events WHERE at < ?", vec![sql::v_int(cutoff)])?;
        let remaining = sql::first(&db, "SELECT COUNT(*) AS n FROM events", vec![])?
            .and_then(|row| sql::int(&row, "n"))
            .unwrap_or(0);
        if remaining > 0 {
            self.state
                .storage()
                .set_alarm(Duration::from_secs(EVENT_TTL_SECS))
                .await?;
        }
        Response::empty()
    }

    async fn websocket_message(
        &self,
        _ws: WebSocket,
        _message: WebSocketIncomingMessage,
    ) -> Result<()> {
        // The app only listens; pings are answered by the auto-response.
        Ok(())
    }

    async fn websocket_close(
        &self,
        ws: WebSocket,
        code: usize,
        _reason: String,
        _clean: bool,
    ) -> Result<()> {
        let code = if code == 1005 || code == 1006 {
            1000
        } else {
            code as u16
        };
        let _ = ws.close(Some(code), Some("bye"));
        Ok(())
    }

    async fn websocket_error(&self, _ws: WebSocket, _error: worker::Error) -> Result<()> {
        Ok(())
    }
}
