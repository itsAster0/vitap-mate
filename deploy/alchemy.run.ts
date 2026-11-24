// The servers around backend v3, deployed with Alchemy from a laptop: `bun run deploy` here
// (stage `prod`), which then deploys the Convex functions (`bunx convex deploy` in ../backend).
// Needs the variables in .env.example (in .env here); CI deploys nothing. State lives in the account's Alchemy
// state-store Worker.
//
// - Convex: self-hosted on hawk (infra repo, services/vmate-convex), served at
//   vmate.aster0.dev by hawk's Caddy. Its env holds the v3 secrets, and the
//   resources below read theirs from it (`convexEnv`), so there is one copy.
// - Engine: on nest, built there and published on its NetBird address (hawk's
//   Convex and Caddy reach it there). Railway `engine-v2` is kept up to date as a
//   standby with the same settings (ENGINE_URL in Convex picks which one is used).
// - Gateway: the UDP gateway on hawk (the Linode fedora-in-maa), built on nest,
//   pushed to GHCR and run over SSH as `vitapmate-gateway`. It reaches Convex on
//   the same machine and the engine over NetBird. Caddy holds UDP 443 there and
//   nginx 50000-50200, so it takes 3478-3481 (open in the Linode and host firewalls).
// - Mail: the `vmate-mail` Worker, which hands forwarded OTP mail to Convex,
//   with its retry queue.
// - Proxy: the `vmate-proxy` Worker on vmate-proxy.aster0.dev (the Proxy route).
// - The v2 Worker (`vmate-edge`) and its Durable Objects are no longer declared, so
//   the first deploy without them deletes them; their data is in Convex since
//   the cutover (docs/plans/2026-11-24-v3-changes.md).
//
// Not here: the Email Routing rule otp@vmate.aster0.dev → vmate-mail (made in the
// dashboard; subdomains have no catch-all), and DNS (infra repo, dns/).

import { execFileSync } from "node:child_process";
import * as Alchemy from "alchemy";
import { adopt } from "alchemy/AdoptPolicy";
import * as Cloudflare from "@alchemy.run/cloudflare";
import * as Docker from "@alchemy.run/docker";
import * as Railway from "@alchemy.run/railway";
import * as Config from "effect/Config";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as Redacted from "effect/Redacted";

const DOMAIN = "vmate.aster0.dev";
const CONVEX_URL = `https://${DOMAIN}`;
/** nest's NetBird address: the engine is published there only. */
const NEST_IP = "100.126.222.47";
const ENGINE_URL = `http://${NEST_IP}:8090`;
const HAWK_IP = "172.232.115.40";

