// The Cloudflare edge Worker:
//
// - `email`: the aster0.dev catch-all. Mail to `vtop-otp-<id>@…` is parsed
//   in memory, the code sealed to the phone's key, and the box handed to
//   that install's event log. The edge never keeps anything readable.
// - `fetch`: app routes for mailboxes (`/v1/mail-otp/*`), installs
//   (`/v1/devices/*`) and the event channel (`GET /v1/events`: server-sent
//   events, a WebSocket or a catch-up read). `/mcp` goes on to the engine (`ENGINE_URL`), which
//   serves the MCP tools, so clients only ever talk to this host.
// - Accounts, access keys and VTOP sessions for the extension and agents
//   (`/v1/link`, `/v1/keys*`, `/v1/account*`, `/v1/whoami`, `/v1/session*`),
//   see account.ts. The engine does the VTOP work for them.
// - VTOP for the phone: `/vtop/*` proxies its own requests (vtop-proxy.ts);
//   `/v1/vtop/*` has the engine sign in and fetch for it (vtop-phone.ts).
// - `/internal/accounts/*`, `/internal/mail-otp/await`: the engine reports
//   sign-ins and waits for forwarded OTPs (sealed to its own key).
//
// State lives in Durable Objects: `Device` (per install), `Mailbox` (per
// address), `Index` (owner hash → mailbox, key hash → account), `Account`
// (per student) and `FcmToken` (one, caches the OAuth token), each in its
// own SQLite tables. Whose VTOP
// session a request carries is asked of the engine, which holds the key the
// owner hash is made with.

import * as account from "./account";
import { eventStream } from "./event-stream";
import { isVtopPath, proxyVtop } from "./vtop-proxy";
import { PHONE_ROUTES, phoneVtop } from "./vtop-phone";
import {
  type Env,
  badRequest,
  createDevice,
  deviceMatches,
  deviceStub,
  empty,
  enginePost,
  engineSend,
  error,
  healthBody,
  isDeviceId,
  isEngine,
  jsonBody,
  now,
  ownerStub,
  setDeviceFcm,
  str,
  variable,
  unknownDevice,
} from "./common";
import { type Payload, payloadFor } from "./mail";
import { MAIL_OWNER, type OtpAwait } from "./shared/engine-api";
import { isPublicKey, seal } from "./shared/mail-seal";
import { MAILBOX_SECRET_PREFIX, hash, randomToken } from "./shared/tokens";
import {
  type MailboxDoc,
  type MailboxMember,
  address,
  addMember,
  fcmToken,
  freshSealed,
  idFromAddress,
  isId,
  memberBySecret,
  memberLabel,
  newId,
  publicKey,
  setPublicKey,
  touch,
  wants,
} from "./state";

export { Account } from "./account";
export { Device } from "./device";
export { FcmToken } from "./fcm";
export { Index, Mailbox } from "./store";

function unknownMailbox(): Response {
  return error(404, "mailbox_not_found", "No such forwarding address.");
}

function mailboxStub(env: Env, id: string) {
  return env.MAILBOX.getByName(id);
}

async function loadMailbox(env: Env, id: string): Promise<MailboxDoc | null> {
  return isId(id) ? mailboxStub(env, id).doc() : null;
}

// ---- installs ---------------------------------------------------------------

/** The install a mailbox call names: a registered device, or (older app
 * builds) a new one made for the FCM token sent. Its id, or an error
 * message. */
async function resolveDevice(
  env: Env,
  body: Record<string, unknown>,
  fcm: string | null,
): Promise<{ id: string } | { message: string } | { unknown: true }> {
  const id = str(body.deviceId);
  const secret = str(body.deviceSecret);
  if (id !== undefined && secret !== undefined) {
    if (!(await deviceMatches(env, id, secret))) return { unknown: true };
    if (fcm !== null) await setDeviceFcm(env, id, fcm);
    return { id };
  }
  if (fcm === null) return { message: "deviceId and deviceSecret, or fcmToken, are required." };
  return { id: (await createDevice(env, fcm)).id };
}

async function deviceRegister(req: Request, env: Env): Promise<Response> {
  const body = await jsonBody(req);
  if (!body) return badRequest("Invalid JSON body.");
  const token = fcmToken(str(body.fcmToken));
  if (!token.ok) return badRequest(token.message);
  if (token.value === null) return badRequest("fcmToken is required.");
  const { id, secret } = await createDevice(env, token.value);
  return Response.json({ id, secret }, { status: 201 });
}

