// JSON contract between the Convex backend and the Rust engine. Mirrors
// vtop-shared's `engine` module; a field renamed there must be renamed here.
// Every call carries `Authorization: Bearer <INTERNAL_TOKEN>`.

/** Engine: whose session this is, or whether a cached one still works. */
export const SESSION = "/engine/session";
/** Engine: seals saved credentials for an account. */
export const SEAL = "/engine/seal";
/** Engine: one step of a sign-in with saved credentials (see `logins.ts`). */
export const LOGIN = "/engine/login";
/** Engine: evicts the MCP server's in-memory session and key caches for the given key hashes. */
export const MCP_EVICT = "/engine/mcp/evict";

/** Either fresh cookies from a client, or an account's cached session. */
export type SessionCheck = {
  cookies?: string;
  sealedSession?: string;
  /** The account a sealed session belongs to (its seal is bound to it). */
  account?: string;
  /** Whether to seal and return `sealedSession`; omitted defaults to `true`. */
  seal?: boolean;
}

export type McpEvict = {
  keyHashes: string[];
}

export type SessionStatus = "valid" | "expired" | "unknown";

export type SessionChecked = {
  status: SessionStatus;
  /** Whose session it is, when valid. */
  registrationNumber?: string;
  /** The `Cookie` header, when valid, or unknown for a cached one. */
  cookies?: string;
  /** The CSRF token that goes with `cookies`, when the session has one (a
   * cached session does; bare cookies do not). */
  csrfToken?: string;
  /** The session sealed for caching, when valid. */
  sealedSession?: string;
  /** The student's mailbox owner hash, when valid. */
  ownerHash?: string;
}

export type SealVault = {
  account: string;
  username: string;
  password: string;
  /** Gmail access, so the engine can read the OTP itself when the student
   * has no forwarding address. */
  gmail?: GmailGrant;
  /** The OTP auto-fill picked on the phone; Gmail skips forwarding. */
  otp?: OtpMethod;
}

export type OtpMethod = "forwarding" | "gmail";

/** A Gmail refresh token and the OAuth client it was issued to: the app's
 * own on phones (no secret), the student's own on desktop. */
export type GmailGrant = {
  refreshToken: string;
  clientId: string;
  clientSecret?: string;
  /** Trash the OTP email once read; otherwise it is only marked read. */
  deleteAfterReading?: boolean;
}

export type SealedVault = {
  sealedVault: string;
}

export type LoginOutcome =
  | { status: "ok"; cookies: string; sealedSession: string; csrfToken?: string }
  /** VTOP rejected the saved username or password. */
  | { status: "invalid_credentials" }
  /** The saved credentials belong to someone else, or no longer open. */
  | { status: "vault_unusable" }
  /** No forwarding address and no Gmail access, so the OTP cannot be read. */
  | { status: "forwarding_required" }
  /** Google refused the saved Gmail access; `sealedVault` is the same
   * credentials without it, to keep in place of the old ones. */
  | { status: "gmail_revoked"; sealedVault: string }
  /** Anything else; the credentials may still be good. `code` is optional for older engines. */
  | { status: "failed"; code?: string; reason: string };


