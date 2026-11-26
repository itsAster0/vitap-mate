// Every table. Times are unix seconds, as the edge kept them, so answers
// carry them unchanged. Secrets (refresh tokens, access keys, pairing and
// OAuth codes) are stored as SHA-256 hex only.
//
// One fact, one place: `regno` lives only on `accounts`. Every row a student
// owns links to it by `accountId` with a `by_account` index, so one query per
// table shows a student's whole footprint. A phone is a `devices` row linked
// to the account, a key an `accessKeys` row, a sign-in a `signIns` row.

import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

const nullable = <T extends ReturnType<typeof v.string> | ReturnType<typeof v.number>>(t: T) => v.union(t, v.null());

export const settings = v.object({
  alwaysUseVault: v.boolean(),
  phoneWaitSecs: v.number(),
  vaultTtlSecs: nullable(v.number()),
  /** Whether connected apps may cache a VTOP session on the server; omitted on legacy rows (defaults to `sealedVault !== null`). */
  cacheSession: v.optional(v.boolean()),
});

export const loginPhase = v.union(
  /** First engine step is running; an OTP that arrives meanwhile waits in `nextOtp`. */
  v.object({ kind: v.literal("starting"), nextOtp: nullable(v.string()) }),
  /** Waiting for VTOP's OTP mail with the engine's sealed half-done state. */
  v.object({ kind: v.literal("waiting_otp"), sealedState: v.string() }),
  /** Submitting a mailed OTP on the engine; a newer OTP arriving meanwhile waits in `nextOtp`. */
  v.object({ kind: v.literal("checking_otp"), sealedState: v.string(), nextOtp: nullable(v.string()) }),
);

/** The key that asked for a session, and from where (the phone's approve sheet). */
export const requestAsker = v.object({
  keyLabel: v.string(),
  via: v.union(v.literal("extension"), v.literal("mcp")),
  /** The address it asked from (an AI app's as our engine saw it on `/mcp`) and whether the key
   * used it before; null when unknown. */
  ip: v.union(v.object({ address: v.string(), seen: v.boolean() }), v.null()),
});

