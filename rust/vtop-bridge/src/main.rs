#![expect(
    clippy::print_stderr,
    reason = "a start-up failure's last line, on stderr before the exit code"
)]

use std::sync::Arc;
use std::time::Duration;

use tracing_subscriber::EnvFilter;
use vtop_bridge::config::Config;
use vtop_bridge::gmail::{DEFAULT_TOKEN_URL, GoogleOAuthRefresher};
use vtop_bridge::login::CoreLogin;
use vtop_bridge::verify::CoreVerifier;
use vtop_bridge::{AppState, Parts, router};

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
            eprintln!("vtop-engine: {error}");
            std::process::exit(2);
        }
    };
    let listen = config.listen;
    // The MCP tools run here too, with sessions from the edge under each
    // caller's own key; the edge sends `/mcp` on to this service.
    // Sessions are kept a few minutes, so a run of tool calls asks the edge
    // (and VTOP, to check it) once.
    let sessions = Arc::new(vtop_mcp::bridge::CachedSessions::new(
        Arc::new(
            vtop_mcp::bridge::BridgeClient::new(https(), config.edge_url.clone())
                .with_internal_token(config.internal_token.clone()),
        ),
        Duration::from_secs(300),
    ));
    let (mcp, mcp_evictor) = vtop_mcp::mcp_router_with_evictor(sessions, Duration::from_secs(300));
    let state = Arc::new(
        AppState::new(
            config,
            Parts {
                forwarded_login: Arc::new(CoreLogin {
                    config: vtop_core::VtopConfig::default(),
                }),
                verifier: Arc::new(CoreVerifier {
                    config: vtop_core::VtopConfig::default(),
                }),
                gmail: Arc::new(GoogleOAuthRefresher::new(https(), DEFAULT_TOKEN_URL.into())),
            },
        )
        .with_mcp_evictor(mcp_evictor),
    );
    tracing::info!("starting vtop-engine on {listen}");

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
            eprintln!("vtop-engine: cannot bind {listen}: {error}");
            std::process::exit(1);
        }
    };
    let app = router(state).merge(mcp);
    if let Err(error) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        eprintln!("vtop-engine: {error}");
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

fn https() -> reqwest::Client {
    vtop_core::client::https_client(&[])
        .build()
        .expect("HTTP client")
}
