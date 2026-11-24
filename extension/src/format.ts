import type { SessionSource } from "./bridge";

export const FIREFOX =
  typeof navigator !== "undefined" && navigator.userAgent.includes("Firefox");

export const BROWSER_NAME = FIREFOX ? "Firefox" : "Chrome";

export const VIA: Record<string, string> = {
  browser: "the browser's session",
  phone: "your phone",
  server: "the server",
};

export function ago(at: number, atNow: number): string {
  const s = Math.max(0, Math.round((atNow - at) / 1000));
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)} min ago`;
  if (s < 86400) return `${Math.floor(s / 3600)} h ago`;
  return `${Math.floor(s / 86400)} d ago`;
}

/** "Cookies from your phone, made 2 h ago, reused". */
export function sourceText(
  via: string | undefined,
  source: SessionSource | undefined,
  atNow: number,
): string {
  if (via === "browser") return "Already in the browser";
  if (!source) {
    return via === "server"
      ? "Cookies from the server sign-in"
      : via === "phone"
        ? "Cookies from your phone"
        : "";
  }
  const who =
    source.from === "server"
      ? "the server sign-in"
      : source.from === "phone"
        ? "your phone"
        : "a saved session";
  const made = source.madeAt ? `, made ${ago(source.madeAt, atNow)}` : "";
  return `Cookies from ${who}${made}, ${source.cached ? "reused" : "new"}`;
}
