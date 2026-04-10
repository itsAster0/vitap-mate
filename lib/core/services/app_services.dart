import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/models/account.dart';
import 'package:vitapmate/core/models/log_entry.dart';
import 'package:vitapmate/core/services/app_logger.dart';
import 'package:vitapmate/core/services/gmail_oauth_service.dart';
import 'package:vitapmate/core/services/storage.dart';
import 'package:vitapmate/core/utils/registration_number.dart';
import 'package:vitapmate/src/api/logging.dart' as rust_logging;
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart' as bridge;

class VtopGateway {
  VtopGateway(this._logger, this._queue);

  final AppLogger _logger;
  final AsyncQueue _queue;
  StreamSubscription<rust_logging.BridgeLogEntry>? _logSubscription;

  void bindRustLogs() {
    _logSubscription ??= rust_logging.createLogStream().listen((entry) {
      _logger.log(
        switch (entry.level) {
          rust_logging.BridgeLogLevel.trace => LogLevel.trace,
          rust_logging.BridgeLogLevel.debug => LogLevel.debug,
          rust_logging.BridgeLogLevel.info => LogLevel.info,
          rust_logging.BridgeLogLevel.warn => LogLevel.warn,
          rust_logging.BridgeLogLevel.error => LogLevel.error,
        },
        entry.source,
        entry.message,
      );
    });
  }

  VtopClient createClient({
    required String registrationNumber,
    required String password,
    String? cookie,
  }) {
    _logger.info('bridge', 'Creating VTOP client for $registrationNumber');
    return bridge.getVtopClient(
      username: registrationNumber,
      password: password,
      cookie: cookie,
    );
  }

  Future<void> login(VtopClient client, String username) {
    _logger.info('vtop_client', 'Attempting VTOP login for $username');
    return _queue.run(
      'vtop_login_$username',
      () => bridge.vtopClientLogin(client: client),
    );
  }

  Future<bool> isAuthenticated(VtopClient client) {
    return bridge.fetchIsAuth(client: client);
  }

  Future<SemesterData> fetchSemesters(VtopClient client) {
    _logger.debug('bridge', 'Fetching semesters');
    return bridge.fetchSemesters(client: client);
  }

  Future<String?> fetchCookie(VtopClient client) async {
    final bytes = await bridge.fetchCookies(client: client);
    if (bytes.isEmpty) return null;
    return utf8.decode(bytes).split(';').first.trim();
  }
}

class SessionCoordinator {
  SessionCoordinator({
    required this.logger,
    required this.preferenceStore,
    required this.gateway,
  });

  final AppLogger logger;
  final PreferenceStore preferenceStore;
  final VtopGateway gateway;
  VtopClient? _client;

  void clearClient() {
    _client = null;
  }

  Future<VtopClient> ensureAuthenticated({
    required ActiveAccount account,
    required String password,
    bool force = false,
  }) async {
    if (force) {
      clearClient();
      await preferenceStore.writeCookie(null);
    }
    final client =
        _client ??= gateway.createClient(
          registrationNumber: account.identity.registrationNumber,
          password: password,
          cookie: preferenceStore.readCookie(),
        );
    if (!force && await gateway.isAuthenticated(client)) {
      return client;
    }
    await gateway.login(client, account.identity.registrationNumber);
    final cookie = await gateway.fetchCookie(client);
    await preferenceStore.writeCookie(cookie);
    logger.info(
      'session',
      'Authenticated VTOP session for ${account.identity.registrationNumber}',
    );
    return client;
  }
}

class AuthRepository {
  AuthRepository(this.services);

  final AppServices services;

  Future<ActiveAccount?> loadActiveAccount() {
    return services.secureAccountStore.readActiveAccount();
  }

  Future<String> requirePassword() async {
    final password = await services.secureAccountStore.readPassword();
    if (password == null || password.isEmpty) {
      throw StateError('No VTOP password is stored for the active account.');
    }
    return password;
  }

