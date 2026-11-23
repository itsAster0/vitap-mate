// UDP mode's gateway: where it is and its public keys. The gateway reports
// them (`/internal/udp-keys`, http.ts); phones read the latest with
// `udp:keys`, so the app follows a move or a new key without a release.

import { v } from "convex/values";
import { internalMutation, query } from "./_generated/server";
import { fail } from "./lib/errors";
import { base64Url } from "./lib/vm1";

export type UdpKey = { id: string; publicKey: string; from: number; until: number };
export type UdpGateway = { host: string; ports: number[]; keys: UdpKey[] };

const isTime = (n: unknown): n is number => Number.isSafeInteger(n) && (n as number) >= 0;

/** The gateway's report, if well formed; public keys are kept unpadded. */
export function parseGateway(raw: Record<string, unknown> | null): UdpGateway | null {
  const { host, ports, keys } = raw ?? {};
  if (typeof host !== "string" || !host.trim() || host.length > 255) return null;
  if (!Array.isArray(ports) || ports.length === 0 || ports.length > 16) return null;
  if (!ports.every((p) => Number.isInteger(p) && p > 0 && p < 65536)) return null;
  if (!Array.isArray(keys) || keys.length === 0 || keys.length > 8) return null;
  const parsed: UdpKey[] = [];
  for (const key of keys as Array<Record<string, unknown> | null>) {
    const { id, publicKey, from, until } = key ?? {};
    if (typeof id !== "string" || !id || id.length > 64 || typeof publicKey !== "string") return null;
    if (base64Url(publicKey)?.length !== 32 || !isTime(from) || !isTime(until)) return null;
    parsed.push({ id, publicKey: publicKey.trim().replace(/=+$/, ""), from, until });
  }
  return { host: host.trim(), ports: ports as number[], keys: parsed };
}

export const keys = query({
  args: {},
  handler: async (ctx): Promise<UdpGateway> => {
    const row = await ctx.db.query("udpGateway").first();
    if (!row) return fail("udp_unavailable", "UDP mode is not set up.");
    return { host: row.host, ports: row.ports, keys: row.keys };
  },
});

export const report = internalMutation({
  args: { host: v.string(), ports: v.array(v.number()), keys: v.array(v.object({ id: v.string(), publicKey: v.string(), from: v.number(), until: v.number() })) },
  handler: async (ctx, gateway) => {
    for (const old of await ctx.db.query("udpGateway").collect()) await ctx.db.delete(old._id);
    await ctx.db.insert("udpGateway", gateway);
  },
});
