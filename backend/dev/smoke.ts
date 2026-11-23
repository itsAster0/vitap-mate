// Smoke test against a deployment (default: the vmate3 test box): the demo token, a refresh and
// its reuse, a full proof (ticket → engine's internal calls → token), mail in and out, an access
// key, and a few queries. Reads the test tokens from .env.test.local and prints none of them.
//
//   bun dev/smoke.ts [https://vmate3.aster0.dev]

import { ConvexHttpClient } from "convex/browser";
import { ConvexError } from "convex/values";
import { anyApi } from "convex/server";

declare const Bun: { file(path: string): { text(): Promise<string> } };

const base = process.env.SMOKE_URL ?? "https://vmate3.aster0.dev";
const env: Record<string, string> = {};
for (const line of (await Bun.file(new URL("../.env.test.local", import.meta.url).pathname).text()).split("\n")) {
  const at = line.indexOf("=");
  if (at > 0) env[line.slice(0, at)] = line.slice(at + 1).replace(/^'|'$/g, "");
}

const b64url = (b: Uint8Array) => btoa(String.fromCharCode(...b)).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
const hex = (b: ArrayBuffer) => Array.from(new Uint8Array(b), (x) => x.toString(16).padStart(2, "0")).join("");

async function phone() {
  const sign = (await crypto.subtle.generateKey({ name: "Ed25519" }, true, ["sign", "verify"])) as CryptoKeyPair;
  const seal = (await crypto.subtle.generateKey({ name: "X25519" }, true, ["deriveBits"])) as CryptoKeyPair;
  const key = b64url(new Uint8Array(await crypto.subtle.exportKey("raw", sign.publicKey)));
  const sealKey = b64url(new Uint8Array(await crypto.subtle.exportKey("raw", seal.publicKey)));
  const signed = async (fn: string, body: object) => {
    const payload = JSON.stringify(body);
    const time = String(Math.floor(Date.now() / 1000));
    const msg = `vm1\nCALL\n/${fn}\n${time}\n${hex(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(payload)))}`;
    const sig = b64url(new Uint8Array(await crypto.subtle.sign({ name: "Ed25519" }, sign.privateKey, new TextEncoder().encode(msg))));
    return { flow: "smoke", signed: { key, time, sig }, payload };
  };
  return { signed, sealKey, device: { sealKey, platform: "android", label: "Smoke" } };
}

let failures = 0;
function check(name: string, ok: boolean, detail = "") {
  console.log(`${ok ? "ok  " : "FAIL"} ${name}${detail ? ` (${detail})` : ""}`);
  if (!ok) failures++;
}
async function code(p: Promise<unknown>): Promise<string> {
  try {
    await p;
    return "none";
  } catch (e) {
    return e instanceof ConvexError ? String((e.data as { code?: string }).code) : `other: ${e instanceof Error ? e.message.slice(0, 120) : e}`;
  }
}
const internalPost = (path: string, token: string, body: object) =>
  fetch(`${base}${path}`, { method: "POST", headers: { authorization: `Bearer ${token}`, "content-type": "application/json" }, body: JSON.stringify(body) });

const api = anyApi;

// ---- public --------------------------------------------------------------------------------
const open = new ConvexHttpClient(base);
const updates = (await open.query(api.guides.updates, {})) as { schema: number };
check("guides:updates", updates.schema === 1);
const demoGuide = (await open.query(api.guides.demo, {})) as { pages: Record<string, unknown> };
check("guides:demo", Object.keys(demoGuide.pages).length > 10);
const guide = await fetch(`${base}/guides/forwarding/1-add.webp`);
check("GET /guides/* image", guide.status === 200 && guide.headers.get("content-type") === "image/webp");
check("GET /guides/* 304", (await fetch(`${base}/guides/forwarding/1-add.webp`, { headers: { "if-none-match": guide.headers.get("etag") ?? "" } })).status === 304);
check("udp:keys before a report", (await code(open.query(api.udp.keys, {}))) === "udp_unavailable");