  Future<String> signInWithGoogleEmail() async {
    return services.gmailOAuthService.authorizeForGmail();
  }

  Future<String> reauthorizeGoogleForGmail() {
    return signInWithGoogleEmail();
  }

  Future<PendingSignInResult> verifyUsernameAndPassword({
    String? email,
    required String username,
    required String password,
  }) async {
    final registrationNumber =
        username.contains('@') ? deriveRegistrationNumber(username) : username;
    final client = services.gateway.createClient(
      registrationNumber: registrationNumber.trim().toLowerCase(),
      password: password,
      cookie: null,
    );
    await services.gateway.login(client, registrationNumber);
    final semesters = await services.gateway.fetchSemesters(client);
    final cookie = await services.gateway.fetchCookie(client) ?? '';
    return PendingSignInResult(
      email: email,
      registrationNumber: registrationNumber,
      password: password,
      semesters: semesters,
      cookie: cookie,
    );
  }

  Future<void> saveVerifiedAccount({
    required PendingSignInResult pending,
    required String selectedSemesterId,
  }) async {
    final account = ActiveAccount(
      identity: StudentIdentity(
        email: pending.email,
        registrationNumber: pending.registrationNumber,
        selectedSemesterId: selectedSemesterId,
      ),
    );
    await services.secureAccountStore.writeActiveAccount(
      account: account,
      password: pending.password,
    );
    await services.preferenceStore.writeCookie(pending.cookie);
    services.sessionCoordinator.clearClient();
    services.logger.info(
      'auth',
      'Saved active account for ${pending.registrationNumber}',
    );
  }

  Future<void> updateActiveAccount({
    required String selectedSemesterId,
    String? newPassword,
  }) async {
    final account = await loadActiveAccount();
    if (account == null) {
      throw StateError('No active VTOP account is available.');
    }
    final password =
        newPassword != null && newPassword.trim().isNotEmpty
            ? newPassword.trim()
            : await requirePassword();
    final client = services.gateway.createClient(
      registrationNumber: account.identity.registrationNumber,
      password: password,
      cookie: null,
    );
    await services.gateway.login(client, account.identity.registrationNumber);
    final updated = account.copyWith(
      identity: StudentIdentity(
        email: account.identity.email,
        registrationNumber: account.identity.registrationNumber,
        selectedSemesterId: selectedSemesterId,
      ),
      isValid: true,
    );
    await services.secureAccountStore.writeActiveAccount(
      account: updated,
      password: password,
    );
    await services.preferenceStore.writeCookie(
      await services.gateway.fetchCookie(client),
    );
    services.sessionCoordinator.clearClient();
  }

  Future<void> signOut() async {
    await services.secureAccountStore.clear();
    await services.preferenceStore.writeCookie(null);
    await services.cacheStore.clearPrefix('');
    services.sessionCoordinator.clearClient();
    services.logger.info('auth', 'Signed out active account');
  }
}

Map<String, dynamic> _asMap(dynamic value) {
  return (value as Map).map(
    (key, innerValue) => MapEntry(key.toString(), innerValue),
  );
}

class VtopDataRepository {
  VtopDataRepository(this.services);

  final AppServices services;

  Future<ActiveAccount> _requireAccount() async {
    final account = await services.authRepository.loadActiveAccount();
    if (account == null) {
      throw StateError('No active VTOP account is available.');
    }
    return account;
  }

  Future<VtopClient> _requireClient() async {
    final account = await _requireAccount();
    final password = await services.authRepository.requirePassword();
    return services.sessionCoordinator.ensureAuthenticated(
      account: account,
      password: password,
    );
  }

  String _key(ActiveAccount account, String section, [String? suffix]) {
    final identity = account.identity;
    final base =
        '${identity.registrationNumber}_${identity.selectedSemesterId}_$section';
    return suffix == null ? base : '${base}_$suffix';
  }

