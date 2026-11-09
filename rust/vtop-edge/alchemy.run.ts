// The server, deployed with Alchemy: `bun run deploy` here, or CI on a push to
// main (.github/workflows/deploy.yml), both as stage `prod`. Needs the variables in .env.example.
// State lives in the account's Alchemy state-store Worker.
//
// - Edge: the `vmate-edge` Worker on vmate.aster0.dev, its Durable Objects
//   and secrets (wrangler.toml is kept for `wrangler dev` and the tests).
// - Engine: Railway `engine-v2` in the `vitapmate` project, built from
//   ../Dockerfile.engine.
// - Gateway: the UDP gateway on the Oracle VPS (robin), built here for x86,
//   pushed to GHCR (public) and run over SSH as the `vitapmate-gateway`
//   container. GATEWAY_SSH is the Docker host, e.g. ssh://rocky@robin.
//
// Not here: the Email Routing rule otp@vmate.aster0.dev → vmate-edge (made in
// the dashboard; subdomains have no catch-all).

import * as Alchemy from "alchemy";
import { adopt } from "alchemy/AdoptPolicy";
import * as Cloudflare from "@alchemy.run/cloudflare";
import * as Docker from "@alchemy.run/docker";
import * as Railway from "@alchemy.run/railway";
import * as Config from "effect/Config";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";

const DOMAIN = "vmate.aster0.dev";
/** Railway's generated domain for `engine-v2`. */
const ENGINE_URL = "https://engine-v2-production.up.railway.app";

/** Shared by the edge, the engine and the gateway (`authorization: Bearer`). */
const internalToken = Config.Redacted("INTERNAL_TOKEN");

export const Engine = Effect.gen(function* () {
  const project = yield* Railway.Project("Vitapmate", { name: "vitapmate" }).pipe(adopt(true));
  return yield* Railway.Service("Engine", {
    project,
    name: "engine-v2",
    context: "..",
    dockerfilePath: "Dockerfile.engine",
    port: 8080,
    env: {
      EDGE_URL: `https://${DOMAIN}`,
      INTERNAL_TOKEN: yield* internalToken,
      VAULT_KEY: yield* Config.Redacted("VAULT_KEY"),
      RUST_LOG: "warn,vtop_bridge=info",
    },
  }).pipe(adopt(true));
});

export const Edge = Cloudflare.Worker("Edge", {
  name: "vmate-edge",
  main: "./src/index.ts",
  compatibility: { date: "2026-10-01" },
  domain: DOMAIN,
  workersDev: false,
  observability: {
    enabled: true,
    logs: { enabled: true, invocationLogs: true },
    traces: { enabled: true, headSamplingRate: 1 },
  },
  env: {
    DEVICE: Cloudflare.DurableObject("DEVICE", { className: "Device" }),
    MAILBOX: Cloudflare.DurableObject("MAILBOX", { className: "Mailbox" }),
    INDEX: Cloudflare.DurableObject("INDEX", { className: "Index" }),
    FCM_TOKEN: Cloudflare.DurableObject("FCM_TOKEN", { className: "FcmToken" }),
    ACCOUNT: Cloudflare.DurableObject("ACCOUNT", { className: "Account" }),
    // Addresses are otp+<id>@vmate.aster0.dev: one Email Routing rule, otp@vmate.aster0.dev → this Worker.
    MAIL_DOMAIN: DOMAIN,
    ENGINE_URL,
    FCM_URL: "https://fcm.googleapis.com",
    PUBLIC_URL: `https://${DOMAIN}`,
    ALLOWED_ORIGINS: "",
    FALLBACK_FORWARD: "",
    INTERNAL_TOKEN: internalToken,
    FIREBASE_CREDENTIALS_JSON: Config.Redacted("FIREBASE_CREDENTIALS_JSON"),
  },
}).pipe(adopt(true));

/** Where the gateway's files live on robin: its Noise keys (./data) and resolvers. */
const GATEWAY_DIR = "/home/rocky/vitapmate-gateway";

export const Gateway = Effect.gen(function* () {
  const robin = yield* Docker.Context("Robin", {
    name: "robin",
    docker: `host=${yield* Config.String("GATEWAY_SSH").pipe(Config.withDefault("ssh://rocky@robin"))}`,
  });
  const image = yield* Docker.Image("GatewayImage", {
    name: "itsaster0/vitapmate-gateway",
    registry: {
      server: "ghcr.io",
      username: yield* Config.String("GHCR_USERNAME").pipe(Config.withDefault("itsAster0")),
      password: yield* Config.Redacted("GHCR_TOKEN"),
    },
    build: { context: "..", dockerfile: "Dockerfile.gateway", platform: "linux/amd64" },
  });
  return yield* Docker.Container("Gateway", {
    context: robin,
    name: "vitapmate-gateway",
    image,
    // The gateway sees each phone's real address (its Noise sessions are keyed
    // by it), with no Docker NAT in the UDP path.
    networkMode: "host",
    volumes: [
      { hostPath: `${GATEWAY_DIR}/resolv.conf`, containerPath: "/etc/resolv.conf", readOnly: true },
      { hostPath: `${GATEWAY_DIR}/data`, containerPath: "/data" },
    ],
    environment: {
      ENGINE_URL,
      EDGE_URL: `https://${DOMAIN}`,
      INTERNAL_TOKEN: yield* internalToken,
      UDP_HOST: "140.245.225.49",
      BIND: "0.0.0.0",
      UDP_PORTS: "3478,51820,8443",
      RUST_LOG: "warn,vtop_gateway=info",
    },
    restart: "unless-stopped",
    logDriver: "json-file",
    logOptions: { "max-size": "10m", "max-file": "3" },
  });
});

export default Alchemy.Stack(
  "vmate",
  {
    providers: Layer.mergeAll(Cloudflare.providers(), Railway.providers(), Docker.providers()),
    state: Cloudflare.state(),
  },
  Effect.gen(function* () {
    const engine = yield* Engine;
    const edge = yield* Edge;
    // SKIP_GATEWAY=1 deploys without it (no GHCR token on this machine); CI always deploys it.
    if (yield* Config.Boolean("SKIP_GATEWAY").pipe(Config.withDefault(false))) {
      return { edge: edge.url, engine: engine.url };
    }
    const gateway = yield* Gateway;
    return { edge: edge.url, engine: engine.url, gateway: gateway.name };
  }),
);