// ---- demo ----------------------------------------------------------------------------------
const demoPhone = await phone();
const demo = (await open.action(api.auth.token, await demoPhone.signed("auth:token", { demo: true, device: demoPhone.device }))) as {
  jwt: string;
  refreshToken: string;
  regno: string;
};
check("auth:token demo", demo.regno === "DEMO2026" && demo.jwt.split(".").length === 3);
const demoClient = new ConvexHttpClient(base);
demoClient.setAuth(demo.jwt);
const info = (await demoClient.query(api.devices.info, { flow: "smoke" })) as { platform: string };
check("devices:info with the demo JWT", info.platform === "android");
const view = (await demoClient.query(api.account.view, { flow: "smoke" })) as { registrationNumber: string };
check("account:view demo", view.registrationNumber === "DEMO2026");
check("sessions:pending demo", ((await demoClient.query(api.sessions.pending, { flow: "smoke" })) as unknown[]).length === 0);
check("demo refused a write", (await code(demoClient.mutation(api.mail.register, { flow: "smoke" }))) === "demo_unavailable");
const refreshed = (await open.action(api.auth.token, await demoPhone.signed("auth:token", { refreshToken: demo.refreshToken }))) as { refreshToken: string };
check("refresh rotates", refreshed.refreshToken !== demo.refreshToken);
// Once the chain moved on (the new token used), the old one is a reuse, not a lost answer.
await open.action(api.auth.token, await demoPhone.signed("auth:token", { refreshToken: refreshed.refreshToken }));
check("reused refresh token", (await code(open.action(api.auth.token, await demoPhone.signed("auth:token", { refreshToken: demo.refreshToken })))) === "refresh_reused");
check("chain is gone after reuse", (await code(open.action(api.auth.token, await demoPhone.signed("auth:token", { refreshToken: refreshed.refreshToken })))) === "refresh_invalid");
check("no JWT", (await code(open.query(api.devices.info, { flow: "smoke" }))) === "unauthenticated");

// ---- a proof, as the engine would report it ----------------------------------------------
async function proveAs(p: Awaited<ReturnType<typeof phone>>, regno: string, first: boolean) {
  const { ticket } = (await open.mutation(api.auth.proveStart, await p.signed("auth:proveStart", {}))) as { ticket: string };
  if (first) {
    check("auth:proveStart", /^[0-9a-f]{32}$/.test(ticket));
    check("token before the proof", (await code(open.action(api.auth.token, await p.signed("auth:token", { ticket, device: p.device })))) === "proof_pending");
  }
  const taken = await internalPost("/internal/proof-ticket", env.INTERNAL_TOKEN, { ticket });
  if (first) {
    check("/internal/proof-ticket", taken.status === 200);
    check("/internal/proof-ticket twice", (await internalPost("/internal/proof-ticket", env.INTERNAL_TOKEN, { ticket })).status === 404);
  }
  const done = await internalPost("/internal/proof-done", env.INTERNAL_TOKEN, { ticket, regno, ownerHash: "owner-smoke" });
  if (first) check("/internal/proof-done", done.status === 204);
  const tokens = (await open.action(api.auth.token, await p.signed("auth:token", { ticket, device: p.device }))) as { jwt: string; regno: string };
  const client = new ConvexHttpClient(base);
  client.setAuth(tokens.jwt);
  return { tokens, client };
}
const p = await phone();
const { tokens: proved, client: me } = await proveAs(p, "smoke0001", true);
check("auth:token with a proved ticket", proved.regno === "SMOKE0001");
const acct = (await me.query(api.account.view, { flow: "smoke" })) as { thisPhone: boolean; phoneCount: number };
check("account:view linked", acct.thisPhone && acct.phoneCount === 1);

const unsaved = (await me.action(api.account.saveCredentials, { flow: "smoke", credentials: { username: "u", password: "p" } })) as { savedCredentials: boolean };
check("account:saveCredentials without a way to read the OTP", unsaved.savedCredentials === false);