  Future<T> _load<T>({
    required String section,
    String? suffix,
    required T Function(Map<String, dynamic>) fromJson,
    required T empty,
    required Future<T> Function(VtopClient client, ActiveAccount account) fetch,
    bool refresh = false,
  }) async {
    final account = await _requireAccount();
    final key = _key(account, section, suffix);
    if (!refresh) {
      final cached = await services.cacheStore.read(key);
      if (cached != null) return fromJson(_asMap(cached));
    }
    final client = await _requireClient();
    final data = await fetch(client, account);
    await services.cacheStore.write(key, (data as dynamic).toJson());
    return data;
  }

  Future<SemesterData> loadSemesters({bool refresh = false}) async {
    final account = await _requireAccount();
    final key = '${account.identity.registrationNumber}_semesters';
    if (!refresh) {
      final cached = await services.cacheStore.read(key);
      if (cached != null) return SemesterData.fromJson(_asMap(cached));
    }
    final client = await _requireClient();
    final data = await bridge.fetchSemesters(client: client);
    await services.cacheStore.write(key, data.toJson());
    return data;
  }

  Future<TimetableData> loadTimetable({bool refresh = false}) {
    return _load(
      section: 'timetable',
      fromJson: TimetableData.fromJson,
      empty: TimetableData(
        slots: const [],
        semesterId: '',
        updateTime: BigInt.zero,
      ),
      refresh: refresh,
      fetch:
          (client, account) => bridge.fetchTimetable(
            client: client,
            semesterId: account.identity.selectedSemesterId,
          ),
    );
  }

  Future<AttendanceData> loadAttendance({bool refresh = false}) {
    return _load(
      section: 'attendance',
      fromJson: AttendanceData.fromJson,
      empty: AttendanceData(
        records: const [],
        semesterId: '',
        updateTime: BigInt.zero,
      ),
      refresh: refresh,
      fetch:
          (client, account) => bridge.fetchAttendance(
            client: client,
            semesterId: account.identity.selectedSemesterId,
          ),
    );
  }

  Future<FullAttendanceData> loadFullAttendance({
    required String courseType,
    required String courseId,
    bool refresh = false,
  }) {
    return _load(
      section: 'full_attendance',
      suffix: '${courseType}_$courseId',
      fromJson: FullAttendanceData.fromJson,
      empty: FullAttendanceData(
        records: const [],
        semesterId: '',
        updateTime: BigInt.zero,
        courseId: courseId,
        courseType: courseType,
      ),
      refresh: refresh,
      fetch:
          (client, account) => bridge.fetchFullAttendance(
            client: client,
            semesterId: account.identity.selectedSemesterId,
            courseId: courseId,
            courseType: courseType,
          ),
    );
  }

  Future<MarksData> loadMarks({bool refresh = false}) {
    return _load(
      section: 'marks',
      fromJson: MarksData.fromJson,
      empty: MarksData(
        records: const [],
        semesterId: '',
        updateTime: BigInt.zero,
      ),
      refresh: refresh,
      fetch:
          (client, account) => bridge.fetchMarks(
            client: client,
            semesterId: account.identity.selectedSemesterId,
          ),
    );
  }

  Future<ExamScheduleData> loadExamSchedule({bool refresh = false}) {
    return _load(
      section: 'exam_schedule',
      fromJson: ExamScheduleData.fromJson,
      empty: ExamScheduleData(
        exams: const [],
        semesterId: '',
        updateTime: BigInt.zero,
      ),
      refresh: refresh,
      fetch:
          (client, account) => bridge.fetchExamShedule(
            client: client,
            semesterId: account.identity.selectedSemesterId,
          ),
    );
  }

  Future<GradeViewData> loadGradesForSemester(
    String semesterId, {
    bool refresh = false,
  }) async {
    final account = await _requireAccount();
    final key = '${account.identity.registrationNumber}_${semesterId}_grades';
    if (!refresh) {
      final cached = await services.cacheStore.read(key);
      if (cached != null) return GradeViewData.fromJson(_asMap(cached));
    }
    final client = await _requireClient();
    final data = await bridge.fetchGradeView(
      client: client,
      semesterId: semesterId,
    );
    await services.cacheStore.write(key, data.toJson());
    return data;
  }

