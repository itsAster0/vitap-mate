import type { AuthConfig } from "convex/server";

// Trusts the backend's own ES256 access JWTs (lib/jwt.ts). `JWT_PUBLIC_JWK` is the JWKS as a
// data URI, so nothing is fetched to check a token.
export default {
  providers: [
    {
      type: "customJwt",
      applicationID: "vmate",
      issuer: process.env.JWT_ISSUER!,
      jwks: process.env.JWT_PUBLIC_JWK!,
      algorithm: "ES256",
    },
  ],
} satisfies AuthConfig;
