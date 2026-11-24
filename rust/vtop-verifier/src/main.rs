#![expect(
    clippy::print_stderr,
    reason = "a start-up failure's last line, on stderr before the exit code"
)]

use std::sync::Arc;

use tracing_subscriber::EnvFilter;
use vtop_verifier::config::Config;
use vtop_verifier::convex::HttpConvex;
use vtop_verifier::{AppState, router};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        // Plain text when stdout is a log collector (Docker).
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stdout()))
        .init();

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("vtop-verifier: {error}");
            std::process::exit(2);
        }
    };
    let http = match vtop_core::client::https_client(&[]).build() {
        Ok(http) => http,
        Err(error) => {
            eprintln!("vtop-verifier: no HTTP client: {error}");
            std::process::exit(1);
        }
    };
    let convex = Arc::new(HttpConvex {
        http,
        base_url: config.convex_url.clone(),
        token: config.token.clone(),
    });
    let state = Arc::new(AppState::new(&config, convex));
    let listen = config.listen;
    tracing::info!(
        max_proofs = config.max_proofs,
        max_queue = config.max_queue,
        queue_wait_secs = config.queue_wait.as_secs(),
        "starting vtop-verifier on {listen}"
    );

    // Hosts without IPv6 (some containers) refuse `[::]`; take IPv4 there.
    let ipv4 = std::net::SocketAddr::from((std::net::Ipv4Addr::UNSPECIFIED, listen.port()));
    let bound = match tokio::net::TcpListener::bind(listen).await {
        Err(error) if listen.is_ipv6() => {
            tracing::info!("cannot bind {listen} ({error}); using {ipv4}");
            tokio::net::TcpListener::bind(ipv4).await
        }
        bound => bound,
    };
    let listener = match bound {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("vtop-verifier: cannot bind {listen}: {error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        eprintln!("vtop-verifier: {error}");
        std::process::exit(1);
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        #[expect(
            clippy::let_underscore_must_use,
            reason = "an error ends the wait, as a ctrl-c would"
        )]
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}