async function deviceUpdate(req: Request, env: Env): Promise<Response> {
  const body = await jsonBody(req);
  if (!body) return badRequest("Invalid JSON body.");
  const id = str(body.id) ?? "";
  if (!(await deviceMatches(env, id, str(body.secret) ?? ""))) {
    return unknownDevice();
  }
  const token = fcmToken(str(body.fcmToken));
  if (!token.ok) return badRequest(token.message);
  if (token.value === null) return badRequest("Nothing to update.");
  await setDeviceFcm(env, id, token.value);
  return empty();
}

/** Hands the request to the install's object, which checks the secret and
 * answers with a socket or a catch-up page. An `Accept: text/event-stream`
 * request gets server-sent events, read here from a socket to the object. */
async function events(req: Request, env: Env): Promise<Response> {
  const id = new URL(req.url).searchParams.get("device") ?? "";
  if (!isDeviceId(id)) return unknownDevice();
  const device = deviceStub(env, id);
  if (!req.headers.get("accept")?.includes("text/event-stream")) return device.fetch(req);

  const headers = new Headers(req.headers);
  headers.set("upgrade", "websocket");
  const res = await device.fetch(new Request(req.url, { headers }));
  // Not a socket: the object's refusal, passed on.
  if (!res.webSocket) return res;
  return new Response(eventStream(res.webSocket), {
    headers: {
      "content-type": "text/event-stream",
      // `no-transform` keeps Cloudflare from compressing, which buffers.
      "cache-control": "no-store, no-transform",
    },
  });
}

// ---- mailboxes --------------------------------------------------------------

/** A mailbox call: the address's id and the caller's mailbox secret, plus
 * what the route needs. */
type MailBody = Record<string, unknown>;

/** The mailbox the body's id and secret open, and which of its installs
 * the secret is. Records the visit, so quiet installs can be told apart. */
async function owned(env: Env, body: MailBody): Promise<{ doc: MailboxDoc; index: number } | null> {
  const secret = str(body.secret) ?? "";
  if (!secret) return null;
  const doc = await loadMailbox(env, str(body.id) ?? "");
  if (!doc) return null;
  const index = memberBySecret(doc, hash(secret));
  if (index === null) return null;
  if (touch(doc, index, now())) await mailboxStub(env, doc.id).save(doc);
  return { doc, index };
}

function newMember(deviceId: string, secret: string, key: string, label: string | undefined, at: number): MailboxMember {
  return {
    deviceId,
    secretHash: hash(secret),
    publicKey: key,
    label: memberLabel(label),
    sealed: null,
    sealedAt: null,
    addedAt: at,
    seenAt: at,
  };
}

/** `POST /internal/mail-otp/await`, from the engine: the next OTP of a
 * student, sealed to the key of the engine's sign-in. */
async function mailAwait(req: Request, env: Env): Promise<Response> {
  if (!isEngine(req, env)) return error(401, "unauthorized", "The engine secret is required.");
  const body = (await jsonBody(req)) as Partial<OtpAwait> | null;
  if (
    !body ||
    typeof body.ownerHash !== "string" ||
    typeof body.publicKey !== "string" ||
    typeof body.since !== "number" ||
    typeof body.waitSecs !== "number"
  ) {
    return badRequest("Invalid JSON body.");
  }
  if (!isPublicKey(body.publicKey)) return badRequest("publicKey must be a base64url X25519 key.");
  const id = await ownerStub(env, body.ownerHash).get();
  if (id === null) return unknownMailbox();
  const awaited = await mailboxStub(env, id).awaitCode({
    publicKey: body.publicKey,
    ownerHash: body.ownerHash,
    since: body.since,
    waitSecs: body.waitSecs,
  });
  return awaited ? Response.json(awaited) : unknownMailbox();
}

/** CORS as the bridge sends it: every origin, or the listed ones. */
function withCors(response: Response, origin: string | null, env: Env): Response {
  const out = new Response(response.body, response);
  const allowed = (env.ALLOWED_ORIGINS ?? "")
    .split(",")
    .map((o) => o.trim())
    .filter(Boolean);
  if (allowed.length === 0) {
    out.headers.set("access-control-allow-origin", "*");
  } else if (origin !== null && allowed.includes(origin)) {
    out.headers.set("access-control-allow-origin", origin);
    out.headers.set("vary", "Origin");
  }
  out.headers.set("access-control-allow-methods", "GET, POST, DELETE, OPTIONS");
  out.headers.set("access-control-allow-headers", "Content-Type");
  return out;
}

/** Whose VTOP session this is, as the engine's owner hash; or the error
 * reply. */
