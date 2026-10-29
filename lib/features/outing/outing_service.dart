import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/backend/vtop_query.dart';
import 'package:vitapmate/vtop/vtop_page.dart';
import 'package:vitapmate/features/docs/docs.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

final outingServiceProvider = Provider<OutingService>(OutingService.new);

/// Outing forms, requests and passes, through the backend the mode says
/// (the server or the phone).
class OutingService {
  OutingService(this._ref);

  final Ref _ref;

  Future<GeneralOutingData> general() =>
      fetchUnsaved(_ref, VtopQueries.generalOuting());

  Future<WeekendOutingData> weekend() =>
      fetchUnsaved(_ref, VtopQueries.weekendOuting());

  /// Sends [action] once. It may have reached VTOP even when it fails (a
  /// dropped connection, a timeout), so the list it changes is reloaded to
  /// show what VTOP has.
  Future<T> _act<T>(VtopAction<T> action, {required bool weekend}) async {
    try {
      return await sendAction(_ref, action);
    } catch (_) {
      _ref.invalidate(weekend ? weekendOutingProvider : generalOutingProvider);
      rethrow;
    }
  }

  Future<OutingApplyResult> applyGeneral({
    required String place,
    required String purpose,
    required DateTime leaving,
    required DateTime returning,
    required String Function(DateTime) formatDate,
  }) => _act(
    VtopActions.applyGeneralOuting(
      place: place,
      purpose: purpose,
      outDate: formatDate(leaving),
      outHour: leaving.hour,
      outMinute: leaving.minute,
      inDate: formatDate(returning),
      inHour: returning.hour,
      inMinute: returning.minute,
    ),
    weekend: false,
  );

  Future<OutingApplyResult> applyWeekend({
    required String place,
    required String purpose,
    required String date,
    required String timeSlot,
    required String contactNumber,
  }) => _act(
    VtopActions.applyWeekendOuting(
      place: place,
      purpose: purpose,
      date: date,
      timeSlot: timeSlot,
      contactNumber: contactNumber,
    ),
    weekend: true,
  );

  /// Cancels a request VTOP still lets the student delete; [cancelId] is
  /// the record's `cancelId`.
  Future<OutingCancelResult> cancel({
    required bool weekend,
    required String cancelId,
  }) => _act(
    weekend
        ? VtopActions.cancelWeekendOuting(cancelId)
        : VtopActions.cancelGeneralOuting(cancelId),
    weekend: weekend,
  );

  /// The pass as a document in Docs, downloaded the first time only, so it
  /// opens offline afterwards (at the gate, say).
  Future<DocWindow> pass({
    required bool weekend,
    required String passId,
    required String name,
  }) async => keepInDocs(
    repository: await _ref.read(docsRepositoryProvider.future),
    docId: 'outing-pass-$passId',
    download: () async => (
      bytes: await fetchUnsaved(
        _ref,
        VtopQueries.outingPass(weekend: weekend, passId: passId),
      ),
      fileName: '$name.pdf',
    ),
  );
}

// `read`, not `watch`: the service never changes, and it reloads these
// after a failed action, which a watch would make a cycle.
final generalOutingProvider = FutureProvider.autoDispose<GeneralOutingData>(
  (ref) => ref.read(outingServiceProvider).general(),
);

final weekendOutingProvider = FutureProvider.autoDispose<WeekendOutingData>(
  (ref) => ref.read(outingServiceProvider).weekend(),
);
