// The one-time backfill from the edge's Durable Objects (`edge/src/export.ts`,
// driven by `scripts/backfill.ts`). Each item is one object's export, its rows
// as the edge stored them, and lands under its old ids: devices by device id,
// accounts by regno, keys by hash, mailboxes by address id, OAuth clients by
// client id.
//
// Runs again and again without harm, and never undoes a write made in Convex:
// - A missing row is inserted.
// - An account or mailbox this import made (same `createdAt`) is brought up to
//   the old one only while the old one is newer (`updatedAt`), which is how the
//   delta pass after the freeze picks up the last changes. Once Convex writes to
//   it, its `updatedAt` passes the frozen old one and the import leaves it be.
// - An account Convex made first (a phone traded in its v2 token before the
//   backfill) gets the rows it lacks, and the old `createdAt`, so the next run
//   treats it as imported.
// - A device is only ever inserted: the app sends its keys, label and FCM token
//   again with every `auth:token`.
//
// Admin only: an internal mutation, called with the deployment's admin key.

import { v } from "convex/values";
import type { Doc, Id } from "./_generated/dataModel";
import { type MutationCtx, internalMutation } from "./_generated/server";
import {
  MAX_KEY_USES,
  MAX_PHONES,
  MAX_SIGN_INS,
  isAccountId,
  sweepAtOf,
} from "./lib/accountState";
import { accountByRegno, phoneRows, unlinkPhone } from "./lib/accounts";
import { fromBase64Url } from "./lib/base64";
import {
  capMembers,
  claimMailbox,
  giveWayTo,
  mailboxRow,
} from "./lib/mailboxes";
import { deviceIdFor } from "./lib/crypto";

const sqlValue = v.union(v.string(), v.number(), v.null());
type SqlValue = string | number | null;
type Row = Record<string, SqlValue>;

export const exported = v.object({
  kind: v.union(
    v.literal("device"),
    v.literal("account"),
    v.literal("mailbox"),
    v.literal("oauth"),
  ),
  id: v.optional(v.string()),
  objectId: v.optional(v.string()),
  found: v.boolean(),
  tables: v.record(v.string(), v.array(v.record(v.string(), sqlValue))),
  kv: v.record(v.string(), sqlValue),
  omitted: v.record(v.string(), v.number()),
  /** A mailbox's student, as the script read it from the accounts' `mailbox_id`. */
  regno: v.optional(v.string()),
});

type Item = typeof exported.type;
/** What happened to each row, as `<table>.<outcome>` → how many. */
type Tally = Record<string, number>;

// ---- reading the edge's rows --------------------------------------------------------

const text = (row: Row, column: string): string | null =>
  typeof row[column] === "string" ? (row[column] as string) : null;
const int = (row: Row, column: string): number | null =>
  typeof row[column] === "number" && (row[column] as number) >= 0
    ? (row[column] as number)
    : null;
const flag = (row: Row, column: string): boolean =>
  (int(row, column) ?? 0) !== 0;

function count(tally: Tally, key: string, n = 1): void {
  tally[key] = (tally[key] ?? 0) + n;
}

/** The account's fields as the edge's `account` row has them (account-db.ts). */
function accountFields(row: Row) {
  return {
    settings: {
      alwaysUseVault: flag(row, "always_use_vault"),
      phoneWaitSecs: int(row, "phone_wait_secs") ?? 20,
      vaultTtlSecs: int(row, "vault_ttl_secs"),
    },
    semesterId: text(row, "semester_id"),
    sealedVault: text(row, "sealed_vault"),
    vaultSavedAt: int(row, "vault_saved_at"),
    vaultGmail: flag(row, "vault_gmail"),
    sealedSession: text(row, "sealed_session"),
    sessionSavedAt: int(row, "session_saved_at"),
    sessionSource: text(row, "session_source"),
    credentialsRejectedAt: int(row, "credentials_rejected_at"),
    createdAt: int(row, "created_at") ?? 0,
    updatedAt: int(row, "updated_at") ?? 0,
  };
}

