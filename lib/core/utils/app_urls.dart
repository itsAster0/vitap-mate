/// URLs of the services the app talks to. Each can be overridden at build
/// time through `.env.json` (`--dart-define-from-file=.env.json`).
library;

/// The sign-in bridge for the browser extension and AI agents.
const vtopBridgeBaseUrl = String.fromEnvironment(
  'VTOP_BRIDGE_URL',
  defaultValue: 'https://vtop-bridge.aster0.dev',
);

/// Where students download the browser extension.
const extensionDownloadUrl = String.fromEnvironment(
  'VTOP_EXTENSION_URL',
  defaultValue:
      'https://github.com/itsKryxen/vitap-mate/releases/tag/extension',
);
