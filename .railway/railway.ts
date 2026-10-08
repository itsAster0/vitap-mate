import { defineRailway, github, preserve, project, service } from "railway/iac";

// This repository manages vtop-server, vtop-bridge and vtop-mcp; vtop_cap and
// vtop_fmc (the Go bridge, until it is retired) live in their own
// repositories and are left alone.
// See https://docs.railway.com/infrastructure-as-code#multi-repo-projects
export const partial = "vtop-server";

const repo = github("itsAster0/vitap-mate", { checkSuites: false });

// Every Rust service builds from the Cargo workspace in rust/.
function rustService(
  name: string,
  options: {
    healthcheck: string;
    watch: string[];
    env: Parameters<typeof service>[1] extends { env?: infer E } ? E : never;
  },
) {
  return service(name, {
    source: repo,
    // The Cargo workspace lives in rust/, not at the repository root.
    rootDirectory: "/rust",
    build: {
      buildEnvironment: "V3",
      builder: "RAILPACK",
      // Matched from the repository root, even with a root directory set.
      watchPatterns: [
        "/rust/vtop-core/**",
        "/rust/Cargo.toml",
        "/rust/Cargo.lock",
        ...options.watch,
      ],
    },
    env: options.env,
    healthcheck: options.healthcheck,
    healthcheckTimeout: 30,
    replicas: { "asia-southeast1-eqsg3a": 1 },
    deploy: {
      limitOverride: { containers: { cpu: 2, memoryBytes: 2000000000 } },
      restartPolicyType: "ON_FAILURE",
      restartPolicyMaxRetries: 5,
      // Serverless: sleeps when idle; private-network traffic wakes it.
      sleepApplication: true,
    },
  });
}

export default defineRailway(() => {
  // Runs rust/vtop-server, reading rust/railpack.json (pinned Rust and the
  // `server` profile). Set VTOP_SERVER_API_KEYS in the dashboard; without it
  // the server is open to anyone.
  const vtopServer = service("vtop-server", {
    source: repo,
    rootDirectory: "/rust",
    build: {
      buildEnvironment: "V3",
      builder: "RAILPACK",
      watchPatterns: [
        "/rust/vtop-core/**",
        "/rust/vtop-server/**",
        "/rust/Cargo.toml",
        "/rust/Cargo.lock",
        "/rust/railpack.json",
      ],
    },
    // Set in the dashboard; preserve() keeps it (unlisted variables are deleted).
    env: { VTOP_SERVER_API_KEYS: preserve() },
    healthcheck: "/health",
    healthcheckTimeout: 30,
    replicas: { "asia-southeast1-eqsg3a": 1 },
    deploy: {
      limitOverride: { containers: { cpu: 2, memoryBytes: 2000000000 } },
      restartPolicyType: "ON_FAILURE",
      restartPolicyMaxRetries: 5,
      sleepApplication: true,
    },
    networking: { privateNetworkEndpoint: "vitap-mate" },
  });

  // Cookie bridge, credential vault and session broker. Secrets are set in
  // the dashboard and kept with preserve(); unlisted variables are deleted.
  const vtopBridge = rustService("vtop-bridge", {
    healthcheck: "/healthz",
    watch: ["/rust/vtop-bridge/**", "/rust/railpack.vtop-bridge.json"],
    env: {
      RAILPACK_CONFIG_FILE: "railpack.vtop-bridge.json",
      // Fixed so vtop-mcp can reach it on the private network.
      PORT: "8080",
      // PUBLIC_BASE_URL is unset: the bridge uses its RAILWAY_PUBLIC_DOMAIN.
      PUBLIC_MCP_URL: "https://${{vtop-mcp.RAILWAY_PUBLIC_DOMAIN}}/mcp",
      FIREBASE_CREDENTIALS_JSON: preserve(),
      VAULT_KEY: preserve(),
    },
  });

  // MCP server. Callers' access keys are checked by the bridge, so it holds
  // no secrets.
  const vtopMcp = rustService("vtop-mcp", {
    healthcheck: "/health",
    watch: ["/rust/vtop-mcp/**", "/rust/railpack.vtop-mcp.json"],
    env: {
      RAILPACK_CONFIG_FILE: "railpack.vtop-mcp.json",
      BRIDGE_URL: "http://${{vtop-bridge.RAILWAY_PRIVATE_DOMAIN}}:8080",
    },
  });

  return project("vitap-mate", {
    resources: [vtopServer, vtopBridge, vtopMcp],
  });
});
