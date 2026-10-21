// VTOP through the edge, for a phone that signs in and fetches itself: the
// same requests, but VTOP's uncompressed pages reach the phone compressed
// (Cloudflare compresses Worker responses for clients that accept it). The
// phone uses this origin as VTOP's, so redirects and cookies are kept on it.
// Only VTOP's own `/vtop/` paths are forwarded, to VTOP and nowhere else.

import { error } from "./common";

export const VTOP_ORIGIN = "https://vtop.vitap.ac.in";

/** Request headers VTOP gets from the phone; the rest are Cloudflare's or
 * would describe the proxy. */
const FORWARDED = [
  "accept",
  "accept-language",
  "content-type",
  "cookie",
  "user-agent",
  "x-requested-with",
  "sec-fetch-dest",
  "sec-fetch-mode",
  "sec-fetch-site",
];

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
  for (const name of FORWARDED) {
    const value = req.headers.get(name);
    if (value !== null) headers.set(name, value);
  }
  for (const name of ["origin", "referer"]) {
    const value = req.headers.get(name);
    if (value !== null) headers.set(name, reorigin(value, proxy, VTOP_ORIGIN));
  }
  // VTOP sees Cloudflare's address; this names the phone's, which VTOP
  // records in its sign-in history. (With X-Real-IP too it lists it twice.)
  const phone = req.headers.get("cf-connecting-ip");
  if (phone) headers.set("x-forwarded-for", phone);
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
