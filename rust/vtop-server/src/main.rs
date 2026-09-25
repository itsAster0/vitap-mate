use std::sync::Arc;

use tracing_subscriber::EnvFilter;
use vtop_server::config::ServerConfig;
use vtop_server::{app, AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        // Plain text when stdout is a log collector (Railway, Docker).
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stdout()))
        .init();

    let config = match ServerConfig::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("vtop-server: {error}");
            std::process::exit(2);
        }
    };
    let bind = config.bind;
    if config.api_keys.is_empty() {
        tracing::warn!(
            "VTOP_SERVER_API_KEYS is not set: the server is OPEN and anyone who can reach it can use it"
        );
    }
    tracing::info!(
        "starting vtop-server on {bind} (keys={}, login={}, cache_ttl={}s, rate={}/min)",
        config.api_keys.len(),
        config.allow_login,
        config.session_cache_ttl.as_secs(),
        config.rate_limit_per_minute
    );

    let listener = match tokio::net::TcpListener::bind(bind).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("vtop-server: cannot bind {bind}: {error}");
            std::process::exit(1);
        }
    };
    let app = app(Arc::new(AppState::new(config)));
    if let Err(error) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        eprintln!("vtop-server: {error}");
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
