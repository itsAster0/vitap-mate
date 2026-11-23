// Account proofs: a phone proves its registration number with TLSNotary
// (rust/vtop-tlsn) instead of sending its VTOP cookie, and gets the same
// account token the cookie check gives (account-token.ts).
//
// 1. `POST /v1/account/prove/start` (signed) hands the install a challenge,
//    kept in its `Device` object for 2 minutes, one at a time.
// 2. `GET /v1/account/prove?challenge=` (signed, a WebSocket) uses it up,
//    then opens the engine's `/engine/proof` with `INTERNAL_TOKEN`, naming
//    this install and the challenge, and relays binary messages both ways.
//    The engine holds the proof to a binding of both, dials VTOP itself and
//    never sees the session.
// 3. The engine's last message is its verdict (text), which the phone never
//    sees: on `ok` the student's `Account` issues the token as after a cookie
//    check, and the phone's last message is `{ accountToken, expiresAt,
//    registrationNumber }`, or `{ code, error }`. Nothing the phone sends
//    names the student.

import { accountToken } from "./account";
import type { Signed } from "./auth";
import { type Env, deviceStub, engineSend, error, errorBody, unknownDevice } from "./common";
import { PROOF_VERIFY, type ProofVerdict } from "./shared/engine-api";

/** What the relay needs of a WebSocket, so tests can pass their own. */
export interface Socket {
  send(data: string | ArrayBuffer): void;
  close(code?: number, reason?: string): void;
  addEventListener(type: "message", listener: (event: { data: unknown }) => void): void;
  addEventListener(type: "close" | "error", listener: () => void): void;
}

/** `POST /v1/account/prove/start`: a new challenge for the calling install. */
export async function proveStart(auth: Signed, env: Env): Promise<Response> {
  const proof = await deviceStub(env, auth.device.id).startProof();
  if (proof === "unknown") return unknownDevice();
  if (proof === "too_many") return error(429, "too_many_proofs", "Too many proofs from this phone; wait a little.");
  return Response.json({ challenge: proof.challenge, expiresAt: proof.expiresAt * 1000 });
}

/** `GET /v1/account/prove?challenge=`: the proof's WebSocket, once the
 * install's challenge is used up and the engine took the relay. */
export async function proveSocket(auth: Signed, req: Request, env: Env): Promise<Response> {
  if (req.headers.get("upgrade")?.toLowerCase() !== "websocket") {
    return error(426, "upgrade_required", "Open this route as a WebSocket.");
  }
  const device = auth.device.id;
  const challenge = new URL(req.url).searchParams.get("challenge") ?? "";
  if (!(await deviceStub(env, device).takeProof(challenge))) {
    return error(401, "challenge_invalid", "Unknown, used or expired challenge; start the proof again.");
  }
  let engine: WebSocket | null = null;
  let status = 0;
  try {
    const query = new URLSearchParams({ device, challenge });
    const response = await engineSend(env, `${PROOF_VERIFY}?${query}`, {
      headers: { upgrade: "websocket", authorization: `Bearer ${env.INTERNAL_TOKEN}` },
    });
    engine = response.webSocket ?? null;
    status = response.status;
  } catch (e) {
    console.log(`account proof: engine unreachable: ${e}`);
  }
  if (engine === null) {
    console.log(`account proof: engine refused the relay: ${status}`);
    return status === 503
      ? error(503, "proof_busy", "The server is busy with other proofs; try again shortly.")
      : error(503, "engine_unavailable", "The server cannot check proofs right now; try again.");
  }
  engine.accept();
  const [client, phone] = Object.values(new WebSocketPair());
  phone.accept();
  relay(phone, engine, (verdict) => proofReply(env, device, verdict));
  console.log("account proof: relaying");
  return new Response(null, { status: 101, webSocket: client });
}

/** The engine's verdict, or null when it is not one. */
export function parseVerdict(text: string): ProofVerdict | null {
  try {
    const v = JSON.parse(text) as Record<string, unknown>;
    if (v.status === "ok" && typeof v.registrationNumber === "string" && typeof v.ownerHash === "string") {
      return { status: "ok", registrationNumber: v.registrationNumber, ownerHash: v.ownerHash };
    }
    if (v.status === "failed" && typeof v.reason === "string") return { status: "failed", reason: v.reason };
  } catch {}
  return null;
}

/** The phone's last message for `verdict`: the token for install `device`,
 * issued as after a cookie check, or why there is none. */
export async function proofReply(env: Env, device: string, verdict: ProofVerdict | null): Promise<Record<string, unknown>> {
  if (verdict?.status !== "ok") {
    console.log("account proof: no proof");
    return errorBody("proof_failed", `VTOP's page did not prove who you are: ${verdict?.reason ?? "no verdict"}`);
  }
  const issued = await accountToken(env, verdict.registrationNumber, device, verdict.ownerHash);
  if (issued === null) return errorBody("token_unavailable", "The server cannot issue account tokens right now.");
  console.log("account proof: account token issued");
  return { accountToken: issued.token, expiresAt: issued.expiresAt, registrationNumber: verdict.registrationNumber.toUpperCase() };
}

/** Relays binary messages between `phone` and `engine` until the engine's
 * text verdict, which goes to `reply` and never to the phone; the phone gets
 * `reply`'s answer as its last message. Text from the phone, or either side
 * closing first, ends both. */
export function relay(
  phone: Socket,
  engine: Socket,
  reply: (verdict: ProofVerdict | null) => Promise<Record<string, unknown>>,
): void {
  let done = false;
  const end = (code: number, reason: string) => {
    done = true;
    for (const socket of [phone, engine]) {
      try {
        socket.close(code, reason);
      } catch {}
    }
  };
  // A socket can close under any send; that ends the relay instead of throwing.
  const send = (socket: Socket, data: string | ArrayBuffer): boolean => {
    try {
      socket.send(data);
      return true;
    } catch {
      end(1011, "send failed");
      return false;
    }
  };
  phone.addEventListener("message", ({ data }) => {
    if (done) return;
    if (data instanceof ArrayBuffer) send(engine, data);
    else end(1008, "binary messages only");
  });
  engine.addEventListener("message", ({ data }) => {
    if (done) return;
    if (data instanceof ArrayBuffer) {
      send(phone, data);
      return;
    }
    done = true;
    const verdict = typeof data === "string" ? parseVerdict(data) : null;
    reply(verdict)
      .then(
        (answer) => answer,
        (e) => {
          console.log(`account proof: issuing failed: ${e}`);
          return errorBody("internal_error", "Something went wrong on the server. Try again.");
        },
      )
      .then((answer) => {
        // The phone may have gone while the token was minted; the sockets close either way.
        try {
          phone.send(JSON.stringify(answer));
        } catch {}
        end(1000, "done");
      });
  });
  for (const socket of [phone, engine]) {
    socket.addEventListener("close", () => done || end(1000, "closed"));
    socket.addEventListener("error", () => done || end(1011, "error"));
  }
}
