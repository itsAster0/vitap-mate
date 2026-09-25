/// Where a vtop-server lives, the key to use with it (if it has one) and
/// whether to use it. Turning the server off keeps the URL and key, so it
/// can be switched back on without typing them again. An empty key suits a
/// server that runs without `VTOP_SERVER_API_KEYS`.
class VtopServerSettings {
  const VtopServerSettings({
    required this.url,
    required this.apiKey,
    this.enabled = true,
  });

  static const disabled = VtopServerSettings(
    url: '',
    apiKey: '',
    enabled: false,
  );

  final String url;
  final String apiKey;

  /// The switch in Settings. The server is only used when this is on and a
  /// URL is saved.
  final bool enabled;

  bool get isEnabled => enabled && hasUrl;

  bool get hasUrl => url.trim().isNotEmpty;

  VtopServerSettings copyWith({String? url, String? apiKey, bool? enabled}) =>
      VtopServerSettings(
        url: url ?? this.url,
        apiKey: apiKey ?? this.apiKey,
        enabled: enabled ?? this.enabled,
      );

  bool get hasApiKey => apiKey.trim().isNotEmpty;

  /// Request headers for this server: JSON, plus the key when there is one.
  Map<String, String> get headers => {
    'content-type': 'application/json',
    if (hasApiKey) 'x-api-key': apiKey.trim(),
  };

  /// `https://host/base` + `/v1/<path>`.
  Uri endpoint(String path) {
    final base = url.trim().replaceAll(RegExp(r'/+$'), '');
    return Uri.parse('$base/v1/$path');
  }

  /// Returns an error message, or null when [url] is usable.
  static String? validateUrl(String url) {
    final trimmed = url.trim();
    if (trimmed.isEmpty) return null;
    final uri = Uri.tryParse(trimmed);
    if (uri == null || !uri.hasScheme || uri.host.isEmpty) {
      return 'Enter a full URL, like https://vtop.example.com';
    }
    final local =
        uri.host == 'localhost' ||
        uri.host == '127.0.0.1' ||
        uri.host == '10.0.2.2';
    if (uri.scheme != 'https' && !(uri.scheme == 'http' && local)) {
      return 'Use https (http only works for a local test server)';
    }
    return null;
  }
}
