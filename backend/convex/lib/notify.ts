// Waking a phone. Phones watch their state with live queries
// (`sessions:pending`, `mail:inbox`), so a push only wakes an app that is
// closed; it carries what the app needs to act without a round trip.

import { internal } from "../_generated/api";
import type { MutationCtx } from "../_generated/server";
import { deviceRow } from "./identity";
import { now } from "./util";

/** Schedules a push of `kind` to the install; false when there is no such install. */
export async function notify(ctx: MutationCtx, deviceId: string, kind: string, fields: Record<string, string> = {}): Promise<boolean> {
  const device = await deviceRow(ctx, deviceId);
  if (!device) return false;
  if (device.fcmToken) {
    await ctx.scheduler.runAfter(0, internal.push.send, { deviceId, data: { kind, at: String(now()), ...fields }, attempt: 0 });
  }
  return true;
}
