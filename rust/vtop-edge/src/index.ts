// The Cloudflare edge Worker:
//
// - `email`: the MAIL_DOMAIN catch-all. Mail to `otp+<id>@…` is parsed
//   in memory, the code sealed to each phone's `sealKey`, and the box handed
//   to that install's event log. The edge never keeps anything readable.
// - `fetch`: installs (`/v1/devices/*`), mailboxes (`/v1/mail-otp/*`) and
//   the event channel (`GET /v1/events`: server-sent events, a WebSocket or
//   a catch-up read). Phones sign every call but `register` with their
//   device key (auth.ts). `/mcp` goes on to the engine (`ENGINE_URL`), which
//   serves the MCP tools, so clients only ever talk to this host. MCP
//   clients get their access key by OAuth (oauth.ts).
// - Accounts, access keys and VTOP sessions for the extension and agents
//   (`/v1/link`, `/v1/keys*`, `/v1/account*`, `/v1/whoami`, `/v1/session*`),
//   see account.ts. The engine does the VTOP work for them.
// - VTOP for the phone: `/vtop/*` proxies its own requests (vtop-proxy.ts);
//   `/v1/vtop/*` has the engine sign in and fetch for it (vtop-phone.ts).
// - `/internal/*` (`INTERNAL_TOKEN`): the engine reports sign-ins and waits
//   for forwarded OTPs; the UDP gateway reports its keys.
//
// State lives in Durable Objects: `Device` (per install), `Mailbox` (per
// address), `Index` (owner hash → mailbox, key hash → account, the UDP
// gateway), `Account` (per student), `FcmToken` (one, caches the OAuth
// token) and `OAuth` (one, MCP clients and their sign-ins), each in its own SQLite tables. Whose VTOP session a request
// carries is asked of the engine, which holds the key the owner hash is
// made with.

import * as account from "./account";
import * as oauth from "./oauth";
import { type Signed, base64Url, checkSignature, deviceIdFor, signed } from "./auth";
import { eventStream } from "./event-stream";
import { isVtopPath, proxyVtop } from "./vtop-proxy";
import { udpGateway, udpKeys } from "./udp-gateway";
import { PHONE_ROUTES, phoneVtop } from "./vtop-phone";
import {
  type Env,
  badRequest,
  deviceStub,
  empty,
  engineSend,
  enginePost,
  error,
  healthBody,
  isInternal,
  jsonBody,
  notInternal,
  now,
  ownerStub,
  parseJson,
  str,
  variable,
} from "./common";
import type { DeviceChanges } from "./device";
import { type Payload, payloadFor } from "./mail";
import { MAIL_OWNER, type OtpAwait } from "./shared/engine-api";
import { isPublicKey, seal } from "./shared/mail-seal";
import {
  type MailboxDoc,
  type MailboxMember,
  address,
  addMember,
  fcmToken,
  freshSealed,
  idFromAddress,
  isId,
  memberLabel,
  memberOf,
  newId,
  platform,
  sealKey,
  touch,
  wants,
} from "./state";

export { Account } from "./account";
export { Device } from "./device";
export { FcmToken } from "./fcm";
export { Index, Mailbox } from "./store";
export { OAuth } from "./oauth";

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

/** `POST /v1/devices/register`, signed with the key it registers. The id
 * comes from the key, so registering again returns the same one. */
async function deviceRegister(req: Request, env: Env): Promise<Response> {
  const raw = new Uint8Array(await req.arrayBuffer());
  const body = parseJson(raw, req.headers);
  if (!body) return badRequest("Invalid JSON body.");
  const signKey = base64Url(str(body.signKey) ?? "");
  if (signKey?.length !== 32) return badRequest("signKey must be a base64url Ed25519 key.");
  const seal = sealKey(str(body.sealKey));
  if (!seal.ok) return badRequest(seal.message);
  if (seal.value === null) return badRequest("sealKey is required.");
  const kind = platform(str(body.platform));
  if (!kind.ok) return badRequest(kind.message);
  const fcm = fcmToken(str(body.fcmToken));
  if (!fcm.ok) return badRequest(fcm.message);
  const refused = await checkSignature(req, raw, signKey);
  if (refused) return refused;
  const id = deviceIdFor(signKey);
  const created = await deviceStub(env, id).register({
    id,
    signKey: str(body.signKey)!.trim().replace(/=+$/, ""),
    sealKey: seal.value,
    label: memberLabel(str(body.label)),
    platform: kind.value!,
    fcmToken: fcm.value,
  });
  if (created) console.log("device: registered an install");
  return Response.json({ deviceId: id }, { status: created ? 201 : 200 });
}

