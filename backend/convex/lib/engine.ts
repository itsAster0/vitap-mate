// Calls to the engine at `ENGINE_URL`, with `INTERNAL_TOKEN`, as the edge
// made them (edge/src/common.ts and vtop-phone.ts).

import { type Trace, env } from "./util";

/** Sends a request to the engine. */
export function engineSend(path: string, init: RequestInit): Promise<Response> {
  return fetch(`${env("ENGINE_URL")}${path}`, init);
}

/** Headers that tie the engine's log lines to ours: the student and the app's flow. */
export function traceHeaders(trace: Trace): Record<string, string> {
  return {
    ...(trace.regno ? { "x-vtop-regno": trace.regno } : {}),
    ...(trace.flow ? { "x-vtop-flow": trace.flow.slice(0, 64) } : {}),
  };
}

/** Posts JSON to the engine with `INTERNAL_TOKEN`. */
export function enginePost(path: string, body: unknown, trace: Trace = {}, signal?: AbortSignal): Promise<Response> {
  return engineSend(path, {
    method: "POST",
    headers: { "content-type": "application/json", authorization: `Bearer ${process.env.INTERNAL_TOKEN ?? ""}`, ...traceHeaders(trace) },
    body: typeof body === "string" ? body : JSON.stringify(body),
    signal,
  });
}

/** Posts to the engine and reads a JSON answer; throws with the reason. */
export async function engineJson<T>(path: string, body: unknown, trace: Trace = {}): Promise<T> {
  const response = await enginePost(path, body, trace);
  if (response.status !== 200) throw new Error(`engine answered ${response.status}`);
  return (await response.json()) as T;
}