type OldKey = Omit<Doc<"accessKeys">, "_id" | "_creationTime" | "accountId">;

function keysOf(tables: Item["tables"]): OldKey[] {
  const uses = (tables.key_uses ?? [])
    .flatMap(
      (u): Array<{ keyHash: string; at: number; via: "extension" | "mcp" }> => {
        const at = int(u, "at");
        const via = text(u, "via");
        const keyHash = text(u, "key_hash");
        return at !== null &&
          keyHash !== null &&
          (via === "extension" || via === "mcp")
          ? [{ keyHash, at, via }]
          : [];
      },
    )
    .sort((a, b) => b.at - a.at);
  return (tables.keys ?? []).flatMap((k) => {
    const keyHash = text(k, "hash");
    const label = text(k, "label");
    const createdAt = int(k, "created_at");
    const deviceId = text(k, "device_id");
    if (
      keyHash === null ||
      label === null ||
      createdAt === null ||
      deviceId === null
    )
      return [];
    const mine = uses
      .filter((u) => u.keyHash === keyHash)
      .slice(0, MAX_KEY_USES);
    return [
      {
        keyHash,
        label,
        createdAt,
        lastUsedAt: int(k, "last_used_at"),
        deviceId,
        outings: flag(k, "outings"),
        uses: mine.map(({ at, via }) => ({ at, via })),
      },
    ];
  });
}

function signInsOf(tables: Item["tables"]) {
  return (tables.sign_ins ?? [])
    .flatMap(
      (
        s,
      ): Array<{
        at: number;
        by: "server" | "phone";
        ok: boolean;
        reason: string | null;
      }> => {
        const at = int(s, "at");
        const by = text(s, "by");
        return at !== null && (by === "server" || by === "phone")
          ? [{ at, by, ok: flag(s, "ok"), reason: text(s, "reason") }]
          : [];
      },
    )
    .sort((a, b) => b.at - a.at)
    .slice(0, MAX_SIGN_INS);
}

function phonesOf(tables: Item["tables"]) {
  return (tables.phones ?? []).flatMap((p) => {
    const deviceId = text(p, "device_id");
    const addedAt = int(p, "added_at");
    return deviceId !== null && addedAt !== null
      ? [{ deviceId, addedAt, seenAt: int(p, "seen_at") ?? addedAt }]
      : [];
  });
}

async function deviceByDeviceId(
  ctx: MutationCtx,
  deviceId: string,
): Promise<Doc<"devices"> | null> {
  return ctx.db
    .query("devices")
    .withIndex("by_deviceId", (q) => q.eq("deviceId", deviceId))
    .unique();
}

// ---- each kind --------------------------------------------------------------------------

async function importDevice(
  ctx: MutationCtx,
  item: Item,
  tally: Tally,
): Promise<void> {
  const row = item.tables.device?.[0];
  const deviceId = row && text(row, "id");
  const signKey = row && text(row, "sign_key");
  const sealKey = row && text(row, "seal_key");
  const platform = row && text(row, "platform");
  if (!row || !deviceId || !signKey || !sealKey || !platform)
    return count(tally, "devices.empty");
  // The id is the key's hash, so a row that doesn't match it would sign as someone else.
  const key = fromBase64Url(signKey);
  if (key === null || (await deviceIdFor(key)) !== deviceId)
    return count(tally, "devices.bad_key");
  if (await deviceByDeviceId(ctx, deviceId))
    return count(tally, "devices.exists");
  await ctx.db.insert("devices", {
    deviceId,
    signKey,
    sealKey,
    platform,
    label: text(row, "label"),
    fcmToken: text(row, "fcm_token") ?? "",
    createdAt: int(row, "created_at") ?? 0,
  });
  count(tally, "devices.inserted");
}

/** How the import treats an account it finds. */
type Mode = "insert" | "sync" | "merge";