async function sessionOwner(env: Env, body: MailBody): Promise<{ owner: string } | { reply: Response }> {
  const session = body.session as Record<string, unknown> | undefined;
  const cookies = str(session?.cookies)?.trim();
  if (!cookies) return { reply: badRequest("session.cookies is required.") };
  const response = await enginePost(env, MAIL_OWNER, { cookies });
  switch (response.status) {
    case 200:
      return { owner: (await response.json<{ ownerHash: string }>()).ownerHash };
    case 401:
      return { reply: error(401, "session_invalid", "VTOP does not accept this session; sign in again.") };
    default:
      console.log(`mail-otp: owner check failed: ${response.status}`);
      return { reply: error(503, "vtop_unreachable", "Could not reach VTOP to check the session. Try again.") };
  }
}

function mailboxReply(env: Env, id: string, secret: string, status: number): Response {
  return Response.json({ id, address: address(id, variable(env, "MAIL_DOMAIN")), secret }, { status });
}

async function mailRegister(body: MailBody, env: Env): Promise<Response> {
  const key = publicKey(str(body.publicKey));
  if (!key.ok) return badRequest(key.message);
  if (key.value === null) return badRequest("publicKey is required.");
  const fcm = fcmToken(str(body.fcmToken));
  if (!fcm.ok) return badRequest(fcm.message);
  const device = await resolveDevice(env, body, fcm.value);
  if ("unknown" in device) return unknownDevice();
  if ("message" in device) return badRequest(device.message);
  const secret = randomToken(MAILBOX_SECRET_PREFIX);
  const at = now();
  for (let attempt = 0; attempt < 3; attempt++) {
    const id = newId();
    const doc: MailboxDoc = {
      id,
      ownerHash: null,
      members: [newMember(device.id, secret, key.value, str(body.label), at)],
      createdAt: at,
      updatedAt: at,
    };
    if (await mailboxStub(env, id).create(doc)) {
      console.log("mail-otp: registered a mailbox");
      return mailboxReply(env, id, secret, 201);
    }
  }
  return error(503, "store_unavailable", "Could not create an address. Try again.");
}

async function mailUpdate(body: MailBody, env: Env): Promise<Response> {
  const mine = await owned(env, body);
  if (!mine) return unknownMailbox();
  const { doc, index } = mine;
  const key = publicKey(str(body.publicKey));
  if (!key.ok) return badRequest(key.message);
  const fcm = fcmToken(str(body.fcmToken));
  if (!fcm.ok) return badRequest(fcm.message);
  const label = memberLabel(str(body.label));
  const moves = str(body.deviceId) !== undefined;
  if (key.value === null && fcm.value === null && label === null && !moves) {
    return badRequest("Nothing to update.");
  }
  const member = doc.members[index];
  if (key.value !== null) setPublicKey(member, key.value);
  if (label !== null) member.label = label;
  if (moves) {
    const device = await resolveDevice(env, body, fcm.value);
    if ("unknown" in device) return unknownDevice();
  if ("message" in device) return badRequest(device.message);
    member.deviceId = device.id;
  } else if (fcm.value !== null) {
    await setDeviceFcm(env, member.deviceId, fcm.value);
  }
  doc.updatedAt = now();
  await mailboxStub(env, doc.id).save(doc);
  return empty();
}

async function mailDelete(body: MailBody, env: Env): Promise<Response> {
  const mine = await owned(env, body);
  if (!mine) return unknownMailbox();
  const { doc, index } = mine;
  if (body.everywhere !== true && doc.members.length > 1) {
    doc.members.splice(index, 1);
    doc.updatedAt = now();
    await mailboxStub(env, doc.id).save(doc);
    console.log("mail-otp: a device left a mailbox");
    return empty();
  }
  if (doc.ownerHash !== null) await ownerStub(env, doc.ownerHash).remove(doc.id);
  await mailboxStub(env, doc.id).remove();
  console.log("mail-otp: deleted a mailbox");
  return empty();
}

/** The installs an address forwards to, for the app's device list. */
async function mailDevices(body: MailBody, env: Env): Promise<Response> {
  const mine = await owned(env, body);
  if (!mine) return unknownMailbox();
  const devices = mine.doc.members.map((m, i) => ({
    deviceId: m.deviceId,
    label: m.label,
    addedAt: m.addedAt,
    seenAt: m.seenAt,
    thisDevice: i === mine.index,
  }));
  return Response.json({ devices });
}

