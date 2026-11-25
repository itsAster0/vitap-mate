// Waking a phone. Phones watch their state with live queries
// (`sessions:pending`, `mail:inbox`), so a push only wakes an app that is
// closed; it carries what the app needs to act without a round trip.

import { internal } from "../_generated/api";
import type { MutationCtx } from "../_generated/server";
import { deviceFcmToken, deviceRow } from "./identity";
import { now } from "./util";

export type NotifyOutcome = "pushed" | "no_token" | "missing";

/** Schedules a push of `kind` to the install; `missing` when there is no such install,
 * `no_token` when the install exists without an FCM token, `pushed` when FCM was scheduled. */
export async function notify(ctx: MutationCtx, deviceId: string, kind: string, fields: Record<string, string> = {}): Promise<NotifyOutcome> {
  const device = await deviceRow(ctx, deviceId);
  if (!device) return "missing";
  if (deviceFcmToken(device) !== null) {
    await ctx.scheduler.runAfter(0, internal.push.send, { deviceId, data: { kind, at: String(now()), ...fields }, attempt: 0 });
    return "pushed";
  }
  return "no_token";
}
