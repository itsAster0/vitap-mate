// The backfill from the edge's Durable Objects: exports shaped as
// `POST /internal/export` answers, imported under their old ids, again and again.

import { afterEach, describe, expect, it, vi } from "vitest";
import { REGNO, T0, TOKENS, type T, advance, codeOf, phone, prove, setup } from "../test/kit";
import { api, internal } from "./_generated/api";
import { issueV2Token } from "./lib/jwt";
import { ensureAccount } from "./lib/accounts";
import { sha256Hex } from "./lib/crypto";
import type { exported } from "./migrate";

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

type Item = typeof exported.type;
type Row = Record<string, string | number | null>;

const KEY = "vtm_imported-key-for-the-test";
const OLD = T0 - 86_400;

const item = (kind: Item["kind"], tables: Record<string, Row[]>, extra: Partial<Item> = {}): Item => ({
  kind,
  found: Object.values(tables).some((rows) => rows.length > 0),
  tables,
  kv: {},
  omitted: {},
  ...extra,
});

async function deviceExport(n: number) {
  const p = await phone(n);
  return {
    p,
    item: item("device", {
      device: [{ id: p.deviceId, sign_key: p.signKey, seal_key: p.sealKey, label: `Phone ${n}`, platform: "android", fcm_token: "fcm-1", seq: 3, created_at: OLD }],
    }),
  };
}

async function accountExport(deviceIds: string[], over: Row = {}, keys: Row[] | null = null, tokenDevices: string[] = deviceIds.slice(0, 1)) {
  const keyHash = await sha256Hex(KEY);
  return item("account", {
    account: [
      {
        id: REGNO,
        always_use_vault: 0,
        phone_wait_secs: 45,
        vault_ttl_secs: 86_400,
        semester_id: "AP2026275",
        mailbox_id: "abcdefghij",
        sealed_vault: "sealed-vault",
        vault_saved_at: OLD,
        sealed_session: "sealed-session",
        session_saved_at: OLD,
        session_source: "phone",
        credentials_rejected_at: null,
        vault_gmail: 1,
        created_at: OLD,
        updated_at: OLD + 10,
        window_start: OLD,
        window_hits: 2,
        ...over,
      },
    ],
    keys: keys ?? [{ hash: keyHash, label: "laptop", created_at: OLD, last_used_at: OLD + 5, device_id: deviceIds[0], outings: 1 }],
    key_uses: [
      { key_hash: keyHash, at: OLD + 1, via: "extension" },
      { key_hash: keyHash, at: OLD + 5, via: "mcp" },
    ],
    phones: deviceIds.map((d, i) => ({ device_id: d, label: null, added_at: OLD + i, seen_at: OLD + 100 + i })),
    sign_ins: [{ at: OLD + 3, by: "server", ok: 1, reason: null }],
    owner: [{ hash: "owner-hash" }],
    tokens: tokenDevices.map((d) => ({ device_id: d, issued_at: OLD })),
  });
}

function mailboxExport(members: string[], over: Row = {}, regno: string | null = REGNO) {
  return item(
    "mailbox",
    {
      mailbox: [
        {
          id: "abcdefghij",
          owner_hash: "owner-hash",
          members: JSON.stringify(members.map((deviceId) => ({ deviceId, sealed: "old-box", sealedAt: OLD, addedAt: OLD, seenAt: OLD + 7 }))),
          created_at: OLD,
          updated_at: OLD + 20,
          ...over,
        },
      ],
    },
    { kv: { gmail_confirm: '{"kind":"gmail_confirm"}', gmail_confirm_at: OLD + 2, last_otp_at: OLD + 1 }, ...(regno !== null ? { regno } : {}) },
  );
}

const oauthExport = () =>
  item("oauth", { clients: [{ id: "client-1", name: "Claude", redirect_uris: '["https://claude.ai/cb"]', created_at: OLD, used_at: OLD + 9 }] });

const run = (t: T, items: Item[]) => t.mutation(internal.migrate.importBatch, { items });

async function rows(t: T) {
  return t.run(async (ctx) => ({
    accounts: await ctx.db.query("accounts").collect(),
    devices: await ctx.db.query("devices").collect(),
    keys: await ctx.db.query("accessKeys").collect(),
    signIns: await ctx.db.query("signIns").collect(),
    mailboxes: await ctx.db.query("mailboxes").collect(),
    members: await ctx.db.query("mailboxDevices").collect(),
    clients: await ctx.db.query("oauthClients").collect(),
  }));
}

