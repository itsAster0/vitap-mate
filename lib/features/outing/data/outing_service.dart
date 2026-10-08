import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/utils/vtop_client_call.dart';
import 'package:vitapmate/features/docs/data/doc_models.dart';
import 'package:vitapmate/features/docs/data/download_to_docs.dart';
import 'package:vitapmate/features/docs/presentation/providers/docs_provider.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart' as vtop_api;

final outingServiceProvider = Provider<OutingService>(OutingService.new);

/// Outing requests go straight from the phone to VTOP, even when a
/// vtop-server is configured: they change things on VTOP, and the device
/// already holds the session.
class OutingService {
  OutingService(this._ref);

  final Ref _ref;

  /// An expired session is caught while loading the form, before anything
  /// is posted, so the retry in [withVtopClient] is safe for applications.
  Future<T> _run<T>(Future<T> Function(VtopClient client) call) =>
      withVtopClient(_ref, call);

  Future<GeneralOutingData> general() =>
      _run((client) => vtop_api.fetchGeneralOuting(client: client));

  Future<WeekendOutingData> weekend() =>
      _run((client) => vtop_api.fetchWeekendOuting(client: client));

  Future<OutingApplyResult> applyGeneral({
    required String place,
    required String purpose,
    required DateTime leaving,
    required DateTime returning,
    required String Function(DateTime) formatDate,
  }) => _run(
    (client) => vtop_api.applyGeneralOuting(
      client: client,
      place: place,
      purpose: purpose,
      outDate: formatDate(leaving),
      outHour: leaving.hour,
      outMinute: leaving.minute,
      inDate: formatDate(returning),
      inHour: returning.hour,
      inMinute: returning.minute,
    ),
  );

  Future<OutingApplyResult> applyWeekend({
    required String place,
    required String purpose,
    required String date,
    required String timeSlot,
    required String contactNumber,
  }) => _run(
    (client) => vtop_api.applyWeekendOuting(
      client: client,
      place: place,
      purpose: purpose,
      date: date,
      timeSlot: timeSlot,
      contactNumber: contactNumber,
    ),
  );

  /// Cancels a request VTOP still lets the student delete; [cancelId] is
  /// the record's `cancelId`.
  Future<OutingCancelResult> cancel({
    required bool weekend,
    required String cancelId,
  }) => _run(
    (client) => weekend
        ? vtop_api.cancelWeekendOuting(client: client, bookingId: cancelId)
        : vtop_api.cancelGeneralOuting(client: client, leaveId: cancelId),
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
      bytes: await _run(
        (client) => vtop_api.fetchOutingPass(
          client: client,
          weekend: weekend,
          passId: passId,
        ),
      ),
      fileName: '$name.pdf',
    ),
  );
}

final generalOutingProvider = FutureProvider.autoDispose<GeneralOutingData>(
  (ref) => ref.watch(outingServiceProvider).general(),
);

final weekendOutingProvider = FutureProvider.autoDispose<WeekendOutingData>(
  (ref) => ref.watch(outingServiceProvider).weekend(),
);