/** Takes another install off the address; it stops getting OTPs. */
async function mailRemoveDevice(body: MailBody, env: Env): Promise<Response> {
  const mine = await owned(env, body);
  if (!mine) return unknownMailbox();
  const { doc, index } = mine;
  const target = str(body.removeDevice);
  if (target === undefined) return badRequest("removeDevice is required.");
  const position = doc.members.findIndex((m) => m.deviceId === target);
  if (position < 0) return error(404, "unknown_device", "That device does not use this address.");
  if (position === index) return badRequest("Use delete to take this device off the address.");
  doc.members.splice(position, 1);
  doc.updatedAt = now();
  await mailboxStub(env, doc.id).save(doc);
  console.log("mail-otp: a device was removed from a mailbox");
  return empty();
}

async function mailInbox(body: MailBody, env: Env): Promise<Response> {
  const mine = await owned(env, body);
  if (!mine) return unknownMailbox();
  const box = freshSealed(mine.doc.members[mine.index], now());
  return Response.json(box ? { sealed: box.sealed, receivedAt: box.at } : { sealed: null });
}

/** Ties the mailbox to the student whose session comes with it, so a new
 * phone of theirs can take it back with `/reclaim`. The latest wins. */
async function mailOwner(body: MailBody, env: Env): Promise<Response> {
  const mine = await owned(env, body);
  if (!mine) return unknownMailbox();
  const { doc } = mine;
  const session = await sessionOwner(env, body);
  if ("reply" in session) return session.reply;
  const { owner } = session;
  // The student already has another live address: that is the one their
  // Gmail filter forwards to, so it keeps the account. The app joins it.
  const existing = await ownerStub(env, owner).get();
  if (existing !== null && existing !== doc.id && (await loadMailbox(env, existing))) {
    return error(409, "address_exists", "This VTOP account already has a forwarding address; use that one.");
  }
  if (doc.ownerHash !== null && doc.ownerHash !== owner) await ownerStub(env, doc.ownerHash).remove(doc.id);
  doc.ownerHash = owner;
  doc.updatedAt = now();
  await mailboxStub(env, doc.id).save(doc);
  await ownerStub(env, owner).put(doc.id);
  return empty();
}

/** Adds this install to the caller's own address, with its own key and
 * secret. The other installs keep getting OTPs. */
async function mailReclaim(body: MailBody, env: Env): Promise<Response> {
  const key = publicKey(str(body.publicKey));
  if (!key.ok) return badRequest(key.message);
  if (key.value === null) return badRequest("publicKey is required.");
  const fcm = fcmToken(str(body.fcmToken));
  if (!fcm.ok) return badRequest(fcm.message);
  const session = await sessionOwner(env, body);
  if ("reply" in session) return session.reply;
  const id = await ownerStub(env, session.owner).get();
  if (id === null) return unknownMailbox();
  const doc = await loadMailbox(env, id);
  if (!doc || doc.ownerHash !== session.owner) return unknownMailbox();
  const device = await resolveDevice(env, body, fcm.value);
  if ("unknown" in device) return unknownDevice();
  if ("message" in device) return badRequest(device.message);
  const secret = randomToken(MAILBOX_SECRET_PREFIX);
  const at = now();
  addMember(doc, newMember(device.id, secret, key.value, str(body.label), at), at);
  doc.updatedAt = at;
  await mailboxStub(env, doc.id).save(doc);
  console.log("mail-otp: a device joined a mailbox");
  return mailboxReply(env, doc.id, secret, 200);
}

const MAIL_ROUTES: Record<string, (body: MailBody, env: Env) => Promise<Response>> = {
  "/v1/mail-otp/register": mailRegister,
  "/v1/mail-otp/update": mailUpdate,
  "/v1/mail-otp/delete": mailDelete,
  "/v1/mail-otp/inbox": mailInbox,
  "/v1/mail-otp/owner": mailOwner,
  "/v1/mail-otp/reclaim": mailReclaim,
  "/v1/mail-otp/devices": mailDevices,
  "/v1/mail-otp/devices/remove": mailRemoveDevice,
};

// ---- entry points -----------------------------------------------------------

