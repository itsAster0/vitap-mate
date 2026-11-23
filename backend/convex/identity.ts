// The internal half of lib/identity.ts, for actions.

import { v } from "convex/values";
import { internalQuery } from "./_generated/server";
import { callerOf } from "./lib/identity";

export const check = internalQuery({
  args: { deviceId: v.string(), regno: v.string(), acct: v.string(), demo: v.boolean() },
  handler: (ctx, claims) => callerOf(ctx, claims),
});
