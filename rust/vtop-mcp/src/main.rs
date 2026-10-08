use std::sync::Arc;

use tracing_subscriber::EnvFilter;
use vtop_mcp::bridge::BridgeClient;
use vtop_mcp::config::Config;
use vtop_mcp::router;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        // Plain text when stdout is a log collector (Railway, Docker).
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stdout()))
        .init();

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("vtop-mcp: {error}");
            std::process::exit(2);
        }
    };
    let listen = config.listen;
    tracing::info!("starting vtop-mcp on {listen}");

    let source = Arc::new(BridgeClient::new(
        reqwest::Client::new(),
        config.bridge_url.clone(),
    ));
    let listener = match tokio::net::TcpListener::bind(listen).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("vtop-mcp: cannot bind {listen}: {error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = axum::serve(listener, router(&config, source))
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        eprintln!("vtop-mcp: {error}");
        std::process::exit(1);
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
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
