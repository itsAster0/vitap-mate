// What a Gmail OTP connection keeps, and what setting one up reports.

class EmailOtpSetupResult {
  const EmailOtpSetupResult({
    required this.success,
    required this.message,
    this.email,
  });
  const EmailOtpSetupResult.ok(this.message, {this.email}) : success = true;
  const EmailOtpSetupResult.failed(this.message, {this.email})
    : success = false;

  final bool success;
  final String message;
  final String? email;
}

class LatestInfoEmail {
  const LatestInfoEmail({
    required this.receivedAt,
    required this.subject,
    required this.snippet,
    this.otp,
  });

  final DateTime receivedAt;
  final String subject;
  final String snippet;
  final String? otp;
}

enum EmailOtpAuthSource { sharedBuiltIn, personalByok }

class EmailOtpOAuthSession {
  const EmailOtpOAuthSession({
    required this.email,
    required this.accessToken,
    required this.refreshToken,
    required this.scopes,
    required this.accessTokenExpiryEpochMs,
    this.schemaVersion = 2,
    this.authSource = EmailOtpAuthSource.sharedBuiltIn,
    this.oauthClientId,
    this.oauthClientSecret,
  });

  final String email;
  final String accessToken;
  final String refreshToken;
  final List<String> scopes;
  final int accessTokenExpiryEpochMs;
  final int schemaVersion;
  final EmailOtpAuthSource authSource;
  final String? oauthClientId;
  final String? oauthClientSecret;

  String get authSourceLabel => switch (authSource) {
    EmailOtpAuthSource.personalByok => 'Personal BYOK',
    EmailOtpAuthSource.sharedBuiltIn => 'Shared fallback',
  };

  bool get hasGmailScope =>
      scopes.contains('https://www.googleapis.com/auth/gmail.modify');

  bool get isExpired => expiresWithin(const Duration(seconds: 30));

  /// True when the access token is expired or will be within [margin].
  bool expiresWithin(Duration margin) {
    final now = DateTime.now().toUtc().millisecondsSinceEpoch;
    return accessTokenExpiryEpochMs <= now + margin.inMilliseconds;
  }

  Map<String, dynamic> toJson() {
    return {
      'email': email,
      'accessToken': accessToken,
      'refreshToken': refreshToken,
      'scopes': scopes,
      'accessTokenExpiryEpochMs': accessTokenExpiryEpochMs,
      'schemaVersion': schemaVersion,
      'authSource': authSource.name,
      if (oauthClientId != null) 'oauthClientId': oauthClientId,
      if (oauthClientSecret != null) 'oauthClientSecret': oauthClientSecret,
    };
  }

  static EmailOtpOAuthSession? fromJson(Map<String, dynamic> raw) {
    final email = (raw['email'] as String?)?.trim();
    final accessToken = (raw['accessToken'] as String?)?.trim();
    final refreshToken = (raw['refreshToken'] as String?)?.trim();
    final expiry = raw['accessTokenExpiryEpochMs'];
    final scopesRaw = raw['scopes'];
    if (email == null ||
        email.isEmpty ||
        accessToken == null ||
        accessToken.isEmpty ||
        refreshToken == null ||
        refreshToken.isEmpty ||
        expiry is! int ||
        scopesRaw is! List<dynamic>) {
      return null;
    }
    final scopes = scopesRaw.whereType<String>().toList(growable: false);
    final sourceName = raw['authSource'] as String?;
    final authSource = EmailOtpAuthSource.values.firstWhere(
      (value) => value.name == sourceName,
      orElse: () => EmailOtpAuthSource.sharedBuiltIn,
    );
    return EmailOtpOAuthSession(
      email: email,
      accessToken: accessToken,
      refreshToken: refreshToken,
      scopes: scopes,
      accessTokenExpiryEpochMs: expiry,
      schemaVersion: raw['schemaVersion'] is int
          ? raw['schemaVersion'] as int
          : 1,
      authSource: authSource,
      oauthClientId: (raw['oauthClientId'] as String?)?.trim(),
      oauthClientSecret: (raw['oauthClientSecret'] as String?)?.trim(),
    );
  }

  EmailOtpOAuthSession copyWith({
    String? accessToken,
    String? refreshToken,
    List<String>? scopes,
    int? accessTokenExpiryEpochMs,
    int? schemaVersion,
    EmailOtpAuthSource? authSource,
    String? oauthClientId,
    String? oauthClientSecret,
  }) {
    return EmailOtpOAuthSession(
      email: email,
      accessToken: accessToken ?? this.accessToken,
      refreshToken: refreshToken ?? this.refreshToken,
      scopes: scopes ?? this.scopes,
      accessTokenExpiryEpochMs:
          accessTokenExpiryEpochMs ?? this.accessTokenExpiryEpochMs,
      schemaVersion: schemaVersion ?? this.schemaVersion,
      authSource: authSource ?? this.authSource,
      oauthClientId: oauthClientId ?? this.oauthClientId,
      oauthClientSecret: oauthClientSecret ?? this.oauthClientSecret,
    );
  }
}