async function importAccount(
  ctx: MutationCtx,
  item: Item,
  tally: Tally,
): Promise<void> {
  const row = item.tables.account?.[0];
  const regno = row && text(row, "id");
  if (!row || !regno || !isAccountId(regno))
    return count(tally, "accounts.empty");
  const old = accountFields(row);
  const ownerHash =
    (item.tables.owner?.[0] && text(item.tables.owner[0], "hash")) ?? null;
  const found = await accountByRegno(ctx, regno);
  let accountId: Id<"accounts">;
  let mode: Mode;
  if (!found) {
    accountId = await ctx.db.insert("accounts", { regno, ownerHash, ...old });
    mode = "insert";
  } else if (found.createdAt === old.createdAt) {
    if (found.updatedAt >= old.updatedAt) {
      await importV2Tokens(ctx, found._id, item.tables.tokens ?? [], tally);
      return count(tally, "accounts.current");
    }
    await ctx.db.patch(found._id, {
      ...old,
      ownerHash: ownerHash ?? found.ownerHash,
    });
    accountId = found._id;
    mode = "sync";
  } else {
    // Convex's own fields are newer; only the history and the rows it lacks come over.
    await ctx.db.patch(found._id, {
      createdAt: Math.min(found.createdAt, old.createdAt),
      updatedAt: Math.max(found.updatedAt, old.updatedAt),
      ownerHash: found.ownerHash ?? ownerHash,
    });
    accountId = found._id;
    mode = "merge";
  }
  count(
    tally,
    `accounts.${mode === "insert" ? "inserted" : mode === "sync" ? "updated" : "merged"}`,
  );
  await importV2Tokens(ctx, accountId, item.tables.tokens ?? [], tally);
  await importKeys(ctx, accountId, keysOf(item.tables), mode, tally);
  await importSignIns(ctx, accountId, signInsOf(item.tables), mode, tally);
  await importPhones(ctx, accountId, phonesOf(item.tables), mode, tally);
  // An address imported before its account waits under the owner hash.
  const owner = (await ctx.db.get(accountId))?.ownerHash;
  if (owner) await claimMailbox(ctx, accountId, owner);
}

/** Mirrors the edge's live v2 tokens (`tokens`: device id → `issued_at`) whatever the account's
 * mode: only the edge issues or revokes them, so a row it no longer holds goes (revoked, or
 * replaced by a newer token). A token already traded in here stays used. */
async function importV2Tokens(
  ctx: MutationCtx,
  accountId: Id<"accounts">,
  tokens: Row[],
  tally: Tally,
): Promise<void> {
  const live = tokens.flatMap((row) => {
    const deviceId = text(row, "device_id");
    const iat = int(row, "issued_at");
    return deviceId && iat !== null ? [{ deviceId, iat }] : [];
  });
  for (const { deviceId, iat } of live) {
    const found = await ctx.db
      .query("v2Tokens")
      .withIndex("by_device_iat", (q) =>
        q.eq("deviceId", deviceId).eq("iat", iat),
      )
      .unique();
    if (!found) {
      await ctx.db.insert("v2Tokens", {
        deviceId,
        iat,
        accountId,
        usedAt: null,
      });
      count(tally, "v2Tokens.inserted");
    } else {
      count(
        tally,
        found.accountId === accountId
          ? "v2Tokens.exists"
          : "v2Tokens.other_account",
      );
    }
  }
  const rows = await ctx.db
    .query("v2Tokens")
    .withIndex("by_account", (q) => q.eq("accountId", accountId))
    .collect();
  for (const row of rows) {
    if (
      row.usedAt !== null ||
      live.some((t) => t.deviceId === row.deviceId && t.iat === row.iat)
    )
      continue;
    await ctx.db.delete(row._id);
    count(tally, "v2Tokens.deleted");
  }
}

