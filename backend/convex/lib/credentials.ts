// Credentials a phone sends to be kept sealed (edge/src/account-db.ts).

import type { GmailGrant, OtpMethod } from "./engineApi";
import { str } from "./util";

export function usableText(raw: unknown): string | null {
  return typeof raw === "string" ? raw.trim() || null : null;
}

export type Credentials = {
  username: string;
  password: string;
  gmail: GmailGrant | null;
  /** The OTP auto-fill picked on the phone; null from older phones. */
  otp: OtpMethod | null;
};

export function credentialsOf(raw: unknown): Credentials | null {
  if (raw === null || typeof raw !== "object") return null;
  const { username, password, gmail, otp } = raw as Record<string, unknown>;
  return {
    username: str(username) ?? "",
    password: str(password) ?? "",
    gmail: gmailGrantOf(gmail),
    otp: otp === "gmail" || otp === "forwarding" ? otp : null,
  };
}

export function gmailGrantOf(raw: unknown): GmailGrant | null {
  if (raw === null || typeof raw !== "object") return null;
  const grant = raw as Record<string, unknown>;
  const refreshToken = usableText(grant.refreshToken);
  const clientId = usableText(grant.clientId);
  if (refreshToken === null || clientId === null) return null;
  const clientSecret = usableText(grant.clientSecret);
  return {
    refreshToken,
    clientId,
    ...(clientSecret !== null ? { clientSecret } : {}),
    deleteAfterReading: grant.deleteAfterReading === true,
  };
}
