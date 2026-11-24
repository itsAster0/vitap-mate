// Validation helpers for client-supplied device fields (FCM token, seal key,
// platform and labels).

import { isPublicKey } from "./mailSeal";

export const MAX_FCM_TOKEN_LEN = 4096;
export const MAX_LABEL_LEN = 60;
export const MAX_PLATFORM_LEN = 20;

/** Ok with the value (or null when none was sent), or the error message. */
export type Checked = { ok: true; value: string | null } | { ok: false; message: string };

export function fcmToken(raw: string | undefined): Checked {
  if (raw === undefined) return { ok: true, value: null };
  const token = raw.trim();
  if (!token || token.length > MAX_FCM_TOKEN_LEN) return { ok: false, message: "fcmToken is not an FCM token." };
  return { ok: true, value: token };
}

/** An X25519 key (`sealKey`), stored without padding. */
export function sealKey(raw: string | undefined): Checked {
  if (raw === undefined) return { ok: true, value: null };
  const key = raw.trim().replace(/=+$/, "");
  return isPublicKey(key) ? { ok: true, value: key } : { ok: false, message: "sealKey must be a base64url X25519 key." };
}

/** `platform` as the app sends it ("android", "ios"). */
export function platform(raw: string | undefined): Checked {
  const value = raw?.trim().toLowerCase() ?? "";
  return /^[a-z0-9_-]+$/.test(value) && value.length <= MAX_PLATFORM_LEN
    ? { ok: true, value }
    : { ok: false, message: "platform is required (android, ios, ...)." };
}

/** A label as sent by the app (a device's or a key's): trimmed, short, or none. */
export function memberLabel(raw: string | undefined | null): string | null {
  const label = raw?.trim();
  return label ? [...label].slice(0, MAX_LABEL_LEN).join("") : null;
}