// ---- mail -----------------------------------------------------------------------------------
const box = (await me.mutation(api.mail.register, { flow: "smoke" })) as { id: string; address: string };
check("mail:register", box.address.startsWith(`otp+${box.id}@`));
const mailIn = await internalPost("/internal/mail", env.MAIL_TOKEN, { addressId: box.id, receivedAt: Date.now(), otp: "123456", gmailConfirm: null, from: "x", subject: "y" });
check("/internal/mail", mailIn.status === 200);
check("/internal/mail wrong token", (await internalPost("/internal/mail", env.INTERNAL_TOKEN, { addressId: box.id })).status === 401);
check("/internal/mail unknown address", (await internalPost("/internal/mail", env.MAIL_TOKEN, { addressId: "aaaaaaaaaa", receivedAt: Date.now(), otp: "123456" })).status === 404);
const inbox = (await me.query(api.mail.inbox, { flow: "smoke", id: box.id })) as { sealed: string | null };
check("mail:inbox has a sealed box", typeof inbox.sealed === "string");

// ---- keys and the key routes ----------------------------------------------------------------
const made = (await me.action(api.keys.create, { flow: "smoke", label: "Smoke key" })) as { key: string; id: string };
check("keys:create", made.key.startsWith("vtm_"));
const whoami = await fetch(`${base}/v1/whoami`, { headers: { authorization: `Bearer ${made.key}` } });
check("GET /v1/whoami", whoami.status === 200 && ((await whoami.json()) as { registrationNumber: string }).registrationNumber === "SMOKE0001");
const asked = await fetch(`${base}/v1/session`, { method: "POST", headers: { authorization: `Bearer ${made.key}` } });
const askedBody = (await asked.json()) as { requestId?: string; stage?: string };
check("POST /v1/session asks the phone", asked.status === 202 && askedBody.stage === "phone", `${asked.status} ${askedBody.stage}`);
const waiting = (await me.query(api.sessions.pending, { flow: "smoke" })) as Array<{ requestId: string }>;
check("sessions:pending shows it", waiting.some((r) => r.requestId === askedBody.requestId));
await me.action(api.sessions.answer, { flow: "smoke", requestId: askedBody.requestId!, error: "smoke: declined" });
check("sessions:answer with an error ends it", ((await me.query(api.sessions.pending, { flow: "smoke" })) as unknown[]).length === 0);
const status = (await me.query(api.app.status, { flow: "smoke" })) as { phones: number; keys: Array<{ id: string }> };
check("app:status", status.phones === 1 && status.keys.some((k) => k.id === made.id));
await me.mutation(api.keys.revoke, { flow: "smoke", id: made.id });
check("revoked key", (await fetch(`${base}/v1/whoami`, { headers: { authorization: `Bearer ${made.key}` } })).status === 401);

// ---- a second phone joins, then removes itself; deleting the account ends the first ----------
const p2 = await phone();
const { client: other } = await proveAs(p2, "SMOKE0001", false);
check("second phone joins the account", ((await other.query(api.account.view, { flow: "smoke" })) as { phoneCount: number }).phoneCount === 2);
check("mail:register joins the same address", ((await other.mutation(api.mail.register, { flow: "smoke" })) as { id: string }).id === box.id);
await other.mutation(api.devices.remove, { flow: "smoke", mailbox: box.id });
check("removed device", (await code(other.query(api.account.view, { flow: "smoke" }))) === "device_removed");
check("one phone left", ((await me.query(api.account.view, { flow: "smoke" })) as { phoneCount: number }).phoneCount === 1);
await me.mutation(api.account.remove, { flow: "smoke" });
check("deleted account", (await code(me.query(api.account.view, { flow: "smoke" }))) === "device_removed");

console.log(failures === 0 ? "smoke: all ok" : `smoke: ${failures} failed`);
if (failures) throw new Error("smoke failed");
