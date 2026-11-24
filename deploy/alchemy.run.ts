// The servers around backend v3, deployed with Alchemy from a laptop: `bun run deploy` here
// (stage `prod`), which then deploys the Convex functions (`bunx convex deploy` in ../backend).
// Needs the variables in .env.example (in .env here); CI deploys nothing. State lives in the account's Alchemy
// state-store Worker.
//
// - Convex's env holds the v3 secrets, and the resources below read theirs from it
//   (`convexEnv`), so there is one copy. It is served at vmate.aster0.dev by hawk's Caddy.
// - hawk (the Linode fedora-in-maa): Convex, the engine, the verifier and the UDP gateway in one
//   compose, hawk/compose.yaml, which hawk/up.sh copies to /opt/vmate and starts. The
//   engine, verifier and gateway images are built on nest and pushed to GHCR; hawk logs in
//   only for the pull. Convex's ENGINE_URL is the engine on hawk's NetBird address.
// - Verifier standby on nest (its NetBird address): hawk's Caddy sends /proof to hawk's
//   verifier, else nest's (the infra repo's Caddyfile).
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
import * as Command from "alchemy/Command";
import * as Cloudflare from "@alchemy.run/cloudflare";
import * as Docker from "@alchemy.run/docker";
import * as Config from "effect/Config";
import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as Redacted from "effect/Redacted";

const DOMAIN = "vmate.aster0.dev";
const CONVEX_URL = `https://${DOMAIN}`;
const HAWK_IP = "172.232.115.40";
/** nest's NetBird address: the verifier's standby is published there only. */
const NEST_IP = "100.126.222.47";
/** The GHCR images; hawk logs in only to pull them (hawk/up.sh). */
const ENGINE_IMAGE = "ghcr.io/itsaster0/vitapmate-engine";
const VERIFIER_IMAGE = "ghcr.io/itsaster0/vitapmate-verifier";

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

const ghcr = Effect.gen(function* () {
  return {
    server: "ghcr.io",
    username: yield* Config.String("GHCR_USERNAME").pipe(Config.withDefault("itsAster0")),
    password: yield* Config.Redacted("GHCR_TOKEN"),
  };
});

/** The engine image, built on nest and pushed to GHCR for hawk. */
const EngineImage = Effect.gen(function* () {
  return yield* Docker.Image("NestEngineImage", {
    context: yield* nest,
    name: ENGINE_IMAGE,
    registry: yield* ghcr,
    build: { context: "../rust", dockerfile: "Dockerfile.engine", platform: "linux/amd64" },
  });
});

type Image = Effect.Success<typeof EngineImage>;

/** The account proof's verifier image, built on nest and pushed to GHCR for hawk. */
const VerifierImage = Effect.gen(function* () {
  return yield* Docker.Image("VerifierImage", {
    context: yield* nest,
    name: VERIFIER_IMAGE,
    registry: yield* ghcr,
    build: { context: "../rust", dockerfile: "Dockerfile.verifier", platform: "linux/amd64" },
  });
});

/** The verifier's standby on nest: hawk's Caddy sends /proof here while hawk's is down. nest has
 * the cores, so it takes more proofs at once. Its only secret is VERIFIER_TOKEN. */
const NestVerifier = (image: Image) =>
  Effect.gen(function* () {
    return yield* Docker.Container("NestVerifier", {
      context: yield* nest,
      name: "vmate-verifier",
      image,
      ports: [{ external: `${NEST_IP}:8091`, internal: 8080 }],
      environment: {
        CONVEX_URL,
        PORT: "8080",
        VERIFIER_TOKEN: yield* convexEnv("VERIFIER_TOKEN"),
        MAX_PROOFS: "6",
        MAX_QUEUE: "64",
        QUEUE_WAIT_SECS: "60",
        // Behind hawk's Caddy: one hop.
        PROXY_HOPS: "1",
        RUST_LOG: "warn,vtop_verifier=info",
      },
      memory: "1g",
      start: true,
      restart: "unless-stopped",
      logDriver: "json-file",
      logOptions: { "max-size": "10m", "max-file": "3" },
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

/** Convex, the engine, the verifier and the gateway on hawk (hawk/compose.yaml). Runs again when
 * an image, a secret or a file in hawk/ changes. */
const Hawk = (engine: Image, verifier: Image) =>
  Effect.gen(function* () {
    const gateway = yield* Docker.Image("GatewayImage", {
      context: yield* nest,
      // With the host, or Alchemy pushes the bare name to Docker Hub.
      name: "ghcr.io/itsaster0/vitapmate-gateway",
      registry: yield* ghcr,
      build: { context: "../rust", dockerfile: "Dockerfile.gateway", platform: "linux/amd64" },
    });
    const env = yield* engineEnv;
    const registry = yield* ghcr;
    return yield* Command.Exec("Hawk", {
      command: "./hawk/up.sh",
      memo: { include: ["hawk/**"] },
      timeout: "10 minutes",
      env: {
        HAWK: yield* Config.String("GATEWAY_SSH").pipe(
          Config.map((url) => url.replace(/^ssh:\/\//, "")),
          Config.withDefault(`root@${HAWK_IP}`),
        ),
        ENGINE_IMAGE: engine.repoDigest ?? engine.imageRef,
        GATEWAY_IMAGE: gateway.repoDigest ?? gateway.imageRef,
        VERIFIER_IMAGE: verifier.repoDigest ?? verifier.imageRef,
        CONVEX_URL: env.CONVEX_URL,
        INTERNAL_TOKEN: env.INTERNAL_TOKEN,
        GATEWAY_TOKEN: env.GATEWAY_TOKEN,
        VAULT_KEY: env.VAULT_KEY,
        VERIFIER_TOKEN: yield* convexEnv("VERIFIER_TOKEN"),
        // hawk logs in for the pull only (hawk/up.sh), so the images need not be public.
        GHCR_USERNAME: registry.username,
        GHCR_TOKEN: registry.password,
      },
    });
  });

export default Alchemy.Stack(
  "vmate",
  {
    providers: Layer.mergeAll(Cloudflare.providers(), Docker.providers()),
    state: Cloudflare.state(),
  },
  Effect.gen(function* () {
    const engine = yield* EngineImage;
    const verifier = yield* VerifierImage;
    yield* Hawk(engine, verifier);
    const standby = yield* NestVerifier(verifier);
    const mail = yield* Mail;
    const proxy = yield* Proxy;
    return { verifierStandby: standby.name, mail: mail.workerName, proxy: proxy.url };
  }),
);
