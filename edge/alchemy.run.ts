// The server, deployed with Alchemy from a laptop: `bun run deploy` here, stage `prod`.
// Needs the variables in .env.example; CI deploys nothing.
// State lives in the account's Alchemy state-store Worker.
//
// - Edge: the `vmate-edge` Worker on vmate.aster0.dev, its Durable Objects
//   and secrets (wrangler.toml is kept for `wrangler dev` and the tests).
// - Engine: Railway `engine-v2` in the `vitapmate` project, built from
//   ../rust/Dockerfile.engine.
// - Gateway: the UDP gateway on the Linode fedora-in-maa (Chennai), built on nest (x86_64,
//   over NetBird; GATEWAY_BUILDER), pushed to GHCR (public) and run over SSH as
//   the `vitapmate-gateway` container. GATEWAY_SSH is the Docker host, e.g.
//   ssh://root@172.232.115.40. Caddy holds UDP 443 there and nginx 50000-50200,
//   so the gateway takes 3478-3481 (open in the Linode and host firewalls).
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

/** Shared by the edge and the engine (`authorization: Bearer`). */
const internalToken = Config.Redacted("INTERNAL_TOKEN");
/** The UDP gateway's own: the engine takes it for fetches and sign-ins, the
 * edge for `/internal/udp-keys`, and nothing else does, so the public VPS
 * never holds `INTERNAL_TOKEN`. */
const gatewayToken = Config.Redacted("GATEWAY_TOKEN");

export const Engine = Effect.gen(function* () {
  const project = yield* Railway.Project("Vitapmate", { name: "vitapmate" }).pipe(adopt(true));
  return yield* Railway.Service("Engine", {
    project,
    name: "engine-v2",
    context: "../rust",
    dockerfilePath: "Dockerfile.engine",
    port: 8080,
    env: {
      EDGE_URL: `https://${DOMAIN}`,
      INTERNAL_TOKEN: yield* internalToken,
      GATEWAY_TOKEN: yield* gatewayToken,
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
  // Public help content (guides/, served under /guides/ by Static Assets). A
  // request matching a file never runs the Worker, so it skips auth and the
  // Durable Objects; any other path falls through to the Worker's router.
  // Cache-Control is in guides/_headers.
  assets: { directory: "./guides", base: "/guides" },
  observability: {
    enabled: true,
    // The Worker's own log lines only, and 1 request in 20 traced: stored spans and log
    // events count toward Observability usage.
    logs: { enabled: true, invocationLogs: false },
    traces: { enabled: true, headSamplingRate: 0.05 },
  },
  env: {
    DEVICE: Cloudflare.DurableObject("DEVICE", { className: "Device" }),
    MAILBOX: Cloudflare.DurableObject("MAILBOX", { className: "Mailbox" }),
    INDEX: Cloudflare.DurableObject("INDEX", { className: "Index" }),
    FCM_TOKEN: Cloudflare.DurableObject("FCM_TOKEN", { className: "FcmToken" }),
    ACCOUNT: Cloudflare.DurableObject("ACCOUNT", { className: "Account" }),
    OAUTH: Cloudflare.DurableObject("OAUTH", { className: "OAuth" }),
    // Addresses are otp+<id>@vmate.aster0.dev: one Email Routing rule, otp@vmate.aster0.dev → this Worker.
    MAIL_DOMAIN: DOMAIN,
    ENGINE_URL,
    FCM_URL: "https://fcm.googleapis.com",
    PUBLIC_URL: `https://${DOMAIN}`,
    ALLOWED_ORIGINS: "",
    FALLBACK_FORWARD: "",
    INTERNAL_TOKEN: internalToken,
    GATEWAY_TOKEN: gatewayToken,
    ACCOUNT_TOKEN_SECRET: Config.Redacted("ACCOUNT_TOKEN_SECRET"),
    FIREBASE_CREDENTIALS_JSON: Config.Redacted("FIREBASE_CREDENTIALS_JSON"),
  },
}).pipe(adopt(true));

/** Where the gateway's files live on the Linode: its Noise keys (./data) and resolvers. */
const GATEWAY_DIR = "/root/vitapmate-gateway";

export const Gateway = Effect.gen(function* () {
  const host = yield* Docker.Context("Linode", {
    name: "fedora-in-maa",
    docker: `host=${yield* Config.String("GATEWAY_SSH").pipe(Config.withDefault("ssh://root@172.232.115.40"))}`,
  });
  const builder = yield* Docker.Context("Builder", {
    name: "nest",
    docker: `host=${yield* Config.String("GATEWAY_BUILDER").pipe(Config.withDefault("ssh://root@nest.netbird.cloud"))}`,
  });
  const image = yield* Docker.Image("GatewayImage", {
    context: builder,
    // With the host, or Alchemy pushes the bare name to Docker Hub.
    name: "ghcr.io/itsaster0/vitapmate-gateway",
    registry: {
      server: "ghcr.io",
      username: yield* Config.String("GHCR_USERNAME").pipe(Config.withDefault("itsAster0")),
      password: yield* Config.Redacted("GHCR_TOKEN"),
    },
    build: { context: "../rust", dockerfile: "Dockerfile.gateway", platform: "linux/amd64" },
  });
  return yield* Docker.Container("Gateway", {
    context: host,
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
      GATEWAY_TOKEN: yield* gatewayToken,
      UDP_HOST: "172.232.115.40",
      BIND: "0.0.0.0",
      UDP_PORTS: "3478,3479,3480,3481",
      RUST_LOG: "warn,vtop_gateway=info",
    },
    start: true,
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
    // SKIP_GATEWAY=1 deploys without it (when there is no GHCR_TOKEN).
    if (yield* Config.Boolean("SKIP_GATEWAY").pipe(Config.withDefault(false))) {
      return { edge: edge.url, engine: engine.url };
    }
    const gateway = yield* Gateway;
    return { edge: edge.url, engine: engine.url, gateway: gateway.name };
  }),
);
