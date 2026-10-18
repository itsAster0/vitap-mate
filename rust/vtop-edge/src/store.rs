//! Durable Objects that hold one record each.
//!
//! `Mailbox`: a forwarding address, named by its id. A delivered box is
//! cleared by an alarm once it can no longer be collected. While the
//! engine signs its student in, it waits here for the next OTP, sealed to a
//! key made for that sign-in (a "waiter").
//! `Index`: one lookup value, named by what it indexes (`owner:<hash>`).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::time::Duration;

use futures_channel::oneshot;
use futures_util::future::{select, Either};

use serde::Deserialize;
use vtop_shared::mailbox::SEALED_TTL_SECS;
use worker::{
    durable_object, DurableObject, Env, Method, Request, Response, Result, SqlStorage, State,
};

use crate::sql;
use crate::state::{MailboxDoc, Waiter, WAITER_TTL_SECS};
use crate::{error, now};

/// The longest one wait lasts.
const MAX_WAIT_SECS: u64 = 30;

#[durable_object]
pub struct Mailbox {
    state: State,
    ready: Cell<bool>,
    /// Engine waits parked until a code is handed to their waiter, by
    /// waiter key. Requests to one object share this instance.
    wake: RefCell<HashMap<String, Vec<oneshot::Sender<()>>>>,
}

