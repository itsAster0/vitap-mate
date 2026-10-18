/// URLs of the services the app talks to, set at build time through
/// `.env.json` (`--dart-define-from-file=.env.json`). None is built in: a
/// build without one leaves that feature unavailable.
library;

/// The sign-in bridge for the browser extension and AI agents.
const vtopBridgeBaseUrl = String.fromEnvironment('VTOP_BRIDGE_URL');

/// The Cloudflare edge: forwarding addresses, installs and the event
/// channel (`/v1/events`).
const vtopEdgeBaseUrl = String.fromEnvironment('VTOP_EDGE_URL');

/// Where students download the browser extension.
const extensionDownloadUrl = String.fromEnvironment('VTOP_EXTENSION_URL');