describe("the backfill", () => {
  it("maps every kind into the v3 model under the old ids", async () => {
    const t = await setup();
    const one = await deviceExport(1);
    const two = await deviceExport(2);
    const tally = await run(t, [one.item, two.item, await accountExport([one.p.deviceId, two.p.deviceId]), mailboxExport([one.p.deviceId]), oauthExport()]);
    expect(tally).toEqual({
      "devices.inserted": 2,
      "accounts.inserted": 1,
      "v2Tokens.inserted": 1,
      "accessKeys.inserted": 1,
      "signIns.inserted": 1,
      "phones.linked": 2,
      "mailboxes.inserted": 1,
      "mailboxDevices.inserted": 1,
      "oauthClients.inserted": 1,
    });
    const r = await rows(t);
    expect(r.accounts).toHaveLength(1);
    const account = r.accounts[0];
    expect(account).toMatchObject({
      regno: REGNO,
      ownerHash: "owner-hash",
      settings: { alwaysUseVault: false, phoneWaitSecs: 45, vaultTtlSecs: 86_400 },
      semesterId: "AP2026275",
      sealedVault: "sealed-vault",
      vaultGmail: true,
      sealedSession: "sealed-session",
      sessionSource: "phone",
      sweepAt: OLD + 86_400,
      createdAt: OLD,
      updatedAt: OLD + 10,
    });
    expect(r.devices.map((d) => ({ id: d.deviceId, accountId: d.accountId, linkedAt: d.linkedAt, seenAt: d.seenAt, fcm: d.fcmToken }))).toEqual([
      { id: one.p.deviceId, accountId: account._id, linkedAt: OLD, seenAt: OLD + 100, fcm: "fcm-1" },
      { id: two.p.deviceId, accountId: account._id, linkedAt: OLD + 1, seenAt: OLD + 101, fcm: "fcm-1" },
    ]);
    expect(r.keys).toEqual([
      expect.objectContaining({
        accountId: account._id,
        label: "laptop",
        deviceId: one.p.deviceId,
        outings: true,
        lastUsedAt: OLD + 5,
        uses: [
          { at: OLD + 5, via: "mcp" },
          { at: OLD + 1, via: "extension" },
        ],
      }),
    ]);
    expect(r.signIns).toEqual([expect.objectContaining({ accountId: account._id, at: OLD + 3, by: "server", ok: true })]);
    expect(r.mailboxes).toEqual([expect.objectContaining({ addressId: "abcdefghij", accountId: account._id, lastOtpAt: OLD + 1, confirmationAt: OLD + 2 })]);
    expect(r.members).toEqual([expect.objectContaining({ addressId: "abcdefghij", deviceId: one.p.deviceId, addedAt: OLD, seenAt: OLD + 7, accountId: account._id })]);
    expect(r.clients).toEqual([expect.objectContaining({ clientId: "client-1", redirectUris: ["https://claude.ai/cb"], usedAt: OLD + 9 })]);

    // The imported key opens the key routes as it did on the edge.
    const whoami = await t.fetch("/v1/whoami", { headers: { authorization: `Bearer ${KEY}` } });
    expect(whoami.status).toBe(200);
    expect(await whoami.json()).toMatchObject({ registrationNumber: REGNO });
  });

  it("changes nothing when run again", async () => {
    const t = await setup();
    const one = await deviceExport(1);
    const batch = [one.item, await accountExport([one.p.deviceId]), mailboxExport([one.p.deviceId]), oauthExport()];
    await run(t, batch);
    const before = await rows(t);
    expect(await run(t, batch)).toEqual({
      "devices.exists": 1,
      "v2Tokens.exists": 1,
      "accounts.current": 1,
      "mailboxes.current": 1,
      "oauthClients.exists": 1,
    });
    expect(await rows(t)).toEqual(before);
  });

  it("brings an imported account up to the edge's later state (the delta pass)", async () => {
    const t = await setup();
    const one = await deviceExport(1);
    const two = await deviceExport(2);
    await run(t, [one.item, two.item, await accountExport([one.p.deviceId, two.p.deviceId]), mailboxExport([one.p.deviceId, two.p.deviceId])]);
    // On the edge meanwhile: the key revoked, phone 2 removed, a setting changed, session and vault cleared, phone 2 left the address.
    const later = await accountExport([one.p.deviceId], { updated_at: OLD + 50, phone_wait_secs: 10, sealed_vault: null, sealed_session: null }, []);
    const tally = await run(t, [later, mailboxExport([one.p.deviceId], { updated_at: OLD + 60 })]);
    expect(tally).toMatchObject({ "accounts.updated": 1, "accessKeys.deleted": 1, "phones.unlinked": 1, "mailboxes.updated": 1 });
    const r = await rows(t);
    expect(r.accounts[0]).toMatchObject({ updatedAt: OLD + 50, settings: { phoneWaitSecs: 10 } });
    expect(r.accounts[0].sweepAt).toBeUndefined();
    expect(r.keys).toEqual([]);
    expect(r.devices.find((d) => d.deviceId === two.p.deviceId)?.accountId).toBeUndefined();
    expect(r.members.map((m) => m.deviceId)).toEqual([one.p.deviceId]);
  });

  it("leaves a phone that signed out on v3 off the account and its address (the delta pass)", async () => {
    const t = await setup();
    const one = await deviceExport(1);
    const two = await deviceExport(2);
    await run(t, [one.item, two.item, await accountExport([one.p.deviceId, two.p.deviceId]), mailboxExport([one.p.deviceId, two.p.deviceId])]);
    // Phone 2 proved on v3, then signed out there; the edge still lists it.
    await t.run(async (ctx) => {
      const device = (await ctx.db.query("devices").collect()).find((d) => d.deviceId === two.p.deviceId)!;
      await ctx.db.patch(device._id, { accountId: undefined, linkedOnce: true });
      for (const m of await ctx.db.query("mailboxDevices").collect()) if (m.deviceId === two.p.deviceId) await ctx.db.delete(m._id);
    });
    const tally = await run(t, [
      await accountExport([one.p.deviceId, two.p.deviceId], { updated_at: OLD + 50 }),
      mailboxExport([one.p.deviceId, two.p.deviceId], { updated_at: OLD + 60 }),
    ]);
    expect(tally).toMatchObject({ "phones.left_on_v3": 1, "mailboxDevices.left_on_v3": 1 });
    const r = await rows(t);
    expect(r.devices.find((d) => d.deviceId === two.p.deviceId)?.accountId).toBeUndefined();
    expect(r.members.map((m) => m.deviceId)).toEqual([one.p.deviceId]);
  });

  it("never undoes what Convex wrote after the import", async () => {
    const t = await setup();
    const one = await deviceExport(1);
    const batch = [one.item, await accountExport([one.p.deviceId])];
    await run(t, batch);
    // In Convex after the switch: the key revoked and the account touched.
    await t.run(async (ctx) => {
      const account = (await ctx.db.query("accounts").first())!;
      await ctx.db.patch(account._id, { semesterId: "AP2026276", updatedAt: T0 });
      for (const key of await ctx.db.query("accessKeys").collect()) await ctx.db.delete(key._id);
    });
    expect(await run(t, batch)).toEqual({ "devices.exists": 1, "v2Tokens.exists": 1, "accounts.current": 1 });
    const r = await rows(t);
    expect(r.accounts[0].semesterId).toBe("AP2026276");
    expect(r.keys).toEqual([]);
  });

  it("adds the old rows to an account Convex made first, once", async () => {
    const t = await setup();
    const p = await phone(1);
    await prove(t, p);
    const two = await deviceExport(2);
    const batch = [two.item, await accountExport([p.deviceId, two.p.deviceId])];
    const tally = await run(t, batch);
    expect(tally).toMatchObject({ "accounts.merged": 1, "accessKeys.inserted": 1, "phones.linked_already": 1, "phones.linked": 1 });
    let r = await rows(t);
    // Convex's own fields stay; the account dates from the edge now.
    expect(r.accounts[0]).toMatchObject({ createdAt: OLD, updatedAt: T0, ownerHash: `owner-${REGNO}`, sealedVault: null });
    expect(r.devices.every((d) => d.accountId === r.accounts[0]._id)).toBe(true);
    advance(60);
    expect(await run(t, batch)).toEqual({ "devices.exists": 1, "v2Tokens.exists": 1, "accounts.current": 1 });
    r = await rows(t);
    expect(r.keys).toHaveLength(1);
  });

  it("imports the edge's live v2 tokens: each trades in once, a revoked one never", async () => {
    const t = await setup();
    const a = await phone(1);
    const b = await phone(2);
    const live = (p: typeof a) => issueV2Token(TOKENS.ACCOUNT_TOKEN_SECRET, { sub: REGNO, dev: p.deviceId, iat: OLD, exp: T0 + 1000 });
    const trade = async (p: typeof a, token: string) => t.action(api.auth.token, await p.signed("auth:token", { v2AccountToken: token, device: p.device }));
    await run(t, [await accountExport([], {}, undefined, [a.deviceId, b.deviceId])]);
    // Revoked on the edge (phone b removed) before the delta pass.
    expect(await run(t, [await accountExport([], { updated_at: OLD + 50 }, undefined, [a.deviceId])])).toMatchObject({ "v2Tokens.deleted": 1 });
    expect(await codeOf(trade(b, await live(b)))).toBe("token_invalid");
    expect((await trade(a, await live(a))).regno).toBe(REGNO);
    expect(await codeOf(trade(a, await live(a)))).toBe("token_invalid");
    // A newer token the edge issued but the import never saw has no row either.
    const unseen = await issueV2Token(TOKENS.ACCOUNT_TOKEN_SECRET, { sub: REGNO, dev: a.deviceId, iat: OLD + 1, exp: T0 + 1000 });
    expect(await codeOf(trade(a, unseen))).toBe("token_invalid");
    // Importing again never revives a used token.
    await run(t, [await accountExport([], { updated_at: OLD + 60 }, undefined, [a.deviceId])]);
    expect(await codeOf(trade(a, await live(a)))).toBe("token_invalid");
  });

  it("leaves what it can't place, and says why", async () => {
    const t = await setup();
    const one = await deviceExport(1);
    const other = await phone(9);
    // A row whose id is not its key's hash.
    const forged = item("device", { device: [{ ...one.item.tables.device[0], sign_key: other.signKey }] });
    const tally = await run(t, [
      forged,
      item("device", {}, { found: false }),
      await accountExport([one.p.deviceId]),
      mailboxExport([one.p.deviceId], { id: "zzzzzzzzzz", owner_hash: "nobody" }, null),
    ]);
    expect(tally).toMatchObject({
      "devices.bad_key": 1,
      "device.nothing_stored": 1,
      "phones.no_device": 1,
      "mailboxes.unclaimed": 1,
    });
    const r = await rows(t);
    expect(r.devices).toEqual([]);
    expect(r.mailboxes).toEqual([]);
  });

  it("keeps an address with no account yet for the student who proves its owner hash", async () => {
    const t = await setup();
    const one = await deviceExport(1);
    const lone = mailboxExport([one.p.deviceId], { id: "kkkkkkkkkk", owner_hash: "later-owner" }, null);
    expect(await run(t, [one.item, lone])).toMatchObject({ "mailboxes.unclaimed": 1 });
    expect(await run(t, [lone])).toMatchObject({ "mailboxes.unclaimed_kept": 1 });
    const claimed = await t.run(async (ctx) => {
      const account = await ensureAccount(ctx, "24abc0001", "later-owner");
      return {
        account,
        mailboxes: await ctx.db.query("mailboxes").collect(),
        waiting: await ctx.db.query("unclaimedMailboxes").collect(),
      };
    });
    expect(claimed.waiting).toEqual([]);
    expect(claimed.mailboxes).toEqual([expect.objectContaining({ addressId: "kkkkkkkkkk", accountId: claimed.account._id, lastOtpAt: OLD + 1 })]);
  });

  it("lets the old address win over one the account made on v3 first", async () => {
    const t = await setup();
    const one = await deviceExport(1);
    await run(t, [one.item, await accountExport([one.p.deviceId])]);
    await t.run(async (ctx) => {
      const account = (await ctx.db.query("accounts").first())!;
      await ctx.db.insert("mailboxes", { addressId: "newnewnewn", accountId: account._id, confirmation: null, confirmationAt: null, lastOtpAt: null, createdAt: T0, updatedAt: T0 });
      await ctx.db.insert("mailboxDevices", { addressId: "newnewnewn", deviceId: one.p.deviceId, addedAt: T0, seenAt: T0, accountId: account._id });
    });
    expect(await run(t, [mailboxExport([one.p.deviceId])])).toMatchObject({ "mailboxes.replaced": 1, "mailboxes.inserted": 1 });
    const r = await rows(t);
    expect(r.mailboxes.map((m) => m.addressId)).toEqual(["abcdefghij"]);
    expect(r.members).toEqual([expect.objectContaining({ addressId: "abcdefghij", deviceId: one.p.deviceId })]);
  });
});