const MAILBOX_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS mailbox (
    id TEXT PRIMARY KEY,
    secret_hash TEXT NOT NULL,
    public_key TEXT NOT NULL,
    device_id TEXT NOT NULL,
    owner_hash TEXT,
    sealed TEXT,
    sealed_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS waiters (
    public_key TEXT PRIMARY KEY,
    owner_hash TEXT NOT NULL,
    since INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    results TEXT NOT NULL
)";

fn mailbox_from(row: &sql::Row) -> Option<MailboxDoc> {
    Some(MailboxDoc {
        id: sql::text(row, "id")?,
        secret_hash: sql::text(row, "secret_hash")?,
        public_key: sql::text(row, "public_key")?,
        device_id: sql::text(row, "device_id")?,
        owner_hash: sql::text(row, "owner_hash"),
        sealed: sql::text(row, "sealed"),
        sealed_at: sql::int(row, "sealed_at"),
        created_at: sql::int(row, "created_at")?,
        updated_at: sql::int(row, "updated_at")?,
    })
}

fn waiter_from(row: &sql::Row) -> Option<Waiter> {
    Some(Waiter {
        public_key: sql::text(row, "public_key")?,
        owner_hash: sql::text(row, "owner_hash")?,
        since: sql::int(row, "since")?,
        expires_at: sql::int(row, "expires_at")?,
        results: serde_json::from_str(&sql::text(row, "results")?).ok()?,
    })
}

impl Mailbox {
    async fn db(&self) -> Result<SqlStorage> {
        let db = self.state.storage().sql();
        if !self.ready.get() {
            if sql::needs_schema(&db)? {
                // The address itself is kept, so Gmail needs no change.
                let old: Option<MailboxDoc> = self.state.storage().get("doc").await.ok().flatten();
                self.state.storage().delete_all().await?;
                sql::create(&db, MAILBOX_SCHEMA)?;
                if let Some(doc) = old {
                    write_mailbox(&db, &doc)?;
                }
            }
            self.ready.set(true);
        }
        Ok(db)
    }

    async fn doc(&self) -> Result<Option<MailboxDoc>> {
        let db = self.db().await?;
        Ok(sql::first(&db, "SELECT * FROM mailbox LIMIT 1", vec![])?
            .as_ref()
            .and_then(mailbox_from))
    }

    /// Wipes the address, tables and all.
    async fn delete_all(&self) -> Result<()> {
        self.state.storage().delete_all().await?;
        self.ready.set(false);
        Ok(())
    }

    async fn save(&self, doc: &MailboxDoc) -> Result<()> {
        write_mailbox(&self.db().await?, doc)?;
        if let Some(at) = doc.sealed_at {
            let due = (at + SEALED_TTL_SECS + 1).saturating_sub(now());
            self.state
                .storage()
                .set_alarm(Duration::from_secs(due.max(1)))
                .await?;
        }
        Ok(())
    }
}

fn write_mailbox(db: &SqlStorage, doc: &MailboxDoc) -> Result<()> {
    sql::run(db, "DELETE FROM mailbox", vec![])?;
    sql::run(
        db,
        "INSERT INTO mailbox (id, secret_hash, public_key, device_id, owner_hash, sealed,
             sealed_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        vec![
            sql::v_text(&doc.id),
            sql::v_text(&doc.secret_hash),
            sql::v_text(&doc.public_key),
            sql::v_text(&doc.device_id),
            sql::v_opt_text(doc.owner_hash.as_deref()),
            sql::v_opt_text(doc.sealed.as_deref()),
            sql::v_opt_int(doc.sealed_at),
            sql::v_int(doc.created_at),
            sql::v_int(doc.updated_at),
        ],
    )
}

impl DurableObject for Mailbox {
    fn new(state: State, _env: Env) -> Self {
        Self {
            state,
            ready: Cell::new(false),
            wake: RefCell::new(HashMap::new()),
        }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        let path = req.path();
        match (req.method(), path.as_str()) {
            (Method::Get, "/doc") => match self.doc().await? {
                Some(doc) => Response::from_json(&doc),
                None => error(404, "mailbox_not_found", "No such forwarding address."),
            },
            (Method::Put, "/doc") => {
                let doc: MailboxDoc = req.json().await?;
                self.save(&doc).await?;
                Ok(Response::empty()?.with_status(204))
            }
            // Create only if new, so a clash of random ids never overwrites.
            (Method::Post, "/create") => {
                if self.doc().await?.is_some() {
                    return error(409, "exists", "Mailbox exists.");
                }
                let doc: MailboxDoc = req.json().await?;
                self.save(&doc).await?;
                Ok(Response::empty()?.with_status(201))
            }
            (Method::Post, "/deliver") => {
                #[derive(Deserialize)]
                struct Deliver {
                    sealed: String,
                }
                let body: Deliver = req.json().await?;
                let Some(mut doc) = self.doc().await? else {
                    return error(404, "mailbox_not_found", "No such forwarding address.");
                };
                let at = now();
                doc.sealed = Some(body.sealed);
                doc.sealed_at = Some(at);
                doc.updated_at = at;
                self.save(&doc).await?;
                Response::from_json(&doc)
            }
            (Method::Delete, "/doc") => {
                self.delete_all().await?;
                Ok(Response::empty()?.with_status(204))
            }
            (Method::Post, "/await") => self.await_code(req.json().await?).await,
            (Method::Get, "/waiters") => Response::from_json(&self.waiters().await?),
            (Method::Post, "/waiter-result") => {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase")]
                struct Delivered {
                    public_key: String,
                    sealed: String,
                }
                let body: Delivered = req.json().await?;
                if let Some(mut waiter) = self.waiter(&body.public_key).await? {
                    waiter.results.push(body.sealed);
                    self.put_waiter(&waiter).await?;
                    let parked = self.wake.borrow_mut().remove(&body.public_key);
                    for waiting in parked.into_iter().flatten() {
                        let _ = waiting.send(());
                    }
                }
                Ok(Response::empty()?.with_status(204))
            }
            _ => error(404, "not_found", "No such route."),
        }
    }

    async fn alarm(&self) -> Result<Response> {
        self.waiters().await?;
        if let Some(mut doc) = self.doc().await? {
            if doc.fresh_sealed(now()).is_none() && doc.sealed.is_some() {
                doc.sealed = None;
                doc.sealed_at = None;
                write_mailbox(&self.db().await?, &doc)?;
            }
        }
        Response::empty()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AwaitBody {
    public_key: String,
    owner_hash: String,
    since: u64,
    wait_secs: u64,
}

impl Mailbox {
    async fn waiter(&self, public_key: &str) -> Result<Option<Waiter>> {
        let db = self.db().await?;
        Ok(sql::first(
            &db,
            "SELECT * FROM waiters WHERE public_key = ?",
            vec![sql::v_text(public_key)],
        )?
        .as_ref()
        .and_then(waiter_from))
    }

    async fn put_waiter(&self, waiter: &Waiter) -> Result<()> {
        sql::run(
            &self.db().await?,
            "INSERT OR REPLACE INTO waiters (public_key, owner_hash, since, expires_at, results)
             VALUES (?, ?, ?, ?, ?)",
            vec![
                sql::v_text(&waiter.public_key),
                sql::v_text(&waiter.owner_hash),
                sql::v_int(waiter.since),
                sql::v_int(waiter.expires_at),
                sql::v_text(&serde_json::to_string(&waiter.results)?),
            ],
        )
    }

    /// Live waiters; expired ones are dropped on the way.
    async fn waiters(&self) -> Result<Vec<Waiter>> {
        let db = self.db().await?;
        sql::run(
            &db,
            "DELETE FROM waiters WHERE expires_at <= ?",
            vec![sql::v_int(now())],
        )?;
        Ok(sql::rows(&db, "SELECT * FROM waiters", vec![])?
            .iter()
            .filter_map(waiter_from)
            .collect())
    }

    /// Registers the waiter if new, then hands out the oldest code sealed
    /// for it, waiting up to `wait_secs` for one. A wait of 0 only
    /// registers, so mail that comes before the first wait is not missed.
    async fn await_code(&self, body: AwaitBody) -> Result<Response> {
        if self.doc().await?.is_none() {
            return error(404, "mailbox_not_found", "No such forwarding address.");
        }
        let key = body.public_key.clone();
        if self.waiter(&key).await?.is_none() {
            let waiter = Waiter {
                public_key: body.public_key.clone(),
                owner_hash: body.owner_hash,
                since: body.since,
                expires_at: now() + WAITER_TTL_SECS,
                results: Vec::new(),
            };
            self.put_waiter(&waiter).await?;
            if self.state.storage().get_alarm().await?.is_none() {
                self.state
                    .storage()
                    .set_alarm(Duration::from_secs(WAITER_TTL_SECS + 1))
                    .await?;
            }
        }
        let deadline = now() + body.wait_secs.min(MAX_WAIT_SECS);
        loop {
            if let Some(mut waiter) = self.waiter(&key).await? {
                if !waiter.results.is_empty() {
                    let sealed = waiter.results.remove(0);
                    self.put_waiter(&waiter).await?;
                    return Response::from_json(&serde_json::json!({ "sealed": sealed }));
                }
            }
            let left = deadline.saturating_sub(now());
            if left == 0 {
                return Response::from_json(&serde_json::json!({ "sealed": null }));
            }
            // Sleeps until /waiter-result wakes it or time runs out, then
            // looks again.
            let (woken, parked) = oneshot::channel();
            self.wake
                .borrow_mut()
                .entry(key.clone())
                .or_default()
                .push(woken);
            let timeout = worker::Delay::from(Duration::from_secs(left));
            if let Either::Right(_) = select(parked, timeout).await {
                if let Some(list) = self.wake.borrow_mut().get_mut(&key) {
                    list.retain(|sender| !sender.is_canceled());
                }
            }
        }
    }
}

#[durable_object]
pub struct Index {
    state: State,
    ready: Cell<bool>,
}

const INDEX_SCHEMA: &str = "CREATE TABLE IF NOT EXISTS entry (value TEXT NOT NULL)";

impl Index {
    async fn db(&self) -> Result<SqlStorage> {
        let db = self.state.storage().sql();
        if !self.ready.get() {
            if sql::needs_schema(&db)? {
                let old: Option<String> = self.state.storage().get("value").await.ok().flatten();
                self.state.storage().delete_all().await?;
                sql::create(&db, INDEX_SCHEMA)?;
                if let Some(value) = old {
                    sql::run(
                        &db,
                        "INSERT INTO entry (value) VALUES (?)",
                        vec![sql::v_text(&value)],
                    )?;
                }
            }
            self.ready.set(true);
        }
        Ok(db)
    }
}

impl DurableObject for Index {
    fn new(state: State, _env: Env) -> Self {
        Self {
            state,
            ready: Cell::new(false),
        }
    }

    async fn fetch(&self, mut req: Request) -> Result<Response> {
        let db = self.db().await?;
        let current = sql::first(&db, "SELECT value FROM entry LIMIT 1", vec![])?
            .and_then(|row| sql::text(&row, "value"));
        match req.method() {
            Method::Get => match current {
                Some(value) => Response::ok(value),
                None => error(404, "not_found", "No entry."),
            },
            Method::Put => {
                let value = req.text().await?;
                sql::run(&db, "DELETE FROM entry", vec![])?;
                sql::run(
                    &db,
                    "INSERT INTO entry (value) VALUES (?)",
                    vec![sql::v_text(&value)],
                )?;
                Ok(Response::empty()?.with_status(204))
            }
            // Deletes only while it still holds the value given.
            Method::Delete => {
                let expected = req.text().await?;
                if current.as_deref() == Some(expected.as_str()) {
                    sql::run(&db, "DELETE FROM entry", vec![])?;
                }
                Ok(Response::empty()?.with_status(204))
            }
            _ => error(405, "method_not_allowed", "Method not allowed."),
        }
    }
}
