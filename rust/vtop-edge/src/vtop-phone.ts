// VTOP on the server, for a phone in "Fetch on the server" mode: the engine
// signs in with the phone's username and password and fetches pages on
// the session it hands back, answering with the parsed page only. Nothing
// is kept here or on the engine; the phone sends its session each time.
// Answers are JSON, which Cloudflare sends Brotli-compressed to a phone
// that accepts it (the engine compresses its own hop too), or a page the
// engine packed with the phone's page dictionary, passed on byte for byte.

import { error, type Env, variable } from "./common";

/** Phone route → engine route. Bodies pass through unchanged. */
export const PHONE_ROUTES: Record<string, string> = {
  "/v1/vtop/fetch": "/engine/fetch",
  "/v1/vtop/login": "/engine/vtop-login",
  "/v1/vtop/login/otp": "/engine/vtop-login/otp",
  "/v1/vtop/login/resend": "/engine/vtop-login/resend",
};

/** Bodies are a session and a page name, or a username and password. */
const MAX_BODY_BYTES = 64 * 1024;

/** A page packed with the page dictionary (`vtop_shared::page_dictionary`):
 * Brotli already, which only the phone can unpack. */
export const PACKED_PAGE = "application/vnd.vmate.page+br";

/** Over the engine's own VTOP limits (25 s a page, 28 s a sign-in), so
 * the phone hears the engine's reason instead of a bare timeout. */
export const ENGINE_TIMEOUT_MS = 32_000;

export async function phoneVtop(
  req: Request,
  env: Env,
  fetcher: (req: Request) => Promise<Response> = (r) => fetch(r),
  timeoutMs = ENGINE_TIMEOUT_MS,
): Promise<Response> {
  const enginePath = PHONE_ROUTES[new URL(req.url).pathname];
  if (!enginePath) return error(404, "not_found", "No such route.");
  if (req.method !== "POST") return error(405, "method_not_allowed", "Use POST.");
  const body = await req.arrayBuffer();
  if (body.byteLength > MAX_BODY_BYTES) return error(413, "too_large", "The request is too large.");

  const abort = new AbortController();
  const timer = setTimeout(() => abort.abort(), timeoutMs);
  let answer: Response;
  try {
    answer = await fetcher(
      new Request(`${variable(env, "ENGINE_URL")}${enginePath}`, {
        method: "POST",
        headers: {
          "content-type": "application/json",
          authorization: `Bearer ${env.BRIDGE_SECRET}`,
          "accept-encoding": "br, gzip",
        },
        body,
        signal: abort.signal,
      }),
    );
    // Read here, under the timer.
    if (answer.headers.get("content-type") === PACKED_PAGE) {
      const packed = await answer.arrayBuffer();
      return new Response(packed, {
        status: answer.status,
        // no-transform: Cloudflare must not compress it again.
        headers: { "content-type": PACKED_PAGE, "cache-control": "no-store, no-transform" },
      });
    }
    // `fetch` has already undone the engine's Brotli, and Cloudflare
    // compresses again for the phone.
    const text = await answer.text();
    return new Response(text, {
      status: answer.status,
      headers: { "content-type": "application/json", "cache-control": "no-store" },
    });
  } catch (e) {
    if (abort.signal.aborted) return error(504, "vtop_timeout", "VTOP took too long to answer.");
    console.log(`vtop-phone: engine unreachable: ${e}`);
    return error(503, "engine_unavailable", "The server can't reach VTOP right now.");
  } finally {
    clearTimeout(timer);
  }
}