async function importKeys(
  ctx: MutationCtx,
  accountId: Id<"accounts">,
  keys: OldKey[],
  mode: Mode,
  tally: Tally,
): Promise<void> {
  for (const key of keys) {
    const found = await ctx.db
      .query("accessKeys")
      .withIndex("by_keyHash", (q) => q.eq("keyHash", key.keyHash))
      .unique();
    if (!found) {
      await ctx.db.insert("accessKeys", { ...key, accountId });
      count(tally, "accessKeys.inserted");
    } else if (found.accountId !== accountId) {
      count(tally, "accessKeys.other_account");
    } else if (mode === "sync") {
      await ctx.db.patch(found._id, key);
      count(tally, "accessKeys.updated");
    } else {
      count(tally, "accessKeys.exists");
    }
  }
  if (mode !== "sync") return;
  // Revoked on the edge since the last pass.
  const kept = new Set(keys.map((k) => k.keyHash));
  const rows = await ctx.db
    .query("accessKeys")
    .withIndex("by_account", (q) => q.eq("accountId", accountId))
    .collect();
  for (const row of rows.filter((r) => !kept.has(r.keyHash))) {
    await ctx.db.delete(row._id);
    count(tally, "accessKeys.deleted");
  }
}

async function importSignIns(
  ctx: MutationCtx,
  accountId: Id<"accounts">,
  signIns: ReturnType<typeof signInsOf>,
  mode: Mode,
  tally: Tally,
): Promise<void> {
  const rows = await ctx.db
    .query("signIns")
    .withIndex("by_account", (q) => q.eq("accountId", accountId))
    .collect();
  if (mode === "sync") for (const row of rows) await ctx.db.delete(row._id);
  const have = mode === "sync" ? [] : rows;
  for (const s of signIns) {
    if (
      have.some(
        (r) =>
          r.at === s.at &&
          r.by === s.by &&
          r.ok === s.ok &&
          r.reason === s.reason,
      )
    )
      continue;
    await ctx.db.insert("signIns", { accountId, ...s });
    count(tally, "signIns.inserted");
  }
}

async function importPhones(
  ctx: MutationCtx,
  accountId: Id<"accounts">,
  phones: ReturnType<typeof phonesOf>,
  mode: Mode,
  tally: Tally,
): Promise<void> {
  let linked = (await phoneRows(ctx, accountId)).length;
  for (const phone of phones) {
    const device = await deviceByDeviceId(ctx, phone.deviceId);
    if (!device) {
      // Its object held nothing, or its export failed; the phone links on its next `auth:token`.
      count(tally, "phones.no_device");
    } else if (device.accountId === accountId) {
      if (mode === "sync")
        await ctx.db.patch(device._id, {
          linkedAt: phone.addedAt,
          seenAt: phone.seenAt,
        });
      count(tally, "phones.linked_already");
    } else if (device.accountId !== undefined) {
      count(tally, "phones.other_account");
    } else if (device.linkedOnce) {
      // Signed out or removed on v3 since: the edge's older link doesn't bring it back.
      count(tally, "phones.left_on_v3");
    } else if (mode === "merge" && linked >= MAX_PHONES) {
      count(tally, "phones.full");
    } else {
      await ctx.db.patch(device._id, {
        accountId,
        linkedAt: phone.addedAt,
        seenAt: phone.seenAt,
      });
      linked += 1;
      count(tally, "phones.linked");
    }
  }
  if (mode !== "sync") return;
  // Removed on the edge since the last pass.
  const kept = new Set(phones.map((p) => p.deviceId));
  for (const row of (await phoneRows(ctx, accountId)).filter(
    (r) => !kept.has(r.deviceId),
  )) {
    await unlinkPhone(ctx, row);
    count(tally, "phones.unlinked");
  }
}

type Member = { deviceId: string; addedAt: number; seenAt: number };

function membersOf(row: Row): Member[] {
  try {
    const raw = JSON.parse(text(row, "members") ?? "[]") as unknown;
    if (!Array.isArray(raw)) return [];
    return raw.flatMap((m: Record<string, unknown>) =>
      typeof m?.deviceId === "string" && typeof m.addedAt === "number"
        ? [
            {
              deviceId: m.deviceId,
              addedAt: m.addedAt,
              seenAt: typeof m.seenAt === "number" ? m.seenAt : m.addedAt,
            },
          ]
        : [],
    );
  } catch {
    return [];
  }
}

