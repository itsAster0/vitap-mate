/* eslint-disable */
/**
 * Generated `api` utility.
 *
 * THIS CODE IS AUTOMATICALLY GENERATED.
 *
 * To regenerate, run `npx convex dev`.
 * @module
 */

import type * as account from "../account.js";
import type * as app from "../app.js";
import type * as auth from "../auth.js";
import type * as crons from "../crons.js";
import type * as devices from "../devices.js";
import type * as extension from "../extension.js";
import type * as guides from "../guides.js";
import type * as http from "../http.js";
import type * as identity from "../identity.js";
import type * as keys from "../keys.js";
import type * as lib_accountState from "../lib/accountState.js";
import type * as lib_accounts from "../lib/accounts.js";
import type * as lib_base64 from "../lib/base64.js";
import type * as lib_checks from "../lib/checks.js";
import type * as lib_credentials from "../lib/credentials.js";
import type * as lib_crypto from "../lib/crypto.js";
import type * as lib_engine from "../lib/engine.js";
import type * as lib_engineApi from "../lib/engineApi.js";
import type * as lib_errors from "../lib/errors.js";
import type * as lib_guideFiles from "../lib/guideFiles.js";
import type * as lib_httpKit from "../lib/httpKit.js";
import type * as lib_identity from "../lib/identity.js";
import type * as lib_jwt from "../lib/jwt.js";
import type * as lib_mailSeal from "../lib/mailSeal.js";
import type * as lib_mailboxes from "../lib/mailboxes.js";
import type * as lib_notify from "../lib/notify.js";
import type * as lib_oauthPage from "../lib/oauthPage.js";
import type * as lib_pairing from "../lib/pairing.js";
import type * as lib_qr from "../lib/qr.js";
import type * as lib_rateLimits from "../lib/rateLimits.js";
import type * as lib_requests from "../lib/requests.js";
import type * as lib_sha256 from "../lib/sha256.js";
import type * as lib_signed from "../lib/signed.js";
import type * as lib_util from "../lib/util.js";
import type * as lib_vm1 from "../lib/vm1.js";
import type * as logins from "../logins.js";
import type * as mail from "../mail.js";
import type * as migrate from "../migrate.js";
import type * as oauth from "../oauth.js";
import type * as pair from "../pair.js";
import type * as proxy from "../proxy.js";
import type * as push from "../push.js";
import type * as screen from "../screen.js";
import type * as sessions from "../sessions.js";
import type * as sweeps from "../sweeps.js";
import type * as udp from "../udp.js";
import type * as vtop from "../vtop.js";

import type {
  ApiFromModules,
  FilterApi,
  FunctionReference,
} from "convex/server";

declare const fullApi: ApiFromModules<{
  account: typeof account;
  app: typeof app;
  auth: typeof auth;
  crons: typeof crons;
  devices: typeof devices;
  extension: typeof extension;
  guides: typeof guides;
  http: typeof http;
  identity: typeof identity;
  keys: typeof keys;
  "lib/accountState": typeof lib_accountState;
  "lib/accounts": typeof lib_accounts;
  "lib/base64": typeof lib_base64;
  "lib/checks": typeof lib_checks;
  "lib/credentials": typeof lib_credentials;
  "lib/crypto": typeof lib_crypto;
  "lib/engine": typeof lib_engine;
  "lib/engineApi": typeof lib_engineApi;
  "lib/errors": typeof lib_errors;
  "lib/guideFiles": typeof lib_guideFiles;
  "lib/httpKit": typeof lib_httpKit;
  "lib/identity": typeof lib_identity;
  "lib/jwt": typeof lib_jwt;
  "lib/mailSeal": typeof lib_mailSeal;
  "lib/mailboxes": typeof lib_mailboxes;
  "lib/notify": typeof lib_notify;
  "lib/oauthPage": typeof lib_oauthPage;
  "lib/pairing": typeof lib_pairing;
  "lib/qr": typeof lib_qr;
  "lib/rateLimits": typeof lib_rateLimits;
  "lib/requests": typeof lib_requests;
  "lib/sha256": typeof lib_sha256;
  "lib/signed": typeof lib_signed;
  "lib/util": typeof lib_util;
  "lib/vm1": typeof lib_vm1;
  logins: typeof logins;
  mail: typeof mail;
  migrate: typeof migrate;
  oauth: typeof oauth;
  pair: typeof pair;
  proxy: typeof proxy;
  push: typeof push;
  screen: typeof screen;
  sessions: typeof sessions;
  sweeps: typeof sweeps;
  udp: typeof udp;
  vtop: typeof vtop;
}>;

/**
 * A utility for referencing Convex functions in your app's public API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = api.myModule.myFunction;
 * ```
 */
export declare const api: FilterApi<
  typeof fullApi,
  FunctionReference<any, "public">
>;

/**
 * A utility for referencing Convex functions in your app's internal API.
 *
 * Usage:
 * ```js
 * const myFunctionReference = internal.myModule.myFunction;
 * ```
 */
export declare const internal: FilterApi<
  typeof fullApi,
  FunctionReference<any, "internal">
>;

export declare const components: {
  rateLimiter: import("@convex-dev/rate-limiter/_generated/component.js").ComponentApi<"rateLimiter">;
};
