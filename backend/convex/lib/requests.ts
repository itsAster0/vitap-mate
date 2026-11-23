// `sessionRequests` rows as `RequestRecord`s, and the steps of a request that
// write (asking the phones), shared by sessions.ts and app.ts.

import type { Doc, Id } from "../_generated/dataModel";
import type { MutationCtx, QueryCtx } from "../_generated/server";
import { type AccountRecord, PHONE_UNREACHABLE, RequestRecord } from "./accountState";
import { notify } from "./notify";

export function requestOf(row: Doc<"sessionRequests">): RequestRecord {
  const { _id, _creationTime, requestId, accountId, ...fields } = row;
  return Object.assign(new RequestRecord(requestId, row.createdAt), fields);
}

export function requestFields(r: RequestRecord) {
  return {
    status: r.status,
    cookies: r.cookies,
    csrfToken: r.csrfToken,
    error: r.error,
    createdAt: r.createdAt,
    expiresAt: r.expiresAt,
    vaultDueAt: r.vaultDueAt,
    vaultStarted: r.vaultStarted,
    phoneAsked: r.phoneAsked,
    phoneError: r.phoneError,
    source: r.source,
    phones: r.phones,
  };
}

export async function requestRow(ctx: QueryCtx, requestId: string): Promise<Doc<"sessionRequests"> | null> {
  if (!requestId || requestId.length > 64) return null;
  return ctx.db
    .query("sessionRequests")
    .withIndex("by_requestId", (q) => q.eq("requestId", requestId))
    .unique();
}

export async function putRequest(ctx: MutationCtx, row: Doc<"sessionRequests">, r: RequestRecord): Promise<void> {
  await ctx.db.patch(row._id, requestFields(r));
}

export async function insertRequest(ctx: MutationCtx, accountId: Id<"accounts">, r: RequestRecord): Promise<void> {
  await ctx.db.insert("sessionRequests", { requestId: r.id, accountId, ...requestFields(r) });
}

/** The account's request still pending, if any: a new session call joins it. */
export async function pendingRequest(ctx: QueryCtx, accountId: Id<"accounts">, at: number) {
  const rows = await ctx.db
    .query("sessionRequests")
    .withIndex("by_account", (q) => q.eq("accountId", accountId))
    .collect();
  return rows.map((row) => ({ row, request: requestOf(row) })).find(({ request }) => request.pending(at)) ?? null;
}

/** Asks each of the request's phones: they see it in `sessions:pending`, and a push wakes a closed
 * app. The first to answer with `sessions:answer` wins. Null once one was asked, else why none
 * could be. */
export async function askPhone(ctx: MutationCtx, doc: AccountRecord, request: RequestRecord, at: number): Promise<string | null> {
  for (const device of request.phones) {
    const fields = { requestId: request.id, wantCredentials: doc.wantCredentials(at) ? "1" : "0" };
    if (await notify(ctx, device, "vtop_cookie_request", fields)) request.phoneAsked = true;
  }
  return request.phoneAsked ? null : PHONE_UNREACHABLE;
}