export default defineSchema({
  /** One student, by registration number (one row per `regno`). */
  accounts: defineTable({
    regno: v.string(),
    /** The student's mailbox owner hash, from the proof. */
    ownerHash: nullable(v.string()),
    settings,
    semesterId: nullable(v.string()),
    sealedVault: nullable(v.string()),
    vaultSavedAt: nullable(v.number()),
    vaultGmail: v.boolean(),
    sealedSession: nullable(v.string()),
    sessionSavedAt: nullable(v.number()),
    sessionSource: nullable(v.string()),
    credentialsRejectedAt: nullable(v.number()),
    /** When the cached session or the saved credentials next expire, so the sweep reads only the
     * rows due; unset when neither is kept. */
    sweepAt: v.optional(v.number()),
    createdAt: v.number(),
    updatedAt: v.number(),
  })
    .index("by_regno", ["regno"])
    .index("by_ownerHash", ["ownerHash"])
    .index("by_sweepAt", ["sweepAt"]),

  /** An app install, by the id its Ed25519 key gives. With `accountId` it is one of that
   * account's phones (`linkedAt`, `seenAt`); without, it is unlinked or the demo's. */
  devices: defineTable({
    deviceId: v.string(),
    signKey: v.string(),
    sealKey: v.string(),
    platform: v.string(),
    label: nullable(v.string()),
    /** Unset when there is none, or Google said it is gone (legacy rows may hold `""`). */
    fcmToken: v.optional(v.string()),
    createdAt: v.number(),
    accountId: v.optional(v.id("accounts")),
    linkedAt: v.optional(v.number()),
    seenAt: v.optional(v.number()),
    /** Linked here by its own `auth:token` at least once: unlinked after that, it signed out or
     * was removed on v3, and a backfill pass must not put it back. */
    linkedOnce: v.optional(v.boolean()),
    /** When an inactive demo install is due to be swept; unset on linked accounts. */
    demoExpiresAt: v.optional(v.number()),
  })
    .index("by_deviceId", ["deviceId"])
    .index("by_account", ["accountId"])
    .index("by_demoExpiresAt", ["demoExpiresAt"]),

  /** One row per refresh token issued, until its chain goes. `usedAt` set: replaced, so showing
   * it again means it leaked. No account: the demo's. */
  refreshTokens: defineTable({
    hash: v.string(),
    deviceId: v.string(),
    chainId: v.string(),
    accountId: v.optional(v.id("accounts")),
    usedAt: nullable(v.number()),
    createdAt: v.number(),
  })
    .index("by_hash", ["hash"])
    .index("by_chain", ["chainId"])
    .index("by_device", ["deviceId"])
    .index("by_account", ["accountId"])
    .index("by_account_createdAt", ["accountId", "createdAt"]),

  /** TLSNotary tickets: `open` from `auth:proveStart`, `taken` once the engine starts the proof,
   * then `proved` (with the registration number the proof showed, until `auth:token` uses the
   * ticket up) or `failed`. */
  proofTickets: defineTable({
    ticket: v.string(),
    deviceId: v.string(),
    status: v.union(v.literal("open"), v.literal("taken"), v.literal("proved"), v.literal("failed")),
    expiresAt: v.number(),
    regno: v.optional(v.string()),
    ownerHash: v.optional(v.string()),
    code: v.optional(v.string()),
  })
    .index("by_ticket", ["ticket"])
    .index("by_device", ["deviceId"])
    .index("by_expiresAt", ["expiresAt"]),

  /** The v2 account tokens the edge still held live (one per install, by its `iat`), imported by
   * the backfill. Only these trade in, each once (`usedAt`): a token the edge revoked or replaced
   * has no row. No secrets: the token itself is checked with `ACCOUNT_TOKEN_SECRET`. */
  v2Tokens: defineTable({ deviceId: v.string(), iat: v.number(), accountId: v.id("accounts"), usedAt: nullable(v.number()) })
    .index("by_device_iat", ["deviceId", "iat"])
    .index("by_account", ["accountId"]),

  /** Access keys for the extension and MCP clients. */
  accessKeys: defineTable({
    keyHash: v.string(),
    accountId: v.id("accounts"),
    label: v.string(),
    createdAt: v.number(),
    lastUsedAt: nullable(v.number()),
    /** The install that made it: the only phone its session requests go to. */
    deviceId: v.string(),
    outings: v.boolean(),
    /** Its latest uses, newest first, at most 10. */
    uses: v.array(v.object({ at: v.number(), via: v.union(v.literal("extension"), v.literal("mcp")) })),
    /** Who it is for: the extension (paired with its code) or an AI app. Absent on keys made before
     * it was kept, read from the label then (`keyOf`). */
    kind: v.optional(v.union(v.literal("browser"), v.literal("mcp"))),
    /** The key's latest addresses (the extension's, or the AI app's as our engine saw it on
     * `/mcp`), newest first, at most `MAX_KEY_IPS`: a request from one not listed shows as a new
     * IP on the phone. */
    ips: v.optional(v.array(v.string())),
  })
    .index("by_keyHash", ["keyHash"])
    .index("by_account", ["accountId"]),

  /** The account's latest VTOP sign-ins, at most 10. */
  signIns: defineTable({
    accountId: v.id("accounts"),
    at: v.number(),
    by: v.union(v.literal("server"), v.literal("phone")),
    ok: v.boolean(),
    reason: nullable(v.string()),
  }).index("by_account", ["accountId", "at"]),

  /** Session requests of key holders (lib/accountState.ts `RequestRecord`). */
  sessionRequests: defineTable({
    requestId: v.string(),
    accountId: v.id("accounts"),
    status: v.union(v.literal("pending"), v.literal("success"), v.literal("error")),
    cookies: nullable(v.string()),
    csrfToken: nullable(v.string()),
    errorCode: v.optional(nullable(v.string())),
    error: nullable(v.string()),
    createdAt: v.number(),
    expiresAt: v.number(),
    vaultDueAt: nullable(v.number()),
    vaultStarted: v.boolean(),
    phoneAsked: v.boolean(),
    /** Whether at least one phone had an FCM push token when asked. */
    pushSent: v.optional(v.boolean()),
    phoneErrorCode: v.optional(nullable(v.string())),
    phoneError: nullable(v.string()),
    source: nullable(v.string()),
    phones: v.array(v.string()),
    /** Set by the scheduled expiry mutation so subscribed live queries re-run at `expiresAt`. */
    expired: v.optional(v.boolean()),
    /** Who asked (lib/accountState.ts `RequestAsker`); absent on rows from before it and on `app:login`'s. */
    asker: v.optional(requestAsker),
  })
    .index("by_requestId", ["requestId"])
    .index("by_account", ["accountId"])
    .index("by_createdAt", ["createdAt"]),

  /** A key made with an extension's pairing code, until it collects it. */
  pairings: defineTable({ codeHash: v.string(), key: v.string(), expiresAt: v.number(), accountId: v.id("accounts") })
    .index("by_codeHash", ["codeHash"])
    .index("by_account", ["accountId"])
    .index("by_expiresAt", ["expiresAt"]),

  oauthClients: defineTable({
    clientId: v.string(),
    name: v.string(),
    redirectUris: v.array(v.string()),
    createdAt: v.number(),
    usedAt: v.number(),
  })
    .index("by_clientId", ["clientId"])
    .index("by_usedAt", ["usedAt"]),

  /** Single-row count of `oauthClients`, so `/register` caps clients without scanning every row. */
  oauthMeta: defineTable({
    count: v.number(),
  }),

  /** Authorize pages waiting for the app; `accessKey` and `accountId` once approved. */
  oauthRequests: defineTable({
    codeHash: v.string(),
    pollHash: v.string(),
    clientId: v.string(),
    redirectUri: v.string(),
    challenge: v.string(),
    state: nullable(v.string()),
    expiresAt: v.number(),
    accessKey: nullable(v.string()),
    accountId: v.optional(v.id("accounts")),
    /** Set at `expiresAt` so live `pair:info` queries re-run while `keys:store` still reports `pair_expired`. */
    expired: v.optional(v.boolean()),
  })
    .index("by_codeHash", ["codeHash"])
    .index("by_pollHash", ["pollHash"])
    .index("by_account", ["accountId"])
    .index("by_expiresAt", ["expiresAt"]),

  /** One-time codes for `/token`. */
  oauthCodes: defineTable({
    codeHash: v.string(),
    clientId: v.string(),
    redirectUri: v.string(),
    challenge: v.string(),
    accessKey: v.string(),
    expiresAt: v.number(),
    accountId: v.id("accounts"),
  })
    .index("by_codeHash", ["codeHash"])
    .index("by_account", ["accountId"])
    .index("by_expiresAt", ["expiresAt"]),

  /** A forwarding address `otp+<addressId>@`, the student's from the start. Gmail's latest
   * forwarding confirmation is kept readable for a day (until an OTP comes) so a phone that
   * joins during setup can still be shown it. */
  mailboxes: defineTable({
    addressId: v.string(),
    accountId: v.id("accounts"),
    confirmation: nullable(v.string()),
    confirmationAt: nullable(v.number()),
    /** Unix seconds of the latest `gmail_confirm` (`sentAt`), kept past `confirmation`'s 24h TTL
     * so phones joining later know Gmail already confirmed this address. */
    confirmedAt: v.optional(nullable(v.number())),
    lastOtpAt: nullable(v.number()),
    createdAt: v.number(),
    updatedAt: v.number(),
  })
    .index("by_addressId", ["addressId"])
    .index("by_account", ["accountId"])
    .index("by_confirmationAt", ["confirmationAt"]),

  // An old address the backfill found no account for. It waits under its owner hash, which the
  // account proof also gives, and moves into `mailboxes` when that student's account appears, so
  // the Gmail forwarding they set up keeps working.
  unclaimedMailboxes: defineTable({
    addressId: v.string(),
    ownerHash: v.string(),
    confirmation: nullable(v.string()),
    confirmationAt: nullable(v.number()),
    confirmedAt: v.optional(nullable(v.number())),
    lastOtpAt: nullable(v.number()),
    createdAt: v.number(),
    updatedAt: v.number(),
  })
    .index("by_addressId", ["addressId"])
    .index("by_ownerHash", ["ownerHash"]),

  mailboxDevices: defineTable({ addressId: v.string(), deviceId: v.string(), addedAt: v.number(), seenAt: v.number(), accountId: v.id("accounts") })
    .index("by_address", ["addressId"])
    .index("by_device", ["deviceId"])
    .index("by_account", ["accountId"]),

  /** Mail sealed to one install's `sealKey`: the latest OTP and Gmail confirmation. */
  otpInbox: defineTable({
    addressId: v.string(),
    deviceId: v.string(),
    kind: v.union(v.literal("otp"), v.literal("gmail_confirm")),
    sealed: v.string(),
    receivedAt: v.number(),
    expiresAt: v.optional(v.number()),
    accountId: v.id("accounts"),
  })
    .index("by_address_device_kind", ["addressId", "deviceId", "kind"])
    .index("by_device", ["deviceId"])
    .index("by_account", ["accountId"])
    .index("by_expiresAt", ["expiresAt"]),

  /** Server sign-ins with saved credentials waiting for VTOP's OTP mail (logins.ts). */
  logins: defineTable({
    accountId: v.id("accounts"),
    /** The session request it answers. */
    requestId: v.string(),
    /** Only mail sent from then on (unix seconds) can hold the code. */
    since: v.number(),
    /** Discriminated phase; optional while pre-phase in-flight rows drain during deploy. */
    phase: v.optional(loginPhase),
    /** Legacy pre-phase fields; kept optional for in-flight rows during rollout and cleared on write. */
    sealedState: v.optional(nullable(v.string())),
    busy: v.optional(v.boolean()),
    next: v.optional(nullable(v.string())),
    expiresAt: v.number(),
  })
    .index("by_account", ["accountId"])
    .index("by_expiresAt", ["expiresAt"]),

  /** The UDP gateway's latest report (one row). */
  udpGateway: defineTable({
    host: v.string(),
    ports: v.array(v.number()),
    keys: v.array(v.object({ id: v.string(), publicKey: v.string(), from: v.number(), until: v.number() })),
  }),
});
