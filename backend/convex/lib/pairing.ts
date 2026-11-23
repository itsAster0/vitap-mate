// Pairing codes the extension and the MCP authorize page show (edge/src/pair.ts).

export const PAIR_TTL_SECS = 300;
export const POLL_AFTER_MS = 2000;

/** Crockford base32, the alphabet the extension draws codes from. */
export const ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const CODE_LENGTH = 10;

/** A code as typed or scanned, without dashes or spaces and with the letters that look like
 * digits read as digits; null when it is not one. */
export function pairCode(input: string | undefined): string | null {
  const code = (input ?? "")
    .toUpperCase()
    .replace(/[\s-]/g, "")
    .replace(/O/g, "0")
    .replace(/[IL]/g, "1");
  if (code.length !== CODE_LENGTH || [...code].some((c) => !ALPHABET.includes(c))) return null;
  return code;
}
