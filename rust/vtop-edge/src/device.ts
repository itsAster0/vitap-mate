// One Durable Object per app install, named by its device id: its signing
// and sealing keys, its FCM token, its event log and,
// while the app is in front, its WebSocket (the app's own, or the Worker's
// when the app reads server-sent events, see event-stream.ts). Every event gets the next
// sequence number and goes out over the socket and FCM; the app applies
// each number once and reads what it missed from `/v1/events?after=`.

import { DurableObject } from "cloudflare:workers";

import { type Env, error, now } from "./common";
import { send } from "./fcm";
import * as sql from "./sql";

/** How long the log keeps an event for catch-up. */
export const EVENT_TTL_SECS = 24 * 60 * 60;
/** The most events one catch-up returns; the app asks again from the last. */
const PAGE = 100;

/** What an event says. The app reads these names; a cookie request's fields
 * are strings, as FCM data is. */
export type EventBody =
  /** A forwarded mail, sealed to the install's mail key. */
  | { kind: "mail_otp"; mailbox: string; sealed: string }
  /** An app or agent wants a VTOP session: the phone answers with its
   * cookies at `callbackUrl` (a signed request). `wantCredentials` is "1"
   * when saved credentials should come with the answer. */
  | {
      kind: "vtop_cookie_request";
      requestId: string;
      callbackUrl: string;
      wantCredentials: string;
    }
  /** VTOP rejected the password the server kept: sign in again. */
  | { kind: "credentials_rejected" };

/** An event in an install's log; `at` is unix seconds. */
export type Event = { seq: number; at: number } & EventBody;

/** What the edge sends over the socket. `hello` comes once after
 * connecting: events after the client's cursor follow. */
export type ServerFrame = { type: "event"; event: Event } | { type: "hello"; latest: number };

/** A registered install. Keys are base64url: `signKey` (Ed25519) checks its
 * requests, `sealKey` (X25519) is what OTP codes are sealed to. */
export interface DeviceInfo {
  id: string;
  signKey: string;
  sealKey: string;
  label: string | null;
  platform: string;
  /** Empty when there is none, or Google said it is gone. */
  fcmToken: string;
}

interface DeviceDoc extends DeviceInfo {
  /** The last sequence number handed out. */
  seq: number;
}

/** What `/v1/devices/update` may change; absent fields stay. */
export interface DeviceChanges {
  sealKey?: string;
  label?: string;
  fcmToken?: string;
}

const SCHEMA = `
CREATE TABLE device (
    id TEXT PRIMARY KEY,
    sign_key TEXT NOT NULL,
    seal_key TEXT NOT NULL,
    label TEXT,
    platform TEXT NOT NULL,
    fcm_token TEXT NOT NULL,
    seq INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE TABLE events (
    seq INTEGER PRIMARY KEY,
    at INTEGER NOT NULL,
    kind TEXT NOT NULL,
    event TEXT NOT NULL
)`;

