// `Engine`: runs the engine (`vtop-bridge`) in a Cloudflare Container and
// passes requests to it. One instance, named "engine", serves everyone.
//
// The container starts on the first request, with this Worker's secrets as
// its environment, and stops `sleepAfter` after the last one. Calls reach it
// here when `BRIDGE_URL` / `MCP_URL` are set to "container" (see
// `IN_CONTAINER` in common.ts).
//
// `@cloudflare/containers` handles starting, waiting for the port and the
// inactivity timer.

import { Container } from "@cloudflare/containers";

interface Env {
  VAULT_KEY: string;
  BRIDGE_SECRET: string;
  PUBLIC_URL: string;
}

const PORT = 8080;

export class Engine extends Container<Env> {
  defaultPort = PORT;
  // Long enough for a sign-in the engine finishes after answering.
  sleepAfter = "5m";
  enableInternet = true;

  constructor(ctx: DurableObjectState<{}>, env: Env) {
    super(ctx, env);
    this.envVars = {
      PORT: String(PORT),
      // Warnings, plus the engine's one line per request; info from rmcp and
      // VTOP fetches is noise (and billed as logs).
      RUST_LOG: "warn,vtop_bridge=info",
      VAULT_KEY: env.VAULT_KEY,
      MAIL_RELAY_SECRET: env.BRIDGE_SECRET,
      EDGE_URL: env.PUBLIC_URL,
    };
  }

  override onStart(): void {
    console.log("engine: container started");
  }

  override async fetch(request: Request): Promise<Response> {
    try {
      // How long a request waits for a starting container to listen.
      await this.startAndWaitForPorts(PORT, { portReadyTimeoutMS: 20_000 });
    } catch (e) {
      console.log(`engine: container did not answer: ${e}`);
      return Response.json(
        { code: "engine_unavailable", error: "The engine is starting; try again." },
        { status: 503 },
      );
    }
    return this.containerFetch(request);
  }
}