async function route(req: Request, env: Env): Promise<Response> {
  const path = new URL(req.url).pathname;
  const { method } = req;
  if (method === "GET" && path === "/healthz") {
    return Response.json(healthBody(req.cf as { colo?: unknown } | undefined));
  }
  if (isVtopPath(path)) return proxyVtop(req);
  if (path in PHONE_ROUTES) return phoneVtop(req, env);
  if (method === "GET" && path === "/v1/events") return events(req, env);
  if (method === "POST" && path === "/v1/devices/register") return deviceRegister(req, env);
  if (method === "POST" && path === "/v1/devices/update") return deviceUpdate(req, env);
  if (method === "POST" && path === "/v1/app/status") return account.appCall(req, env, "status");
  if (method === "POST" && path === "/v1/app/session/clear") return account.appCall(req, env, "session-clear");
  if (method === "POST" && path === "/v1/app/login") return account.appCall(req, env, "login");
  const mailRoute = MAIL_ROUTES[path];
  if (method === "POST" && mailRoute) {
    const body = await jsonBody(req);
    return body ? mailRoute(body, env) : badRequest("Invalid JSON body.");
  }
  if (method === "POST" && path === "/internal/mail-otp/await") return mailAwait(req, env);
  if (path.startsWith("/internal/accounts/")) return account.internal(req, env, path);
  if (account.handles(path)) {
    const origin = req.headers.get("origin");
    const response = method === "OPTIONS" ? empty() : await account.handle(req, env, path);
    return withCors(response, origin, env);
  }
  if (path.startsWith("/internal/")) return error(404, "not_found", "No such route.");
  if (path === "/mcp" || path.startsWith("/mcp/")) return passOn(req, env);
  return error(404, "not_found", "No such route.");
}

/** Hands a request to the engine as it came, with the caller's address for
 * its rate limits. */
async function passOn(req: Request, env: Env): Promise<Response> {
  const url = new URL(req.url);
  const headers = new Headers();
  for (const [name, value] of req.headers) {
    if (!["host", "x-forwarded-for", "x-real-ip"].includes(name) && !name.startsWith("cf-")) {
      headers.append(name, value);
    }
  }
  const ip = req.headers.get("cf-connecting-ip");
  if (ip !== null) headers.set("x-forwarded-for", ip);
  const init: RequestInit = { method: req.method, headers };
  if (!["GET", "HEAD", "OPTIONS"].includes(req.method)) {
    const body = await req.arrayBuffer();
    if (body.byteLength > 0) init.body = body;
  }
  return engineSend(env, `${url.pathname}${url.search}`, init);
}

/** The catch-all. Throwing makes the sending server retry later. */
async function email(message: ForwardableEmailMessage, env: Env): Promise<void> {
  const id = idFromAddress(message.to);
  if (id === null) {
    const to = env.FALLBACK_FORWARD ?? "";
    if (to) await message.forward(to);
    else message.setReject("Unknown address");
    return;
  }
  const doc = await loadMailbox(env, id);
  if (!doc) {
    message.setReject("Unknown address");
    return;
  }
  const payload = await payloadFor(message.raw, now());
  if (!payload) {
    console.log("mail-otp: dropped a mail for a mailbox");
    return;
  }
  const plaintext = new TextEncoder().encode(JSON.stringify(payload));
  // Sealed once per install, each to its own key.
  const boxes = doc.members.map((member) => {
    const sealed = seal(member.publicKey, id, plaintext);
    if (sealed === null) throw new Error("seal: a member's key is not an X25519 key");
    return { deviceId: member.deviceId, sealed };
  });
  if (!(await mailboxStub(env, id).deliver(boxes))) throw new Error("deliver failed: no mailbox");
  // The code is stored now. What follows is best effort: an error here must
  // not make the mail server send it again (a second push for the same
  // code).
  if (payload.kind === "otp") {
    try {
      await handToWaiters(env, id, payload, plaintext);
    } catch (e) {
      console.log(`mail-otp: handing to a server sign-in failed: ${e}`);
    }
  }
  for (const { deviceId, sealed } of boxes) {
    try {
      const seq = await deviceStub(env, deviceId).append({ kind: "mail_otp", mailbox: id, sealed });
      // The box is kept; the app still pulls it from /inbox.
      if (seq === null) console.log("mail-otp: event append failed: no install");
    } catch (e) {
      console.log(`mail-otp: event append failed: ${e}`);
    }
  }
  console.log(`mail-otp: relayed a ${payload.kind} to ${doc.members.length} device(s)`);
}

/** Seals an OTP to each engine sign-in waiting on this mailbox. */
async function handToWaiters(env: Env, id: string, payload: Payload, plaintext: Uint8Array): Promise<void> {
  const stub = mailboxStub(env, id);
  const at = now();
  for (const waiter of await stub.waiters()) {
    if (!wants(waiter, payload.sentAt, at)) continue;
    const sealed = seal(waiter.publicKey, waiter.ownerHash, plaintext);
    if (sealed === null) continue;
    await stub.waiterResult(waiter.publicKey, sealed);
    console.log("mail-otp: handed an OTP to a server sign-in");
  }
}

export default {
  fetch: route,
  email,
} satisfies ExportedHandler<Env>;
