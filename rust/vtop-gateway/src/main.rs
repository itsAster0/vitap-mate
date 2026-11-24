//! `vtop-gateway`: UDP mode's gateway, next to the engine on hawk.
//!
//! Environment, every one required (the deploy's compose sets them all):
//! `ENGINE_URL` (the engine's base URL), `CONVEX_URL` (Convex's HTTP actions),
//! `GATEWAY_TOKEN` (its own bearer token, which the engine takes for fetches
//! and sign-ins and Convex for `/internal/udp-keys`), `UDP_HOST` (the public
//! address the app sends UDP to), `UDP_PORTS` (comma-separated), `BIND` (the
//! address to listen on) and `GATEWAY_DATA_DIR` (where its keys are kept).

use tracing_subscriber::EnvFilter;
use vtop_gateway::{Config, bind, run};

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
        let ports: Vec<u16> = var("UDP_PORTS")?
            .split(',')
            .map(|s| {
                s.trim()
                    .parse()
                    .map_err(|_| "UDP_PORTS has an invalid port".to_string())
            })
            .collect::<Result<_, _>>()?;
        let config = Config {
            engine_url: var("ENGINE_URL")?,
            edge_url: var("CONVEX_URL")?,
            login_poll: std::time::Duration::from_secs(2),
            token: var("GATEWAY_TOKEN")?,
            host: var("UDP_HOST")?,
            data_dir: var("GATEWAY_DATA_DIR")?.into(),
            ports,
        };
        let sockets = bind(&var("BIND")?, &config.ports).await?;
        Ok::<_, String>((config, sockets))
    };
    match bound.await {
        Ok((config, sockets)) => run(config, sockets).await,
        Err(e) => {
            #[expect(
                clippy::print_stderr,
                reason = "the configuration error, before tracing can say it"
            )]
            {
                eprintln!("vtop-gateway: {e}");
            }
            std::process::exit(2);
        }
    }
}
