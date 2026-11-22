//! What reaches our servers: the edge's URL and the session cookies sent to it, the routes to
//! VTOP (Proxy, Server, UDP), cookie consent, the account token, Connected apps, mail forwarding
//! and the Server page.

pub(crate) mod account_token;
pub(crate) mod consent;
pub(crate) mod edge;
pub(crate) mod forwarding;
pub(crate) mod keys;
pub(crate) mod route;
pub(crate) mod server;
pub(crate) mod udp;