/** A secret from the Convex deployment's env (`bunx convex env get`, with ../backend/.env.local). */
const convexEnv = (name: string) =>
  Effect.sync(() => {
    const value = execFileSync("bunx", ["convex", "env", "get", name], {
      cwd: "../backend",
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    if (!value) throw new Error(`${name} is not set in Convex`);
    return Redacted.make(value);
  });

/** What the engine needs, wherever it runs. */
const engineEnv = Effect.gen(function* () {
  return {
    CONVEX_URL,
    INTERNAL_TOKEN: yield* convexEnv("INTERNAL_TOKEN"),
    GATEWAY_TOKEN: yield* convexEnv("GATEWAY_TOKEN"),
    // The v2 engine's key: vaults and sessions sealed on v2 still open.
    VAULT_KEY: yield* Config.Redacted("VAULT_KEY"),
    RUST_LOG: "warn,vtop_bridge=info",
  };
});

const nest = Effect.gen(function* () {
  return yield* Docker.Context("Builder", {
    name: "nest",
    docker: `host=${yield* Config.String("GATEWAY_BUILDER").pipe(Config.withDefault("ssh://root@nest.netbird.cloud"))}`,
  });
});

export const Engine = Effect.gen(function* () {
  const host = yield* nest;
  const image = yield* Docker.Image("NestEngineImage", {
    context: host,
    name: "vmate-engine",
    build: { context: "../rust", dockerfile: "Dockerfile.engine", platform: "linux/amd64" },
  });
  return yield* Docker.Container("NestEngine", {
    context: host,
    name: "vmate-engine",
    image,
    ports: [{ external: `${NEST_IP}:8090`, internal: 8080 }],
    // Caddy on hawk sends /proof here: one hop, so X-Forwarded-For's last entry is the phone.
    environment: { ...(yield* engineEnv), PROXY_HOPS: "1" },
    start: true,
    restart: "unless-stopped",
    logDriver: "json-file",
    logOptions: { "max-size": "10m", "max-file": "3" },
  }).pipe(adopt(true));
});

export const RailwayEngine = Effect.gen(function* () {
  const project = yield* Railway.Project("Vitapmate", { name: "vitapmate" }).pipe(adopt(true));
  // "Engine" was its id when it was the only engine: kept, so it is updated, not replaced.
  return yield* Railway.Service("Engine", {
    project,
    name: "engine-v2",
    context: "../rust",
    dockerfilePath: "Dockerfile.engine",
    port: 8080,
    // Behind hawk's Caddy and Railway's own proxy: the phone is second from the end.
    env: { ...(yield* engineEnv), PROXY_HOPS: "2" },
  }).pipe(adopt(true));
});

export const Mail = Effect.gen(function* () {
  const queue = yield* Cloudflare.Queues.Queue("MailQueue", { name: "vmate-mail-retry" }).pipe(adopt(true));
  const worker = yield* Cloudflare.Worker("Mail", {
    name: "vmate-mail",
    main: "../mail/src/index.ts",
    compatibility: { date: "2026-10-01" },
    workersDev: false,
    observability: { enabled: true, logs: { enabled: true, invocationLogs: false } },
    env: {
      CONVEX_URL,
      // Where mail to other or unknown addresses goes; empty rejects it.
      FALLBACK_FORWARD: "",
      MAIL_TOKEN: yield* convexEnv("MAIL_TOKEN"),
      MAIL_QUEUE: queue,
    },
  }).pipe(adopt(true));
  // The consumer gives up after 10 minutes on its own (../mail/src/index.ts), well inside
  // maxRetries with its backoff.
  yield* Cloudflare.Queues.Consumer("MailRetry", {
    queueId: queue.queueId,
    scriptName: worker.workerName,
    settings: { batchSize: 10, maxWaitTimeMs: 1000, maxRetries: 20 },
  }).pipe(adopt(true));
  return worker;
});

export const Proxy = Cloudflare.Worker("Proxy", {
  name: "vmate-proxy",
  main: "../proxy/src/index.ts",
  compatibility: { date: "2026-10-01" },
  domain: "vmate-proxy.aster0.dev",
  workersDev: false,
  observability: { enabled: true, logs: { enabled: true, invocationLogs: false } },
}).pipe(adopt(true));

/** Where the gateway's files live on hawk: its Noise keys (./data) and resolvers. */
const GATEWAY_DIR = "/root/vitapmate-gateway";

export const Gateway = Effect.gen(function* () {
  const host = yield* Docker.Context("Linode", {
    name: "fedora-in-maa",
    docker: `host=${yield* Config.String("GATEWAY_SSH").pipe(Config.withDefault(`ssh://root@${HAWK_IP}`))}`,
  });
  const image = yield* Docker.Image("GatewayImage", {
    context: yield* nest,
    // With the host, or Alchemy pushes the bare name to Docker Hub.
    name: "ghcr.io/itsaster0/vitapmate-gateway",
    registry: {
      server: "ghcr.io",
      username: yield* Config.String("GHCR_USERNAME").pipe(Config.withDefault("itsAster0")),
      password: yield* Config.Redacted("GHCR_TOKEN"),
    },
    build: { context: "../rust", dockerfile: "Dockerfile.gateway", platform: "linux/amd64" },
  });
  // The image is built on nest, so hawk has only the tag it pulled last: a container made from
  // `image` would run that stale binary. Pulled on the host each deploy.
  const pulled = yield* Docker.RemoteImage("GatewayPull", {
    context: host,
    name: image.name,
    tag: image.tag,
    platform: "linux/amd64",
  });
  return yield* Docker.Container("Gateway", {
    context: host,
    name: "vitapmate-gateway",
    image: pulled,
    // The gateway sees each phone's real address (its Noise sessions are keyed
    // by it), with no Docker NAT in the UDP path.
    networkMode: "host",
    volumes: [
      { hostPath: `${GATEWAY_DIR}/resolv.conf`, containerPath: "/etc/resolv.conf", readOnly: true },
      { hostPath: `${GATEWAY_DIR}/data`, containerPath: "/data" },
    ],
    environment: {
      ENGINE_URL,
      // Convex's HTTP actions on the same machine: its OTP polls never leave hawk.
      EDGE_URL: "http://127.0.0.1:3411",
      GATEWAY_TOKEN: yield* convexEnv("GATEWAY_TOKEN"),
      UDP_HOST: HAWK_IP,
      BIND: "0.0.0.0",
      UDP_PORTS: "3478,3479,3480,3481",
      RUST_LOG: "warn,vtop_gateway=info",
    },
    start: true,
    restart: "unless-stopped",
    logDriver: "json-file",
    logOptions: { "max-size": "10m", "max-file": "3" },
  }).pipe(adopt(true));
});

export default Alchemy.Stack(
  "vmate",
  {
    providers: Layer.mergeAll(Cloudflare.providers(), Railway.providers(), Docker.providers()),
    state: Cloudflare.state(),
  },
  Effect.gen(function* () {
    const engine = yield* Engine;
    const railway = yield* RailwayEngine;
    const mail = yield* Mail;
    const proxy = yield* Proxy;
    const out = { engine: engine.name, railway: railway.url, mail: mail.workerName, proxy: proxy.url };
    // SKIP_GATEWAY=1 deploys without it (when there is no GHCR_TOKEN).
    if (yield* Config.Boolean("SKIP_GATEWAY").pipe(Config.withDefault(false))) return out;
    const gateway = yield* Gateway;
    return { ...out, gateway: gateway.name };
  }),
);
