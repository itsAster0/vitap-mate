// What the backend and the engine say to each other, as JSON (copied from
// edge/src/shared/engine-api.ts; the engine did not change). Mirrors
// vtop-shared's `engine` module (the engine's side); a field renamed there
// must be renamed here. Every call carries `Authorization: Bearer
// <INTERNAL_TOKEN>`.

/** Engine: whose session this is, or whether a cached one still works. */
export const SESSION = "/engine/session";
/** Engine: seals saved credentials for an account. */
export const SEAL = "/engine/seal";
/** Engine: one step of a sign-in with saved credentials (see `logins.ts`). */
export const LOGIN = "/engine/login";
/** Engine: the mailbox owner hash of a live session (mail-otp routes). */
export const MAIL_OWNER = "/internal/mail-otp/owner";
/** Engine: verifies a phone's TLSNotary proof of its regno, a WebSocket the
 * edge opens as `GET ?device=<id>&challenge=<hex>` after it consumed that
 * device's challenge. Binary messages carry the proof both ways; the
 * engine's last message is one text frame, a `ProofVerdict`. */
export const PROOF_VERIFY = "/engine/proof";

/** Either fresh cookies from a client, or an account's cached session. */
export type SessionCheck = {
  cookies?: string;
  sealedSession?: string;
  /** The account a sealed session belongs to (its seal is bound to it). */
  account?: string;
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

/** How a `PROOF_VERIFY` ended. */
export type ProofVerdict =
  | { status: "ok"; registrationNumber: string; ownerHash: string }
  | { status: "failed"; reason: string };

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
  /** Anything else; the credentials may still be good. */
  | { status: "failed"; reason: string };


