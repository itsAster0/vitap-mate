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

/** A required environment value, without a trailing slash. */
export function env(name: string): string {
  const value = (process.env[name] ?? "").trim();
  if (!value) throw new Error(`${name} is not configured`);
  return value.replace(/\/+$/, "");
}

/** Who a log line is about: the app's flow id and the student, when known. */
export type Trace = { flow?: string | null; regno?: string | null };

/** One line in the log, led by `flow=` and `regno=` so a student's lines and one action's lines
 * can be pulled together. Never pass cookies, passwords, OTPs, keys or tokens. */
export function log(trace: Trace, message: string): void {
  const tags = [trace.flow ? `flow=${trace.flow.slice(0, 64)}` : "", trace.regno ? `regno=${trace.regno}` : ""].filter(Boolean).join(" ");
  console.log(tags ? `[${tags}] ${message}` : message);
}