export class Device extends DurableObject<Env> {
  private ready = false;

  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    // Pings are answered without waking the object.
    ctx.setWebSocketAutoResponse(new WebSocketRequestResponsePair("ping", "pong"));
  }

  private get db(): SqlStorage {
    const db = this.ctx.storage.sql;
    if (!this.ready) {
      sql.prepare(db, SCHEMA);
      this.ready = true;
    }
    return db;
  }

  private doc(): DeviceDoc | null {
    const row = sql.first(this.db, "SELECT * FROM device LIMIT 1");
    if (!row) return null;
    const id = sql.text(row, "id");
    const signKey = sql.text(row, "sign_key");
    const sealKey = sql.text(row, "seal_key");
    const platform = sql.text(row, "platform");
    const fcmToken = sql.text(row, "fcm_token");
    const seq = sql.int(row, "seq");
    if (id === null || signKey === null || sealKey === null || platform === null || fcmToken === null || seq === null) {
      return null;
    }
    return { id, signKey, sealKey, label: sql.text(row, "label"), platform, fcmToken, seq };
  }

  /** The install, or null when it is not registered. */
  async info(): Promise<DeviceInfo | null> {
    const doc = this.doc();
    if (!doc) return null;
    const { seq: _, ...info } = doc;
    return info;
  }

  /** What the app's Server page shows of the install: no keys or token,
   * only whether it has a token and how many events wait in its log. */
  async summary(): Promise<{ label: string | null; platform: string; pushToken: boolean; createdAt: number; events: number; latest: number } | null> {
    const row = sql.first(this.db, "SELECT label, platform, fcm_token, created_at, seq FROM device LIMIT 1");
    if (!row) return null;
    const count = sql.first(this.db, "SELECT COUNT(*) AS n FROM events");
    return {
      label: sql.text(row, "label"),
      platform: sql.text(row, "platform") ?? "",
      pushToken: Boolean(sql.text(row, "fcm_token")),
      createdAt: sql.int(row, "created_at") ?? 0,
      events: (count && sql.int(count, "n")) ?? 0,
      latest: sql.int(row, "seq") ?? 0,
    };
  }

  /** Forgets the install: keys, FCM token and events. Its sockets close. */
  async remove(): Promise<void> {
    for (const socket of this.ctx.getWebSockets()) {
      try {
        socket.close(1000, "deleted");
      } catch {}
    }
    await this.ctx.storage.deleteAlarm();
    await this.ctx.storage.deleteAll();
    this.ready = false;
  }

  /** Registers the install, or (same `signKey`, so same id) updates its
   * keys, label and platform, and its FCM token when one is sent. True
   * when it is new. */
  async register(info: Omit<DeviceInfo, "fcmToken"> & { fcmToken: string | null }): Promise<boolean> {
    if (this.doc()) {
      this.db.exec(
        "UPDATE device SET seal_key = ?, label = ?, platform = ?, fcm_token = COALESCE(?, fcm_token)",
        info.sealKey,
        info.label,
        info.platform,
        info.fcmToken,
      );
      return false;
    }
    this.db.exec(
      `INSERT INTO device (id, sign_key, seal_key, label, platform, fcm_token, seq, created_at)
       VALUES (?, ?, ?, ?, ?, ?, 0, ?)`,
      info.id,
      info.signKey,
      info.sealKey,
      info.label,
      info.platform,
      info.fcmToken ?? "",
      now(),
    );
    return true;
  }

  async update(changes: DeviceChanges): Promise<void> {
    if (changes.sealKey !== undefined) this.db.exec("UPDATE device SET seal_key = ?", changes.sealKey);
    if (changes.label !== undefined) this.db.exec("UPDATE device SET label = ?", changes.label);
    if (changes.fcmToken !== undefined) this.db.exec("UPDATE device SET fcm_token = ?", changes.fcmToken);
  }

  private eventsAfter(after: number): Event[] {
    return sql
      .rows(this.db, "SELECT event FROM events WHERE seq > ? ORDER BY seq LIMIT ?", after, PAGE)
      .flatMap((row) => {
        const event = sql.text(row, "event");
        return event === null ? [] : [JSON.parse(event) as Event];
      });
  }

  /** Logs the event and sends it out; its number, or null when the
   * install does not exist. */
  async append(body: EventBody): Promise<number | null> {
    // Numbered in SQL, so appends running at the same time never share a
    // number.
    const numbered = sql.first(this.db, "UPDATE device SET seq = seq + 1 RETURNING seq");
    const seq = numbered && sql.int(numbered, "seq");
    const doc = this.doc();
    if (seq === null || !doc) return null;
    const event: Event = { seq, at: now(), ...body };
    this.db.exec(
      "INSERT OR REPLACE INTO events (seq, at, kind, event) VALUES (?, ?, ?, ?)",
      event.seq,
      event.at,
      event.kind,
      JSON.stringify(event),
    );
    if ((await this.ctx.storage.getAlarm()) === null) {
      await this.ctx.storage.setAlarm(Date.now() + EVENT_TTL_SECS * 1000);
    }

    const frame = JSON.stringify({ type: "event", event } satisfies ServerFrame);
    for (const socket of this.ctx.getWebSockets()) {
      // A dead socket is closed by the runtime; the app catches up.
      try {
        socket.send(frame);
      } catch {}
    }
    if (doc.fcmToken && (await this.push(doc.fcmToken, event))) {
      // Google says the token is dead: stop sending to it until the app
      // sends a new one (unless it already has, meanwhile).
      this.db.exec("UPDATE device SET fcm_token = '' WHERE fcm_token = ?", doc.fcmToken);
    }
    return seq;
  }

  /** FCM goes out for every event, sockets or not: a socket may be dead
   * without anyone knowing yet, and the app drops duplicates by `seq`.
   * True when Google reports the token gone. */
  private async push(token: string, event: Event): Promise<boolean> {
    try {
      const outcome = await send(this.env, token, event);
      if (outcome === "token_invalid") console.log("device: FCM token is gone, cleared");
      if (outcome === "failed") console.log("device: FCM send failed");
      return outcome === "token_invalid";
    } catch (e) {
      console.log(`device: FCM error: ${e}`);
      return false;
    }
  }

  /** `GET /v1/events`, its signature checked by the Worker: a WebSocket,
   * or a catch-up page. */
  override async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);
    if (req.method !== "GET" || url.pathname !== "/v1/events") {
      return error(404, "not_found", "No such route.");
    }
    const doc = this.doc();
    if (!doc) return error(404, "unknown_device", "This install is not registered.");
    const after = Number.parseInt(url.searchParams.get("after") ?? "", 10);
    const events = this.eventsAfter(Number.isSafeInteger(after) && after > 0 ? after : 0);
    if (req.headers.get("upgrade")?.toLowerCase() !== "websocket") {
      return Response.json({ latest: doc.seq, events });
    }

    const [client, server] = Object.values(new WebSocketPair());
    this.ctx.acceptWebSocket(server);
    server.send(JSON.stringify({ type: "hello", latest: doc.seq } satisfies ServerFrame));
    for (const event of events) {
      server.send(JSON.stringify({ type: "event", event } satisfies ServerFrame));
    }
    return new Response(null, { status: 101, webSocket: client });
  }

  /** Drops events older than a day. */
  override async alarm(): Promise<void> {
    this.db.exec("DELETE FROM events WHERE at < ?", Math.max(0, now() - EVENT_TTL_SECS));
    const row = sql.first(this.db, "SELECT COUNT(*) AS n FROM events");
    const remaining = (row && sql.int(row, "n")) ?? 0;
    if (remaining > 0) {
      await this.ctx.storage.setAlarm(Date.now() + EVENT_TTL_SECS * 1000);
    }
  }

  // The app only listens; pings are answered by the auto-response.
  override async webSocketMessage(): Promise<void> {}

  override async webSocketClose(ws: WebSocket, code: number): Promise<void> {
    try {
      ws.close(code === 1005 || code === 1006 ? 1000 : code, "bye");
    } catch {
      // Already closed by the runtime.
    }
  }

  override async webSocketError(): Promise<void> {}
}
