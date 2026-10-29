//! `vtop-gateway`: UDP mode's gateway, on its own host next to an engine
//! elsewhere.
//!
//! Environment: `ENGINE_URL` (the engine's base URL), `BRIDGE_SECRET` (the
//! engine's relay secret, as the edge holds it), `EDGE_URL`,
//! `GATEWAY_IDENTITY` (opens the Noise keys the edge seals to this gateway),
//! and `BIND` and `PORTS` as [`vtop_gateway::bind_from_env`] reads them.
//! `vtop-gateway identity` makes a new identity: the secret for
//! `GATEWAY_IDENTITY`, and its public half for the edge's
//! `GATEWAY_IDENTITY_KEY`.

use tracing_subscriber::EnvFilter;
use vtop_gateway::{bind_from_env, run, Config};

#[tokio::main]
async fn main() {
    if std::env::args().nth(1).as_deref() == Some("identity") {
        let (secret, public) = vtop_shared::mail_seal::generate_keypair();
        println!("GATEWAY_IDENTITY={secret}");
        println!("GATEWAY_IDENTITY_KEY={public}");
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
    let bound = async {
        let config = Config {
            engine_url: var("ENGINE_URL")?,
            engine_secret: var("BRIDGE_SECRET")?,
            edge_url: var("EDGE_URL")?,
            identity: var("GATEWAY_IDENTITY")?,
        };
        Ok::<_, String>((config, bind_from_env().await?))
    };
    match bound.await {
        Ok((config, sockets)) => run(config, sockets).await,
        Err(e) => {
            eprintln!("vtop-gateway: {e}");
            std::process::exit(2);
        }
    }
}
