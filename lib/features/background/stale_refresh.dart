import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/features/attendance/presentation/providers/attendance_provider.dart';
import 'package:vitapmate/features/attendance/presentation/providers/full_attendance_provider.dart';
import 'package:vitapmate/features/attendance/presentation/providers/state/attendance_repository.dart';
import 'package:vitapmate/features/background/sync.dart';
import 'package:vitapmate/features/more/presentation/providers/exam_schedule.dart';
import 'package:vitapmate/features/more/presentation/providers/marks_provider.dart';
import 'package:vitapmate/features/more/presentation/providers/state/exam_schedule.dart';
import 'package:vitapmate/features/settings/presentation/providers/semester_id_provider.dart';
import 'package:vitapmate/features/settings/presentation/providers/state/semester_id.dart';
import 'package:vitapmate/features/timetable/presentation/providers/state/timetable_repo.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

/// Pages older than this are refetched after a quick fetch in the open app.
const openAppRefreshMaxAge = Duration(hours: 12);

final staleRefresherProvider = Provider<StaleRefresher>((ref) {
  final refresher = StaleRefresher(ref);
  final quickFetches = ref
      .read(globalAsyncQueueProvider.notifier)
      .quickFetches
      .listen((_) => unawaited(refresher.onQuickFetch()));
  ref.onDispose(quickFetches.cancel);
  return refresher;
});

/// Refreshes stale VTOP pages in the open app, one at a time, while the user
/// has nothing running. A quick fetch starts it: the session is live and the
/// network is good. It stops rather than log in again, so it never asks for
/// an OTP. Closing the app drops the rest; the next open finishes it.
class StaleRefresher {
  StaleRefresher(this._ref);

  final Ref _ref;
  bool _running = false;
  DateTime? _lastRun;

  static const _cooldown = Duration(minutes: 15);

  /// The user's requests go first; wait this long after the last one.
  static const _quietPeriod = Duration(seconds: 2);

  /// A page slower than this means the network got worse: stop, so the
  /// refresh never competes with the user on a slow connection.
  static const _slowPage = Duration(seconds: 5);

  /// Stop this long before the session would need a new login.
  static const _sessionMargin = Duration(minutes: 5);

  /// VTOP ends a session after about this long, whatever the app setting.
  static const _vtopSessionLength = Duration(minutes: 30);

  Future<void> onQuickFetch() async {
    if (_running) return;
    if (_lastRun != null && DateTime.now().difference(_lastRun!) < _cooldown) {
      return;
    }
    if (WidgetsBinding.instance.lifecycleState != AppLifecycleState.resumed) {
      return;
    }
    await _ref.read(settingsProvider.future);
    if (!_ref.read(autoRefreshProvider)) return;
    _running = true;
    _lastRun = DateTime.now();
    try {
      await refresh();
    } catch (error) {
      // Saved data stays as it was; the next quick fetch tries again.
      AppLogger.instance.info('sync.stale', 'refresh stopped: $error');
    } finally {
      _running = false;
    }
  }

  /// Refetches every page older than [maxAge], most used first.
  Future<void> refresh({Duration maxAge = openAppRefreshMaxAge}) async {
    final user = await _ref.read(vtopUserProvider.future);
    if (!user.isValid || user.semid == null) return;

    Future<void> step(BigInt? updatedAt, Future<void> Function() update) async {
      if (updatedAt != null && isVtopDataFresh(updatedAt, maxAge)) return;
      await _waitForIdle();
      if (!await _sessionLive()) {
        throw StateError('session needs a new login');
      }
      final watch = Stopwatch()..start();
      await update();
      if (watch.elapsed > _slowPage) {
        throw StateError('network is slow (${watch.elapsed.inSeconds}s page)');
      }
    }

    final attendanceRepo = await _ref.read(attendanceRepositoryProvider.future);
    await step(
      (await attendanceRepo.loadCache())?.updateTime,
      () => _ref.read(attendanceProvider.notifier).updateAttendance(),
    );
    await step(
      (await (await _ref.read(
        timetableRepositoryProvider.future,
      )).loadCache())?.updateTime,
      () => _ref.read(timetableProvider.notifier).updateTimetable(),
    );
    await step(
      (await (await _ref.read(
        marksRepositoryProvider.future,
      )).loadCache())?.updateTime,
      () => _ref.read(marksProvider.notifier).updatemarks(),
    );
    await step(
      (await (await _ref.read(
        examScheduleRepositoryProvider.future,
      )).loadCache())?.updateTime,
      () => _ref.read(examScheduleProvider.notifier).updatexamschedule(),
    );
    await step(
      (await (await _ref.read(
        semidRepositoryProvider.future,
      )).loadCache())?.updateTime,
      () => _ref.read(semesterIdProvider.notifier).updatesemids(),
    );
    for (final course in (await attendanceRepo.loadCache())?.records ?? []) {
      final repo = await _ref.read(
        fullAttendanceRepositoryProvider(
          course.courseType,
          course.courseId,
        ).future,
      );
      await step(
        (await repo.loadCache())?.updateTime,
        () => _ref
            .read(
              fullAttendanceProvider(
                course.courseType,
                course.courseId,
              ).notifier,
            )
            .updateAttendance(),
      );
    }
  }

  /// Returns at once when no VTOP request is running. Otherwise the user is
  /// busy: wait for their requests, then [_quietPeriod] in case more follow.
  Future<void> _waitForIdle() async {
    final queue = _ref.read(globalAsyncQueueProvider.notifier);
    bool busy() => _ref
        .read(globalAsyncQueueProvider)
        .running
        .keys
        .any((key) => key.startsWith('vtop'));
    while (busy()) {
      await queue.taskStream.firstWhere(
        (keys) => !keys.any((key) => key.startsWith('vtop')),
      );
      await Future<void>.delayed(_quietPeriod);
    }
  }

  /// True while the session can serve requests without a new login, which
  /// could ask the user for an OTP they did not start.
  Future<bool> _sessionLive() async {
    final client = await _ref.read(vClientProvider.future);
    if (!await fetchIsAuth(client: client)) return false;
    final loggedInAt = exportSessionState(client: client).loggedInAt;
    if (loggedInAt == null) return true;
    final age = DateTime.now().toUtc().difference(
      DateTime.fromMillisecondsSinceEpoch(
        loggedInAt.toInt() * 1000,
        isUtc: true,
      ),
    );
    final reuse = _ref.read(vtopSessionReuseTtlProvider);
    final limit = reuse < _vtopSessionLength ? reuse : _vtopSessionLength;
    return age < limit - _sessionMargin;
  }
}
