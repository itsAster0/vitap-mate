sealed class VtopUserEntity {
  const VtopUserEntity();

  const factory VtopUserEntity.unconfigured() = UnconfiguredVtopUser._;

  factory VtopUserEntity.configured({
    required String username,
    required String password,
    required String semesterId,
    String? registrationNumber,
  }) {
    return ConfiguredVtopUser._(
      username: _requiredValue(username, 'username'),
      password: _requiredSecret(password),
      semesterId: _requiredValue(semesterId, 'semesterId'),
      registrationNumber: _optional(registrationNumber),
    );
  }

  factory VtopUserEntity.credentialsRejected({
    required String username,
    required String password,
    required String semesterId,
    String? registrationNumber,
  }) {
    return RejectedVtopUser._(
      username: _requiredValue(username, 'username'),
      password: _requiredSecret(password),
      semesterId: _requiredValue(semesterId, 'semesterId'),
      registrationNumber: _optional(registrationNumber),
    );
  }

  factory VtopUserEntity.fromJson(Map<String, dynamic> json) {
    final state = json['state'];
    if (state == 'unconfigured') return const VtopUserEntity.unconfigured();

    final username = (json['username'] as String?)?.trim() ?? '';
    final password = json['password'] as String? ?? '';
    final semesterId =
        (json['semesterId'] as String? ?? json['semid'] as String?)?.trim() ??
        '';
    if (username.isEmpty || password.trim().isEmpty || semesterId.isEmpty) {
      return const VtopUserEntity.unconfigured();
    }
    final registrationNumber = _optional(json['registrationNumber'] as String?);

    final rejected = state == 'credentialsRejected' || json['isValid'] == false;
    return rejected
        ? VtopUserEntity.credentialsRejected(
            username: username,
            password: password,
            semesterId: semesterId,
            registrationNumber: registrationNumber,
          )
        : VtopUserEntity.configured(
            username: username,
            password: password,
            semesterId: semesterId,
            registrationNumber: registrationNumber,
          );
  }

  String? get username;
  String? get password;
  String? get semid;

  /// The student's VTOP registration number, which can differ from the
  /// login [username]. Unknown (null) until a sign-in provides it; saved
  /// with the account so the app need not sign in just to learn it.
  String? get registrationNumber;

  bool get isValid => this is ConfiguredVtopUser;
  bool get isConfigured => this is! UnconfiguredVtopUser;

  VtopUserEntity withSemester(String semesterId);
  VtopUserEntity withCredentials(String username, String password);

  /// The same account with its registration number now known.
  VtopUserEntity withRegistrationNumber(String registrationNumber);
  VtopUserEntity rejectCredentials();
  Map<String, dynamic> toJson();

  static String _requiredValue(String value, String name) {
    final trimmed = value.trim();
    if (trimmed.isEmpty) {
      throw ArgumentError.value(value, name, 'must not be empty');
    }
    return trimmed;
  }

  static String _requiredSecret(String value) {
    if (value.trim().isEmpty) {
      throw ArgumentError.value(value, 'password', 'must not be empty');
    }
    return value;
  }

  static String? _optional(String? value) {
    final trimmed = value?.trim();
    return (trimmed == null || trimmed.isEmpty) ? null : trimmed;
  }
}

final class UnconfiguredVtopUser extends VtopUserEntity {
  const UnconfiguredVtopUser._();

  @override
  String? get username => null;
  @override
  String? get password => null;
  @override
  String? get semid => null;
  @override
  String? get registrationNumber => null;

  @override
  VtopUserEntity withSemester(String semesterId) => this;
  @override
  VtopUserEntity withCredentials(String username, String password) => this;
  @override
  VtopUserEntity withRegistrationNumber(String registrationNumber) => this;
  @override
  VtopUserEntity rejectCredentials() => this;
  @override
  Map<String, dynamic> toJson() => const {'state': 'unconfigured'};
}

sealed class StoredVtopUser extends VtopUserEntity {
  const StoredVtopUser({
    required this.username,
    required this.password,
    required String semesterId,
    required this.registrationNumber,
  }) : semid = semesterId;

  @override
  final String username;
  @override
  final String password;
  @override
  final String semid;
  @override
  final String? registrationNumber;
}

final class ConfiguredVtopUser extends StoredVtopUser {
  const ConfiguredVtopUser._({
    required super.username,
    required super.password,
    required super.semesterId,
    required super.registrationNumber,
  });

  @override
  VtopUserEntity withSemester(String semesterId) => VtopUserEntity.configured(
    username: username,
    password: password,
    semesterId: semesterId,
    registrationNumber: registrationNumber,
  );
  @override
  VtopUserEntity withCredentials(String username, String password) =>
      VtopUserEntity.configured(
        username: username,
        password: password,
        semesterId: semid,
      );
  @override
  VtopUserEntity withRegistrationNumber(String registrationNumber) =>
      VtopUserEntity.configured(
        username: username,
        password: password,
        semesterId: semid,
        registrationNumber: registrationNumber,
      );
  @override
  VtopUserEntity rejectCredentials() => VtopUserEntity.credentialsRejected(
    username: username,
    password: password,
    semesterId: semid,
    registrationNumber: registrationNumber,
  );
  @override
  Map<String, dynamic> toJson() => {
    'state': 'configured',
    'username': username,
    'password': password,
    'semesterId': semid,
    if (registrationNumber != null) 'registrationNumber': registrationNumber,
  };
}

final class RejectedVtopUser extends StoredVtopUser {
  const RejectedVtopUser._({
    required super.username,
    required super.password,
    required super.semesterId,
    required super.registrationNumber,
  });

  @override
  VtopUserEntity withSemester(String semesterId) =>
      VtopUserEntity.credentialsRejected(
        username: username,
        password: password,
        semesterId: semesterId,
        registrationNumber: registrationNumber,
      );
  @override
  VtopUserEntity withCredentials(String username, String password) =>
      VtopUserEntity.configured(
        username: username,
        password: password,
        semesterId: semid,
      );
  @override
  VtopUserEntity withRegistrationNumber(String registrationNumber) =>
      VtopUserEntity.credentialsRejected(
        username: username,
        password: password,
        semesterId: semid,
        registrationNumber: registrationNumber,
      );
  @override
  VtopUserEntity rejectCredentials() => this;
  @override
  Map<String, dynamic> toJson() => {
    'state': 'credentialsRejected',
    'username': username,
    'password': password,
    'semesterId': semid,
    if (registrationNumber != null) 'registrationNumber': registrationNumber,
  };
}
