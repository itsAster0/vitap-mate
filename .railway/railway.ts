import { defineRailway, github, preserve, project, service } from "railway/iac";

// This repository manages vtop-server (vtop-bridge and vtop-mcp moved to
// the Cloudflare edge, rust/vtop-edge); vtop_cap and
// vtop_fmc (the Go bridge, until it is retired) live in their own
// repositories and are left alone.
// See https://docs.railway.com/infrastructure-as-code#multi-repo-projects
export const partial = "vtop-server";

const repo = github("itsAster0/vitap-mate", { checkSuites: false });

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

  return project("vitap-mate", {
    resources: [vtopServer],
  });
});
