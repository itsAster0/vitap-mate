/// URLs of the services the app talks to. The VITAP Mate server has a
/// built-in default, which a build can change through `.env.json`
/// (`--dart-define-from-file=.env.json`) and the user in Settings → Privacy
/// → Server (see `core/providers/server_url.dart`).
library;

/// The VITAP Mate server: installs, the event channel, OTP forwarding,
/// sign-in for connected apps, and VTOP data.
const defaultServerUrl = String.fromEnvironment(
  'VTOP_SERVER_URL',
  defaultValue: 'https://vmate-api.aster0.dev',
);

/// Where students download the browser extension.
const extensionDownloadUrl = String.fromEnvironment('VTOP_EXTENSION_URL');
