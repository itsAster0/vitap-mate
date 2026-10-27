// What the edge and the engine say to each other, as JSON. Mirrors
// vtop-shared's `engine` module (the engine's side); a field renamed there
// must be renamed here. Every call carries `Authorization: Bearer <shared
// secret>`.

/** Engine: whose session this is, or whether a cached one still works. */
export const SESSION = "/engine/session";
/** Engine: seals saved credentials for an account. */
export const SEAL = "/engine/seal";
/** Engine: signs in with saved credentials; answers at once and reports to
 * the edge's `LOGIN_DONE` when finished. */
export const LOGIN = "/engine/login";
/** Engine: the mailbox owner hash of a live session (mail-otp routes). */
export const MAIL_OWNER = "/internal/mail-otp/owner";
/** Edge: the outcome of a `LOGIN`. */
export const LOGIN_DONE = "/internal/accounts/login-done";
/** Edge: waits for the next forwarded OTP of a student, sealed to a key. */
export const OTP_AWAIT = "/internal/mail-otp/await";

/** Either fresh cookies from a client, or an account's cached session. */
export interface SessionCheck {
  cookies?: string;
  sealedSession?: string;
  /** The account a sealed session belongs to (its seal is bound to it). */
  account?: string;
}

export type SessionStatus = "valid" | "expired" | "unknown";

export interface SessionChecked {
  status: SessionStatus;
  /** Whose session it is, when valid. */
  registrationNumber?: string;
  /** The `Cookie` header, when valid, or unknown for a cached one. */
  cookies?: string;
  /** The session sealed for caching, when valid. */
  sealedSession?: string;
  /** The student's mailbox owner hash, when valid. */
  ownerHash?: string;
}

export interface SealVault {
  account: string;
  username: string;
  password: string;
  /** Gmail access, so the engine can read the OTP itself when the student
   * has no forwarding address. */
  gmail?: GmailGrant;
}

/** A Gmail refresh token and the OAuth client it was issued to: the app's
 * own on phones (no secret), the student's own on desktop. */
export interface GmailGrant {
  refreshToken: string;
  clientId: string;
  clientSecret?: string;
  /** Trash the OTP email once read; otherwise it is only marked read. */
  deleteAfterReading?: boolean;
}

export interface SealedVault {
  sealedVault: string;
}

export interface LoginStart {
  account: string;
  sealedVault: string;
  requestId: string;
}

export type LoginOutcome =
  | { status: "ok"; cookies: string; sealedSession: string }
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

export interface LoginDone {
  account: string;
  requestId: string;
  outcome: LoginOutcome;
}

export interface OtpAwait {
  ownerHash: string;
  /** An X25519 public key made for this one sign-in. */
  publicKey: string;
  /** Only mail sent at or after this (unix seconds) counts. */
  since: number;
  waitSecs: number;
}

export interface OtpAwaited {
  /** The mail payload sealed to `publicKey`; null when nothing came. */
  sealed: string | null;
}

/** Account ids are upper-cased registration numbers, as the engine makes
 * them (vtop-shared's `account_id`). */
export function accountId(registrationNumber: string): string {
  return registrationNumber.trim().toUpperCase();
}