async function deviceUpdate(auth: Signed, env: Env): Promise<Response> {
  const body = auth.json;
  if (!body) return badRequest("Invalid JSON body.");
  const changes: DeviceChanges = {};
  const seal = sealKey(str(body.sealKey));
  if (!seal.ok) return badRequest(seal.message);
  if (seal.value !== null) changes.sealKey = seal.value;
  const fcm = fcmToken(str(body.fcmToken));
  if (!fcm.ok) return badRequest(fcm.message);
  if (fcm.value !== null) changes.fcmToken = fcm.value;
  const label = memberLabel(str(body.label));
  if (label !== null) changes.label = label;
  if (Object.keys(changes).length === 0) return badRequest("Nothing to update.");
  await deviceStub(env, auth.device.id).update(changes);
  return empty();
}

/** `POST /v1/devices/info`: the install as the edge keeps it. */
async function deviceInfo(auth: Signed, env: Env): Promise<Response> {
  const summary = await deviceStub(env, auth.device.id).summary();
  return summary ? Response.json({ deviceId: auth.device.id, ...summary }) : error(404, "unknown_device", "This install is not registered.");
}

/** `POST /v1/devices/delete`: the install deletes itself. With `mailbox`
 * it leaves that address first (deleting it when last), with
 * `registrationNumber` it is taken off that account's phones. Registering
 * again later starts afresh. */
async function deviceDelete(auth: Signed, env: Env): Promise<Response> {
  const body = auth.json ?? {};
  const mailbox = str(body.mailbox);
  if (mailbox) await mailDelete(auth.device.id, { id: mailbox }, env);
  const registrationNumber = str(body.registrationNumber)?.trim();
  if (registrationNumber) await account.forgetPhone(env, registrationNumber, auth.device.id);
  await deviceStub(env, auth.device.id).remove();
  console.log("device: an install deleted itself");
  return empty();
}

/** Hands the request to the install's object, which answers with a socket
 * or a catch-up page. An `Accept: text/event-stream` request gets
 * server-sent events, read here from a socket to the object. */
