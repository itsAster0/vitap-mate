//! `vtop-gateway`: UDP mode's gateway, on its own host next to an engine
//! elsewhere.
//!
//! Environment: `ENGINE_URL` (the engine's base URL), `EDGE_URL`,
//! `INTERNAL_TOKEN` (the bearer token shared with the engine and the edge),
//! `UDP_HOST` (the public name the app sends UDP to), `UDP_PORTS`
//! (comma-separated, default 3478,51820,443), `BIND` (default `0.0.0.0`)
//! and `GATEWAY_DATA_DIR` (where its keys are kept, default `./data`).

use tracing_subscriber::EnvFilter;
use vtop_gateway::{bind, run, Config};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        // Plain text when stdout is a log collector (Docker).
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stdout()))
        .init();
    let var = |name: &str| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("{name} is not set"))
    };
    let bound = async {
        let ports: Vec<u16> = match var("UDP_PORTS") {
            Ok(p) => p.split(',').filter_map(|s| s.trim().parse().ok()).collect(),
            Err(_) => vtop_wire::PORTS.to_vec(),
        };
        let config = Config {
            engine_url: var("ENGINE_URL")?,
            edge_url: var("EDGE_URL")?,
            token: var("INTERNAL_TOKEN")?,
            host: var("UDP_HOST")?,
            data_dir: var("GATEWAY_DATA_DIR")
                .unwrap_or_else(|_| "./data".into())
                .into(),
            ports,
        };
        let sockets = bind(
            &var("BIND").unwrap_or_else(|_| "0.0.0.0".into()),
            &config.ports,
        )
        .await?;
        Ok::<_, String>((config, sockets))
    };
    match bound.await {
        Ok((config, sockets)) => run(config, sockets).await,
        Err(e) => {
            eprintln!("vtop-gateway: {e}");
            std::process::exit(2);
        }
    }
}
