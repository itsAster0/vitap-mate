//! `vtop-gateway`: the UDP gateway alone, or `vtop-gateway keygen` for a
//! new key pair. On Fly the engine runs the gateway in its own process
//! instead (see `vtop-bridge/src/main.rs`); this binary is for running it
//! next to an engine elsewhere.
//!
//! Environment: `ENGINE_URL` (the engine's base URL), `BRIDGE_SECRET` (the
//! engine's relay secret, as the edge holds it), `EDGE_URL`, and `GATEWAY_KEY`, `BIND`
//! and `PORTS` as [`vtop_gateway::bind_from_env`] reads them. The key's
//! public half goes into the app build (`VTOP_UDP_KEY`) and is logged at
//! start.

use tracing_subscriber::EnvFilter;
use vtop_gateway::{bind_from_env, run};
use vtop_wire::hex;

#[tokio::main]
async fn main() {
    if std::env::args().nth(1).as_deref() == Some("keygen") {
        let (private, public) = vtop_wire::generate_keypair();
        println!("GATEWAY_KEY={}", hex(&private));
        println!("VTOP_UDP_KEY={}", hex(&public));
        return;
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        // Plain text when stdout is a log collector (Fly, Docker).
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stdout()))
        .init();
    let var = |name: &str| std::env::var(name).map_err(|_| format!("{name} is not set"));
    let bound = match (var("ENGINE_URL"), var("BRIDGE_SECRET"), var("EDGE_URL")) {
        (Ok(url), Ok(secret), Ok(edge)) => bind_from_env(url, secret, edge)
            .await
            .unwrap_or_else(|| Err("GATEWAY_KEY is not set".into())),
        (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => Err(e),
    };
    match bound {
        Ok((config, sockets)) => run(config, sockets).await,
        Err(e) => {
            eprintln!("vtop-gateway: {e}");
            std::process::exit(2);
        }
    }
}