async function importMailbox(
  ctx: MutationCtx,
  item: Item,
  tally: Tally,
): Promise<void> {
  const row = item.tables.mailbox?.[0];
  const addressId = row && text(row, "id");
  if (!row || !addressId) return count(tally, "mailboxes.empty");
  const ownerHash = text(row, "owner_hash");
  const account =
    (item.regno !== undefined ? await accountByRegno(ctx, item.regno) : null) ??
    (ownerHash !== null
      ? await ctx.db
          .query("accounts")
          .withIndex("by_ownerHash", (q) => q.eq("ownerHash", ownerHash))
          .first()
      : null);
  const kv = item.kv;
  const confirmation =
    typeof kv.gmail_confirm === "string" ? kv.gmail_confirm : null;
  const fields = {
    confirmation,
    confirmationAt:
      confirmation !== null && typeof kv.gmail_confirm_at === "number"
        ? kv.gmail_confirm_at
        : null,
    lastOtpAt: typeof kv.last_otp_at === "number" ? kv.last_otp_at : null,
    createdAt: int(row, "created_at") ?? 0,
    updatedAt: int(row, "updated_at") ?? 0,
  };
  if (!account) {
    // No account yet: it waits under its owner hash and moves to the account the student's proof
    // makes (claimMailbox), so the address their Gmail forwards to stays theirs.
    if (ownerHash === null) return count(tally, "mailboxes.no_owner");
    const waiting = await ctx.db
      .query("unclaimedMailboxes")
      .withIndex("by_addressId", (q) => q.eq("addressId", addressId))
      .first();
    if (waiting) {
      if (waiting.updatedAt < fields.updatedAt)
        await ctx.db.patch(waiting._id, fields);
      return count(tally, "mailboxes.unclaimed_kept");
    }
    if (await mailboxRow(ctx, addressId))
      return count(tally, "mailboxes.other_account");
    await ctx.db.insert("unclaimedMailboxes", {
      addressId,
      ownerHash,
      ...fields,
    });
    return count(tally, "mailboxes.unclaimed");
  }
  const found = await ctx.db
    .query("mailboxes")
    .withIndex("by_addressId", (q) => q.eq("addressId", addressId))
    .unique();
  let sync = false;
  if (!found) {
    // Gmail forwards to the old address, so it wins over one made on v3 before this import,
    // unless that one got an OTP since.
    const way = await giveWayTo(ctx, account._id, addressId, fields.lastOtpAt);
    if (way === "kept") return count(tally, "mailboxes.older_than_current");
    if (way === "replaced") count(tally, "mailboxes.replaced");
    await ctx.db.insert("mailboxes", {
      addressId,
      accountId: account._id,
      ...fields,
    });
    count(tally, "mailboxes.inserted");
  } else if (found.accountId !== account._id) {
    return count(tally, "mailboxes.other_account");
  } else if (
    found.createdAt === fields.createdAt &&
    found.updatedAt < fields.updatedAt
  ) {
    await ctx.db.patch(found._id, fields);
    count(tally, "mailboxes.updated");
    sync = true;
  } else {
    return count(tally, "mailboxes.current");
  }
  const members = membersOf(row);
  const rows = await ctx.db
    .query("mailboxDevices")
    .withIndex("by_address", (q) => q.eq("addressId", addressId))
    .collect();
  for (const member of members) {
    if (rows.some((r) => r.deviceId === member.deviceId)) continue;
    // Mail is sealed to the device's key, so a member without a device row could never open it.
    const device = await deviceByDeviceId(ctx, member.deviceId);
    if (!device) {
      count(tally, "mailboxDevices.no_device");
      continue;
    }
    // Another student's phone, or signed out or removed on v3 since: it must not get this one's
    // OTPs.
    if (device.accountId !== undefined ? device.accountId !== account._id : device.linkedOnce) {
      count(tally, "mailboxDevices.left_on_v3");
      continue;
    }
    await ctx.db.insert("mailboxDevices", {
      addressId,
      ...member,
      accountId: account._id,
    });
    count(tally, "mailboxDevices.inserted");
  }
  await capMembers(ctx, addressId);
  if (!sync) return;
  const kept = new Set(members.map((m) => m.deviceId));
  for (const r of rows.filter((r) => !kept.has(r.deviceId))) {
    await ctx.db.delete(r._id);
    count(tally, "mailboxDevices.deleted");
  }
}

