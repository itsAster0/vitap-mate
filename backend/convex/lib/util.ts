// Small things every module uses.

/** Unix seconds. */
export function now(): number {
  return Math.floor(Date.now() / 1000);
}

/** A string field of a parsed body. */
export function str(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

/** A JSON object, or null when the text is not one. */
export function parseObject(text: string): Record<string, unknown> | null {
  try {
    const body: unknown = JSON.parse(text);
    return body !== null && typeof body === "object" && !Array.isArray(body) ? (body as Record<string, unknown>) : null;
  } catch {
    return null;
  }
}

/** Account ids are upper-cased registration numbers, as the engine makes them. */
export function accountId(registrationNumber: string): string {
  return registrationNumber.trim().toUpperCase();
}

/** A required environment value, failing fast when unset or blank. */
export function requiredEnv(name: string): string {
  const value = (process.env[name] ?? "").trim();
  if (!value) throw new Error(`${name} is not configured`);
  return value;
}

/** A required environment value (URL or domain), without a trailing slash. */
export function env(name: string): string {
  return requiredEnv(name).replace(/\/+$/, "");
}

/** Who a log line is about: the app's flow id and the student, when known. */
export type Trace = { flow?: string | null; regno?: string | null };

/** One line in the log, led by `flow=` and `regno=` so a student's lines and one action's lines
 * can be pulled together. Never pass cookies, passwords, OTPs, keys or tokens. */
export function log(trace: Trace, message: string): void {
  console.log(traced(trace, message));
}

/** A failure the caller has to put up with, such as VTOP being slow. Same rules as `log`. */
export function logWarn(trace: Trace, message: string): void {
  console.warn(traced(trace, message));
}

/** An unexpected failure, at error level: `message` with the error's name, message and the top
 * of its stack. A SyntaxError's message is left out, as it quotes the text it could not parse
 * (an engine reply can hold cookies or a session). Same rules as `log`. */
export function logError(trace: Trace, message: string, err?: unknown): void {
  console.error(traced(trace, err === undefined ? message : `${message}: ${describeError(err)}`));
}

/** `name: message (at …; at …)` of a thrown value, short enough for one log line.
 * Scrubs `/mcp/<key>` path segments so access keys never leak in error logs. */
export function describeError(err: unknown): string {
  if (!(err instanceof Error)) return typeof err;
  const text =
    err instanceof SyntaxError
      ? ""
      : err.message
          .replace(/\/mcp\/[^\s/?#"')]+/g, "/mcp/…")
          .replace(/\s+/g, " ")
          .slice(0, 200);
  const frames = (err.stack ?? "")
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.startsWith("at "))
    .slice(0, 3);
  return `${err.name}${text ? `: ${text}` : ""}${frames.length ? ` (${frames.join("; ")})` : ""}`;
}

function traced(trace: Trace, message: string): string {
  const tags = [trace.flow ? `flow=${trace.flow.slice(0, 64)}` : "", trace.regno ? `regno=${trace.regno}` : ""].filter(Boolean).join(" ");
  return tags ? `[${tags}] ${message}` : message;
}
