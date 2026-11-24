// VTOP through Cloudflare, for a phone that signs in and fetches itself: the
// same requests, but VTOP's uncompressed pages reach the phone compressed
// (Cloudflare compresses Worker responses for clients that accept it). The
// phone uses this origin as VTOP's, so redirects and cookies are kept on it.
// Only VTOP's own `/vtop/` paths are forwarded, to VTOP and nowhere else.

/** An error reply, `{ code, error }`: the phone's proxy route reads a JSON 502-504 as this
 * Worker failing, and VTOP's own errors are HTML. */
function error(status: number, code: string, message: string): Response {
  return Response.json({ code, error: message }, { status });
}

function notFound(): Response {
  return error(404, "not_found", "Only VTOP's /vtop/ paths go through here.");
}

export const VTOP_ORIGIN = "https://vtop.vitap.ac.in";

/** How long VTOP gets to start answering. Under the app's own 30 s limit,
 * so the phone hears "VTOP is slow" from here instead of giving up and
 * hanging up. */
export const VTOP_TIMEOUT_MS = 25_000;

/** Cap proxied request bodies so `/vtop/*` cannot relay giant payloads. */
export const MAX_BODY_BYTES = 1 << 20;

/** Headers the phone sends that must NOT reach VTOP: hop-by-hop headers, the
 * ones that would describe the proxy instead of the phone, and the client-IP
 * headers Cloudflare adds (anything `cf-*` is stripped too). Everything else
 * is forwarded, so the proxied request matches what the phone sends direct.
 * `origin`/`referer` are forwarded but reoriginated to VTOP. */
const STRIP = new Set([
  "host",
  "content-length",
  "connection",
  "proxy-connection",
  "keep-alive",
  "transfer-encoding",
  "upgrade",
  "te",
  "trailer",
  "x-forwarded-for",
  "x-forwarded-proto",
  "x-forwarded-host",
  "x-real-ip",
  "true-client-ip",
  "forwarded",
  "via",
  "cdn-loop",
]);

export function isVtopPath(path: string): boolean {
  return path === "/vtop" || path.startsWith("/vtop/");
}

const ALLOWED_METHODS = new Set(["GET", "POST", "HEAD"]);

/** `value` with origin `from` swapped for `to`, matching the parsed URL origin so prefix-similar
 * hosts (e.g. `https://proxy.example.evil.com`) are never rewritten. */
function reorigin(value: string, from: string, to: string): string {
  let parsed: URL;
  try {
    parsed = new URL(value);
  } catch {
    return value;
  }
  if (parsed.origin !== from) return value;
  const rest = `${parsed.pathname}${parsed.search}${parsed.hash}`;
  // Bare `Origin` headers carry no trailing slash.
  if (rest === "/" && !value.endsWith("/")) return to;
  return `${to}${rest}`;
}

/** A VTOP cookie as the phone should store it: without `Domain`, so it
 * belongs to the proxy's host. */
function hostOnly(cookie: string): string {
  return cookie
    .split(";")
    .filter((part) => part.trim().toLowerCase().split("=")[0] !== "domain")
    .join(";");
}

/** Reads `req`'s body up to `limit` bytes, returning null if `content-length`
 * or the streamed bytes exceed it. */
async function readBodyCapped(req: Request, limit: number): Promise<ArrayBuffer | null> {
  const declared = Number(req.headers.get("content-length"));
  if (Number.isFinite(declared) && declared > limit) return null;
  if (!req.body) return new ArrayBuffer(0);
  const reader = req.body.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > limit) {
        await reader.cancel();
        return null;
      }
      chunks.push(value);
    }
  } finally {
    reader.releaseLock();
  }
  const out = new Uint8Array(size);
  let at = 0;
  for (const chunk of chunks) {
    out.set(chunk, at);
    at += chunk.byteLength;
  }
  return out.buffer;
}

/** An unexpected failure, at error level, with the error's name, message and the top of its
 * stack. A SyntaxError's message is left out, as it quotes the text it could not parse. Never
 * pass cookies, bodies or URLs (a query can hold a token). */
function logError(message: string, err?: unknown): void {
  if (!(err instanceof Error)) {
    console.error(err === undefined ? message : `${message}: ${typeof err}`);
    return;
  }
  const frames = (err.stack ?? "")
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.startsWith("at "))
    .slice(0, 3);
  const text = err instanceof SyntaxError ? "" : err.message.replace(/\s+/g, " ").slice(0, 200);
  console.error(`${message}: ${err.name}${text ? `: ${text}` : ""}${frames.length ? ` (${frames.join("; ")})` : ""}`);
}

export async function proxyVtop(
  req: Request,
  fetcher: (req: Request) => Promise<Response> = (r) => fetch(r),
  timeoutMs = VTOP_TIMEOUT_MS,
): Promise<Response> {
  const url = new URL(req.url);
  if (!isVtopPath(url.pathname)) return notFound();
  if (!ALLOWED_METHODS.has(req.method)) return error(405, "method_not_allowed", "Method not allowed.");
  const proxy = url.origin;

  const hasBody = req.method !== "GET" && req.method !== "HEAD";
  const body = hasBody ? await readBodyCapped(req, MAX_BODY_BYTES) : undefined;
  if (body === null) return error(413, "too_large", "The request is too large.");

  const headers = new Headers();
  req.headers.forEach((value, name) => {
    if (STRIP.has(name) || name.startsWith("cf-")) return;
    headers.set(
      name,
      name === "origin" || name === "referer"
        ? reorigin(value, proxy, VTOP_ORIGIN)
        : value,
    );
  });
  // Covers VTOP's headers only; once it answers, the body streams through.
  const abort = new AbortController();
  const timer = setTimeout(() => abort.abort(), timeoutMs);
  let upstream: Response;
  try {
    upstream = await fetcher(
      new Request(`${VTOP_ORIGIN}${url.pathname}${url.search}`, {
        method: req.method,
        headers,
        body,
        redirect: "manual",
        signal: abort.signal,
      }),
    );
  } catch (e) {
    if (abort.signal.aborted) {
      console.warn("proxy: VTOP timed out");
      return error(504, "vtop_timeout", "VTOP took too long to answer.");
    }
    // The phone reached us, so this is VTOP being down, not the phone.
    logError("proxy: VTOP unreachable", e);
    // Legacy: 502 here, 503 for the same code elsewhere. The app's proxy
    // route reads a JSON 502-504 as the proxy failing (vtop-ffi route.rs).
    return error(502, "vtop_unreachable", "VTOP can't be reached right now.");
  } finally {
    clearTimeout(timer);
  }

  const out = new Headers();
  upstream.headers.forEach((value, name) => {
    if (name === "set-cookie" || name === "content-length" || name === "content-encoding") return;
    out.set(name, name === "location" ? reorigin(value, VTOP_ORIGIN, proxy) : value);
  });
  for (const cookie of upstream.headers.getSetCookie()) out.append("set-cookie", hostOnly(cookie));
  return new Response(upstream.body, { status: upstream.status, statusText: upstream.statusText, headers: out });
}
