declare const Bun: { write(path: string, data: string): Promise<number> };

// Writes fresh TEST secrets for the vmate3 deployment to .env.test.local (gitignored) and prints
// only their names. Never use these for prod.
const rand = (n: number) => Array.from(crypto.getRandomValues(new Uint8Array(n)), (b) => b.toString(16).padStart(2, "0")).join("");
const pair = (await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, ["sign", "verify"])) as CryptoKeyPair;
const kid = rand(8);
const priv = { ...(await crypto.subtle.exportKey("jwk", pair.privateKey)), kid };
const { kty, crv, x, y } = await crypto.subtle.exportKey("jwk", pair.publicKey);
const jwks = JSON.stringify({ keys: [{ kty, crv, x, y, kid, alg: "ES256", use: "sig" }] });
const values: Record<string, string> = {
  JWT_ISSUER: "https://vmate3.aster0.dev",
  JWT_PRIVATE_JWK: JSON.stringify(priv),
  JWT_PUBLIC_JWK: `data:text/plain;charset=utf-8;base64,${btoa(jwks)}`,
  INTERNAL_TOKEN: rand(32),
  MAIL_TOKEN: rand(32),
  GATEWAY_TOKEN: rand(32),
  ACCOUNT_TOKEN_SECRET: rand(32),
  ENGINE_URL: "http://100.126.222.47:8090",
  PUBLIC_URL: "https://vmate3.aster0.dev",
  MAIL_DOMAIN: "vmate.aster0.dev",
};
const path = new URL("../.env.test.local", import.meta.url).pathname;
await Bun.write(path, Object.entries(values).map(([k, v]) => `${k}='${v}'`).join("\n") + "\n");
console.log(`wrote ${Object.keys(values).join(", ")}`);
