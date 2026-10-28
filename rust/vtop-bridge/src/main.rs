use std::sync::Arc;
use std::time::Duration;

use tracing_subscriber::EnvFilter;
use vtop_bridge::config::Config;
use vtop_bridge::engine::HttpEdge;
use vtop_bridge::gmail::{GoogleOAuthRefresher, DEFAULT_TOKEN_URL};
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
            eprintln!("vtop-engine: {error}");
            std::process::exit(2);
        }
    };
    let listen = config.listen;
    let relay_secret = config.mail_relay_secret.clone();
    let edge = Arc::new(HttpEdge {
        http: reqwest::Client::new(),
        base_url: config.edge_url.clone(),
        secret: config.mail_relay_secret.clone(),
    });
    // The MCP tools run here too, with sessions from the edge under each
    // caller's own key; the edge sends `/mcp` on to this service.
    // Sessions are kept a few minutes, so a run of tool calls asks the edge
    // (and VTOP, to check it) once.
    let sessions = vtop_mcp::bridge::CachedSessions::new(
        Arc::new(vtop_mcp::bridge::BridgeClient::new(
            reqwest::Client::new(),
            config.edge_url.clone(),
        )),
        Duration::from_secs(300),
    );
    let mcp = vtop_mcp::mcp_router(Arc::new(sessions), Duration::from_secs(300));
    let state = Arc::new(AppState::new(
        config,
        Parts {
            forwarded_login: Arc::new(CoreLogin {
                config: vtop_core::VtopConfig::default(),
            }),
            verifier: Arc::new(CoreVerifier {
                config: vtop_core::VtopConfig::default(),
            }),
            edge,
            gmail: Arc::new(GoogleOAuthRefresher::new(
                reqwest::Client::new(),
                DEFAULT_TOKEN_URL.into(),
            )),
        },
    ));
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
    // UDP mode's gateway, where GATEWAY_KEY is set (on Fly): it answers
    // phones over UDP and asks this engine for pages over loopback.
    match vtop_gateway::bind_from_env(format!("http://127.0.0.1:{}", listen.port()), relay_secret)
        .await
    {
        None => {}
        Some(Ok((gateway, sockets))) => {
            tokio::spawn(vtop_gateway::run(gateway, sockets));
        }
        Some(Err(error)) => {
            eprintln!("vtop-engine: UDP gateway: {error}");
            std::process::exit(2);
        }
    }
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