async function importOAuth(
  ctx: MutationCtx,
  item: Item,
  tally: Tally,
): Promise<void> {
  for (const row of item.tables.clients ?? []) {
    const clientId = text(row, "id");
    const name = text(row, "name");
    if (!clientId || name === null) {
      count(tally, "oauthClients.empty");
      continue;
    }
    const found = await ctx.db
      .query("oauthClients")
      .withIndex("by_clientId", (q) => q.eq("clientId", clientId))
      .unique();
    if (found) {
      count(tally, "oauthClients.exists");
      continue;
    }
    let redirectUris: string[] = [];
    try {
      const raw = JSON.parse(text(row, "redirect_uris") ?? "[]") as unknown;
      if (Array.isArray(raw))
        redirectUris = raw.filter((u): u is string => typeof u === "string");
    } catch {
      count(tally, "oauthClients.bad_redirects");
    }
    const createdAt = int(row, "created_at") ?? 0;
    await ctx.db.insert("oauthClients", {
      clientId,
      name,
      redirectUris,
      createdAt,
      usedAt: int(row, "used_at") ?? createdAt,
    });
    const meta = await ctx.db.query("oauthMeta").first();
    if (meta) await ctx.db.patch(meta._id, { count: meta.count + 1 });
    count(tally, "oauthClients.inserted");
  }
}

/** Sets `sweepAt` on accounts written before it existed (imported ones included), so the sweep's
 * index finds them. Run once after deploying it; returns how many changed. */
export const setSweepAt = internalMutation({
  args: {},
  handler: async (ctx) => {
    let changed = 0;
    for (const row of await ctx.db.query("accounts").collect()) {
      const sweepAt = sweepAtOf(row);
      if (row.sweepAt === sweepAt) continue;
      await ctx.db.patch(row._id, { sweepAt });
      changed += 1;
    }
    return changed;
  },
});

/** Drops forwarding memberships (and their sealed mail) whose install is another student's phone,
 * or was signed out or removed on v3: the rule `importMailbox` keeps, for rows imported before it
 * did. Returns how many went. */
export const dropStrayMembers = internalMutation({
  args: {},
  handler: async (ctx) => {
    let dropped = 0;
    for (const row of await ctx.db.query("mailboxDevices").collect()) {
      const device = await deviceByDeviceId(ctx, row.deviceId);
      const stray = !device || (device.accountId !== undefined ? device.accountId !== row.accountId : device.linkedOnce === true);
      if (!stray) continue;
      const boxes = await ctx.db
        .query("otpInbox")
        .withIndex("by_device", (q) => q.eq("deviceId", row.deviceId))
        .collect();
      for (const box of boxes) if (box.addressId === row.addressId) await ctx.db.delete(box._id);
      await ctx.db.delete(row._id);
      dropped += 1;
    }
    return dropped;
  },
});

/** Imports one batch, in the order given. Devices go before accounts (which link them) and
 * accounts before mailboxes (which belong to them), so the script sends them so. */
export const importBatch = internalMutation({
  args: { items: v.array(exported) },
  handler: async (ctx, { items }) => {
    const tally: Tally = {};
    for (const item of items) {
      if (!item.found) {
        count(tally, `${item.kind}.nothing_stored`);
        continue;
      }
      switch (item.kind) {
        case "device":
          await importDevice(ctx, item, tally);
          break;
        case "account":
          await importAccount(ctx, item, tally);
          break;
        case "mailbox":
          await importMailbox(ctx, item, tally);
          break;
        case "oauth":
          await importOAuth(ctx, item, tally);
          break;
      }
    }
    return tally;
  },
});
