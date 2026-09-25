import { defineRailway, github, project, service } from "railway/iac";

// This repository manages only the vtop-server service; vtop_cap and vtop_fmc
// live in their own repositories and are left alone.
// See https://docs.railway.com/infrastructure-as-code#multi-repo-projects
export const partial = "vtop-server";

export default defineRailway(() => {
  // Runs rust/vtop-server. Set VTOP_SERVER_API_KEYS in the dashboard; without
  // it the server is open to anyone.
  const vtopServer = service("vtop-server", {
    source: github("itsAster0/vitap-mate", { checkSuites: false }),
    // The Cargo workspace lives in rust/, not at the repository root.
    rootDirectory: "/rust",
    build: {
      buildEnvironment: "V3",
      // Reads rust/railpack.json: pinned Rust and the `server` profile.
      builder: "RAILPACK",
      // Matched from the repository root, even with a root directory set.
      watchPatterns: [
        "/rust/vtop-core/**",
        "/rust/vtop-server/**",
        "/rust/Cargo.toml",
        "/rust/Cargo.lock",
        "/rust/railpack.json",
      ],
    },
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