  Future<GradeDetailsData> loadGradeDetailsForSemester({
    required String semesterId,
    required String courseId,
    bool refresh = false,
  }) async {
    final account = await _requireAccount();
    final key =
        '${account.identity.registrationNumber}_${semesterId}_grade_details_$courseId';
    if (!refresh) {
      final cached = await services.cacheStore.read(key);
      if (cached != null) return GradeDetailsData.fromJson(_asMap(cached));
    }
    final client = await _requireClient();
    final data = await bridge.fetchGradeViewDetails(
      client: client,
      semesterId: semesterId,
      courseId: courseId,
    );
    await services.cacheStore.write(key, data.toJson());
    return data;
  }

  Future<GradeHistoryData> loadGradeHistory({bool refresh = false}) async {
    final account = await _requireAccount();
    final key = '${account.identity.registrationNumber}_grade_history';
    if (!refresh) {
      final cached = await services.cacheStore.read(key);
      if (cached != null) return GradeHistoryData.fromJson(_asMap(cached));
    }
    final client = await _requireClient();
    final data = await bridge.fetchGradeHistory(client: client);
    await services.cacheStore.write(key, data.toJson());
    return data;
  }
}

class AppServices {
  AppServices({
    required this.logger,
    required this.secureAccountStore,
    required this.preferenceStore,
    required this.cacheStore,
    required this.gmailOAuthService,
    required this.gateway,
    required this.sessionCoordinator,
  });

  final AppLogger logger;
  final SecureAccountStore secureAccountStore;
  final PreferenceStore preferenceStore;
  final JsonCacheStore cacheStore;
  final GmailOAuthService gmailOAuthService;
  final VtopGateway gateway;
  final SessionCoordinator sessionCoordinator;
  late final AuthRepository authRepository = AuthRepository(this);
  late final VtopDataRepository vtopDataRepository = VtopDataRepository(this);

  static Future<AppServices> create({AsyncQueue? queue}) async {
    final appSupportDirectory = await getApplicationSupportDirectory();
    final cacheDirectory = await getTemporaryDirectory();
    final logger = AppLogger(File('${cacheDirectory.path}/logs/app_logs.json'));
    await logger.initialize();
    final prefs = await SharedPreferences.getInstance();
    final resolvedQueue = queue ?? DedupingAsyncQueue();
    final secureAccountStore = SecureAccountStore(const FlutterSecureStorage());
    final gateway = VtopGateway(logger, resolvedQueue);
    gateway.bindRustLogs();
    return AppServices(
      logger: logger,
      secureAccountStore: secureAccountStore,
      preferenceStore: PreferenceStore(prefs),
      cacheStore: JsonCacheStore(
        Directory('${appSupportDirectory.path}/cache'),
      ),
      gmailOAuthService: GmailOAuthService(logger, secureAccountStore),
      gateway: gateway,
      sessionCoordinator: SessionCoordinator(
        logger: logger,
        preferenceStore: PreferenceStore(prefs),
        gateway: gateway,
      ),
    );
  }
}

class DedupingAsyncQueue implements AsyncQueue {
  final Map<String, Future<Object?>> _running = {};

  @override
  Future<T> run<T>(String id, Future<T> Function() task) async {
    final existing = _running[id];
    if (existing != null) {
      return existing as Future<T>;
    }

    if (id.startsWith('vtop')) {
      final loginTasks =
          _running.entries
              .where((entry) => entry.key.startsWith('vtop_login'))
              .map((entry) => entry.value)
              .toList();
      if (loginTasks.isNotEmpty) {
        await Future.wait(loginTasks);
      }
    }

    late final Future<T> future;
    future = Future<T>.sync(task).whenComplete(() {
      _running.remove(id);
    });
    _running[id] = future;
    return future;
  }
}
