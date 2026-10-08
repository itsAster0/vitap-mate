use std::sync::Arc;

use tracing_subscriber::EnvFilter;
use vtop_bridge::config::Config;
use vtop_bridge::fcm::{self, FcmMessenger};
use vtop_bridge::firestore::{FirestoreStore, DEFAULT_BASE_URL};
use vtop_bridge::gmail::{GoogleOAuthRefresher, DEFAULT_TOKEN_URL};
use vtop_bridge::google::{ServiceAccountTokens, TokenSource};
use vtop_bridge::login::CoreLogin;
use vtop_bridge::verify::CoreVerifier;
use vtop_bridge::{router, AppState, Parts};

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
            eprintln!("vtop-bridge: {error}");
            std::process::exit(2);
        }
    };
    let listen = config.listen;
    let tokens: Arc<dyn TokenSource> =
        match ServiceAccountTokens::from_json(&config.credentials_json) {
            Ok(tokens) => Arc::new(tokens),
            Err(error) => {
                eprintln!("vtop-bridge: {error}");
                std::process::exit(2);
            }
        };
    let http = reqwest::Client::new();
    let store = Arc::new(FirestoreStore::new(
        http.clone(),
        tokens.clone(),
        DEFAULT_BASE_URL.into(),
    ));
    let messenger = Arc::new(FcmMessenger::new(
        http.clone(),
        tokens,
        fcm::DEFAULT_BASE_URL.into(),
    ));
    let refresher = Arc::new(GoogleOAuthRefresher::new(http, DEFAULT_TOKEN_URL.into()));
    let login = Arc::new(CoreLogin {
        config: vtop_core::VtopConfig::default(),
    });
    let verifier = Arc::new(CoreVerifier {
        config: vtop_core::VtopConfig::default(),
    });
    tracing::info!("starting vtop-bridge on {listen}");

    let listener = match tokio::net::TcpListener::bind(listen).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("vtop-bridge: cannot bind {listen}: {error}");
            std::process::exit(1);
        }
    };
    let app = router(Arc::new(AppState::new(
        config,
        Parts {
            store,
            messenger,
            refresher,
            login,
            verifier,
        },
    )))
    .into_make_service_with_connect_info::<std::net::SocketAddr>();
    if let Err(error) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        eprintln!("vtop-bridge: {error}");
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
