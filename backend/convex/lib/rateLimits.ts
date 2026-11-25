// Every rate limit, kept by the rate-limiter component.

import { HOUR, MINUTE, RateLimiter, SECOND } from "@convex-dev/rate-limiter";
import { components } from "../_generated/api";

export const rateLimits = new RateLimiter(components.rateLimiter, {
  /** Proofs an install starts (by device id): each holds a VTOP connection on the engine. One
   * every 5 s, 20 an hour. */
  proofGap: { kind: "token bucket", rate: 1, period: 5 * SECOND, capacity: 1 },
  proofHourly: { kind: "fixed window", rate: 20, period: HOUR },
  /** Proofs the engine runs (counted when it takes a ticket), so its 4 at once aren't swamped.
   * Counted there rather than at `proveStart`: a ticket costs nothing until a proof uses it, so
   * fresh device keys asking for tickets can't use this up for everyone. */
  proofGlobal: { kind: "token bucket", rate: 60, period: MINUTE, capacity: 20 },
  /** Tickets from all installs together: only so a flood of fresh keys can't fill the table. */
  proofTickets: { kind: "token bucket", rate: 600, period: MINUTE, capacity: 200 },
  /** Server-route sign-ins, before any account: per VTOP username and in total, so a flood of
   * new device keys can't get students locked out or VTOP blocking the engine. */
  loginUser: { kind: "token bucket", rate: 10, period: 10 * MINUTE, capacity: 5 },
  loginGlobal: { kind: "token bucket", rate: 120, period: MINUTE, capacity: 40 },
  /** OAuth clients registered: per IP so one caller can't crowd out real apps, with a higher
   * global ceiling. */
  oauthRegisterIp: { kind: "token bucket", rate: 10, period: HOUR, capacity: 10 },
  oauthRegister: { kind: "token bucket", rate: 200, period: HOUR, capacity: 60 },
  /** OAuth `/authorize` page opens, per IP and globally. */
  oauthAuthorizeIp: { kind: "token bucket", rate: 30, period: HOUR, capacity: 15 },
  oauthAuthorize: { kind: "token bucket", rate: 300, period: HOUR, capacity: 100 },
  /** OAuth `/authorize/poll` (page polls every 2 s), per IP and globally. */
  oauthPollIp: { kind: "token bucket", rate: 60, period: MINUTE, capacity: 30 },
  oauthPoll: { kind: "token bucket", rate: 1200, period: MINUTE, capacity: 300 },
  /** OAuth `/token` exchanges, per IP and globally. */
  oauthTokenIp: { kind: "token bucket", rate: 30, period: MINUTE, capacity: 15 },
  oauthToken: { kind: "token bucket", rate: 300, period: MINUTE, capacity: 100 },
  /** Extension `/v1/pair` polling (polls every 2 s), per IP and globally. */
  pairIp: { kind: "token bucket", rate: 60, period: MINUTE, capacity: 30 },
  pairGlobal: { kind: "token bucket", rate: 1200, period: MINUTE, capacity: 300 },
  /** Session calls from an account's keys (extension, MCP), by account id. */
  sessionCall: { kind: "fixed window", rate: 60, period: MINUTE },
  /** Key JWTs for the extension's socket, by account: a client stuck reconnecting can't keep the
   * backend signing. */
  keyToken: { kind: "token bucket", rate: 30, period: MINUTE, capacity: 10 },
  /** Demo `auth:token` grants: per install and globally so fresh device keys can't fill `devices`
   * and `refreshTokens`. */
  tokenDemoDevice: { kind: "token bucket", rate: 10, period: HOUR, capacity: 5 },
  tokenDemoGlobal: { kind: "token bucket", rate: 120, period: MINUTE, capacity: 40 },
  /** Refresh `auth:token` rotations, per install, so a looping client can't hammer JWT signing. */
  tokenRefresh: { kind: "token bucket", rate: 30, period: MINUTE, capacity: 15 },
  /** Proof ticket `auth:token` exchanges, per install. */
  tokenTicket: { kind: "token bucket", rate: 30, period: MINUTE, capacity: 15 },
  /** Legacy v2 account token `auth:token` trade-ins, per install. */
  tokenV2: { kind: "token bucket", rate: 30, period: MINUTE, capacity: 15 },
});
