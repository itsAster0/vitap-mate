// Errors as the edge answered them, `{ code, error, ...extra }`: thrown as a
// ConvexError from functions, and as a JSON body with a status from HTTP
// actions. `code` is snake_case and stable; clients match on it.

import { ConvexError, type Value } from "convex/values";

export type ErrorData = { code: string; error: string } & Record<string, Value>;

export function fail(code: string, message: string, extra: Record<string, Value> = {}): never {
  throw new ConvexError<ErrorData>({ code, error: message, ...extra });
}

export const badRequest = (message: string): never => fail("bad_request", message);

/** An error reply for an HTTP action. */
export function error(status: number, code: string, message: string, init: { fields?: Record<string, unknown>; headers?: HeadersInit } = {}): Response {
  return Response.json({ code, error: message, ...init.fields }, { status, headers: init.headers });
}

export function empty(status = 204): Response {
  return new Response(null, { status });
}

/** The `{ code, error }` of a thrown ConvexError, for an HTTP action to answer with. */
export function errorData(e: unknown): ErrorData | null {
  if (e instanceof ConvexError && typeof e.data === "object" && e.data !== null && typeof (e.data as ErrorData).code === "string") {
    return e.data as ErrorData;
  }
  return null;
}
