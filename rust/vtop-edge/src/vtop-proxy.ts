// VTOP through the edge, for a phone that signs in and fetches itself: the
// same requests, but VTOP's uncompressed pages reach the phone compressed
// (Cloudflare compresses Worker responses for clients that accept it). The
// phone uses this origin as VTOP's, so redirects and cookies are kept on it.
// Only VTOP's own `/vtop/` paths are forwarded, to VTOP and nowhere else.

import { error } from "./common";

export const VTOP_ORIGIN = "https://vtop.vitap.ac.in";

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

/** `value` with the proxy's origin swapped for `to`. */
function reorigin(value: string, from: string, to: string): string {
  return value.startsWith(from) ? to + value.slice(from.length) : value;
}

/** A VTOP cookie as the phone should store it: without `Domain`, so it
 * belongs to the proxy's host. */
function hostOnly(cookie: string): string {
  return cookie
    .split(";")
    .filter((part) => part.trim().toLowerCase().split("=")[0] !== "domain")
    .join(";");
}

export async function proxyVtop(
  req: Request,
  fetcher: (req: Request) => Promise<Response> = (r) => fetch(r),
): Promise<Response> {
  const url = new URL(req.url);
  if (!isVtopPath(url.pathname)) return error(404, "not_found", "No such route.");
  const proxy = url.origin;

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
  const hasBody = req.method !== "GET" && req.method !== "HEAD";
  const upstream = await fetcher(
    new Request(`${VTOP_ORIGIN}${url.pathname}${url.search}`, {
      method: req.method,
      headers,
      body: hasBody ? await req.arrayBuffer() : undefined,
      redirect: "manual",
    }),
  );

  const out = new Headers();
  upstream.headers.forEach((value, name) => {
    if (name === "set-cookie" || name === "content-length" || name === "content-encoding") return;
    out.set(name, name === "location" ? reorigin(value, VTOP_ORIGIN, proxy) : value);
  });
  for (const cookie of upstream.headers.getSetCookie()) out.append("set-cookie", hostOnly(cookie));
  return new Response(upstream.body, { status: upstream.status, statusText: upstream.statusText, headers: out });
}