async function events(auth: Signed, req: Request, env: Env): Promise<Response> {
  const device = deviceStub(env, auth.device.id);
  const headers = new Headers(req.headers);
  if (!req.headers.get("accept")?.includes("text/event-stream")) {
    return device.fetch(new Request(req.url, { headers }));
  }
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

type MailBody = Record<string, unknown>;

/** The mailbox named by the body's `id`, if the calling install is one of
 * its members. Records the visit, so quiet installs can be told apart. */
async function owned(env: Env, deviceId: string, body: MailBody): Promise<{ doc: MailboxDoc; index: number } | null> {
  const doc = await loadMailbox(env, str(body.id) ?? "");
  if (!doc) return null;
  const index = memberOf(doc, deviceId);
  if (index === null) return null;
  if (touch(doc, index, now())) await mailboxStub(env, doc.id).save(doc);
  return { doc, index };
}

function newMember(deviceId: string, at: number): MailboxMember {
  return { deviceId, sealed: null, sealedAt: null, addedAt: at, seenAt: at };
}

/** `POST /internal/mail-otp/await`, from the engine: the next OTP of a
 * student, sealed to the key of the engine's sign-in. */
async function mailAwait(req: Request, env: Env): Promise<Response> {
  if (!isInternal(req, env)) return notInternal();
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
export function withCors(response: Response, origin: string | null, env: Env): Response {
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
  out.headers.set("access-control-allow-headers", "Content-Type, Authorization");
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

function mailboxReply(env: Env, id: string, status: number): Response {
  return Response.json({ id, address: address(id, variable(env, "MAIL_DOMAIN")) }, { status });
}

async function mailRegister(deviceId: string, _body: MailBody, env: Env): Promise<Response> {
  const at = now();
  for (let attempt = 0; attempt < 3; attempt++) {
    const id = newId();
    const doc: MailboxDoc = { id, ownerHash: null, members: [newMember(deviceId, at)], createdAt: at, updatedAt: at };
    if (await mailboxStub(env, id).create(doc)) {
      console.log("mail-otp: registered a mailbox");
      return mailboxReply(env, id, 201);
    }
  }
  return error(503, "store_unavailable", "Could not create an address. Try again.");
}

async function mailDelete(deviceId: string, body: MailBody, env: Env): Promise<Response> {
  const mine = await owned(env, deviceId, body);
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
async function mailDevices(deviceId: string, body: MailBody, env: Env): Promise<Response> {
  const mine = await owned(env, deviceId, body);
  if (!mine) return unknownMailbox();
  const devices = await Promise.all(
    mine.doc.members.map(async (m, i) => ({
      deviceId: m.deviceId,
      label: (await deviceStub(env, m.deviceId).info())?.label ?? null,
      addedAt: m.addedAt,
      seenAt: m.seenAt,
      thisDevice: i === mine.index,
    })),
  );
  return Response.json({ devices });
}

/** Takes another install off the address; it stops getting OTPs. */
async function mailRemoveDevice(deviceId: string, body: MailBody, env: Env): Promise<Response> {
  const mine = await owned(env, deviceId, body);
  if (!mine) return unknownMailbox();
  const { doc, index } = mine;
  const target = str(body.removeDevice);
  if (target === undefined) return badRequest("removeDevice is required.");
  const position = memberOf(doc, target);
  if (position === null) return error(404, "unknown_device", "That device does not use this address.");
  if (position === index) return badRequest("Use delete to take this device off the address.");
  doc.members.splice(position, 1);
  doc.updatedAt = now();
  await mailboxStub(env, doc.id).save(doc);
  console.log("mail-otp: a device was removed from a mailbox");
  return empty();
}

async function mailInbox(deviceId: string, body: MailBody, env: Env): Promise<Response> {
  const mine = await owned(env, deviceId, body);
  if (!mine) return unknownMailbox();
  const box = freshSealed(mine.doc.members[mine.index], now());
  return Response.json(box ? { sealed: box.sealed, receivedAt: box.at } : { sealed: null });
}

/** Ties the mailbox to the student whose session comes with it, so a new
 * phone of theirs can join it with `/reclaim`. The latest wins. */
async function mailOwner(deviceId: string, body: MailBody, env: Env): Promise<Response> {
  const mine = await owned(env, deviceId, body);
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

/** Adds this install to the student's own address (their VTOP session
 * says whose). The other installs keep getting OTPs. */
async function mailReclaim(deviceId: string, body: MailBody, env: Env): Promise<Response> {
  const session = await sessionOwner(env, body);
  if ("reply" in session) return session.reply;
  const id = await ownerStub(env, session.owner).get();
  if (id === null) return unknownMailbox();
  const doc = await loadMailbox(env, id);
  if (!doc || doc.ownerHash !== session.owner) return unknownMailbox();
  const at = now();
  addMember(doc, newMember(deviceId, at), at);
  doc.updatedAt = at;
  await mailboxStub(env, doc.id).save(doc);
  console.log("mail-otp: a device joined a mailbox");
  return mailboxReply(env, doc.id, 200);
}

const MAIL_ROUTES: Record<string, (deviceId: string, body: MailBody, env: Env) => Promise<Response>> = {
  "/v1/mail-otp/register": mailRegister,
  "/v1/mail-otp/delete": mailDelete,
  "/v1/mail-otp/inbox": mailInbox,
  "/v1/mail-otp/owner": mailOwner,
  "/v1/mail-otp/reclaim": mailReclaim,
  "/v1/mail-otp/devices": mailDevices,
  "/v1/mail-otp/devices/remove": mailRemoveDevice,
};

/** Signed phone routes, by `METHOD path`. */
const SIGNED: Record<string, (auth: Signed, req: Request, env: Env) => Promise<Response>> = {
  "POST /v1/devices/update": (auth, _req, env) => deviceUpdate(auth, env),
  "POST /v1/devices/info": (auth, _req, env) => deviceInfo(auth, env),
  "POST /v1/devices/delete": (auth, _req, env) => deviceDelete(auth, env),
  "GET /v1/events": events,
  "GET /v1/udp": (_auth, _req, env) => udpGateway(env),
  "POST /v1/app/status": (auth, _req, env) => account.appCall(auth, env, "status"),
  "POST /v1/app/session/clear": (auth, _req, env) => account.appCall(auth, env, "session-clear"),
  "POST /v1/app/login": (auth, _req, env) => account.appCall(auth, env, "login"),
  ...Object.fromEntries(
    Object.entries(MAIL_ROUTES).map(([path, handler]) => [
      `POST ${path}`,
      (auth: Signed, _req: Request, env: Env) =>
        auth.json ? handler(auth.device.id, auth.json, env) : Promise.resolve(badRequest("Invalid JSON body.")),
    ]),
  ),
  ...Object.fromEntries(
    Object.keys(PHONE_ROUTES).map((path) => [
      `POST ${path}`,
      (auth: Signed, req: Request, env: Env) => phoneVtop(req, auth.body, env),
    ]),
  ),
};

// ---- entry points -----------------------------------------------------------

async function route(req: Request, env: Env): Promise<Response> {
  const path = new URL(req.url).pathname;
  const { method } = req;
  if (method === "GET" && path === "/healthz") {
    return Response.json(healthBody(req.cf as { colo?: unknown } | undefined));
  }
  if (isVtopPath(path)) return proxyVtop(req);
  if (method === "POST" && path === "/v1/devices/register") return deviceRegister(req, env);
  const phoneRoute = SIGNED[`${method} ${path}`];
  if (phoneRoute) {
    const auth = await signed(req, env);
    return auth instanceof Response ? auth : phoneRoute(auth, req, env);
  }
  if (path in PHONE_ROUTES) return error(405, "method_not_allowed", "Use POST.");
  if (method === "POST" && path === "/internal/mail-otp/await") return mailAwait(req, env);
  if (method === "POST" && path === "/internal/udp-keys") return udpKeys(req, env);
  if (path.startsWith("/internal/accounts/")) return account.internal(req, env, path);
  if (account.handles(path)) {
    const origin = req.headers.get("origin");
    const response = method === "OPTIONS" ? empty() : await account.handle(req, env, path);
    return withCors(response, origin, env);
  }
  if (path.startsWith("/internal/")) return error(404, "not_found", "No such route.");
  const oauthReply = await oauth.handle(req, env, path);
  if (oauthReply) return oauthReply;
  if (path === "/mcp" || path.startsWith("/mcp/")) return mcp(req, env);
  return error(404, "not_found", "No such route.");
}

/** `/mcp`: a client with no token is told where to get one (MCP
 * authorization); the rest go to the engine, whose 401s get the same hint. */
async function mcp(req: Request, env: Env): Promise<Response> {
  if (req.method !== "OPTIONS" && !req.headers.get("authorization")) return oauth.unauthorized(env);
  const response = await passOn(req, env);
  if (response.status !== 401 || response.headers.has("www-authenticate")) return response;
  const out = new Response(response.body, response);
  out.headers.set("www-authenticate", oauth.challenge(env));
  return out;
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
  // Sealed once per install, each to its own key; an install no longer
  // registered is skipped.
  const sealed = await Promise.all(
    doc.members.map(async ({ deviceId }) => {
      const key = (await deviceStub(env, deviceId).info())?.sealKey;
      const box = key === undefined ? null : seal(key, id, plaintext);
      return box === null ? [] : [{ deviceId, sealed: box }];
    }),
  );
  const boxes = sealed.flat();
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
  console.log(`mail-otp: relayed a ${payload.kind} to ${boxes.length} device(s)`);
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
