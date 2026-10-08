import 'dart:io';

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/vtop_client_call.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart' as vtop_api;

const _vtopBase = 'https://vtop.vitap.ac.in/vtop';

final coursePageServiceProvider = Provider<CoursePageService>(
  CoursePageService.new,
);

Future<String> _semesterId(Ref ref) =>
    ref.watch(vtopUserProvider.selectAsync((user) => user.semid!));

/// The courses on the course page for the selected semester.
final coursePageCoursesProvider = FutureProvider.autoDispose<CoursePageCourses>(
  (ref) async {
    final semester = await _semesterId(ref);
    return withVtopClient(
      ref,
      (client) =>
          vtop_api.fetchCoursePageCourses(client: client, semesterId: semester),
    );
  },
);

/// Every class of the course with this course page id.
final coursePageClassesProvider = FutureProvider.autoDispose
    .family<CoursePageClasses, String>((ref, courseId) async {
      final semester = await _semesterId(ref);
      return withVtopClient(
        ref,
        (client) => vtop_api.fetchCoursePageClasses(
          client: client,
          semesterId: semester,
          courseId: courseId,
        ),
      );
    });

/// A class's lecture plan, by (faculty erp id, class id).
final coursePageDetailProvider = FutureProvider.autoDispose
    .family<CoursePageDetail, (String, String)>((ref, key) async {
      final semester = await _semesterId(ref);
      return withVtopClient(
        ref,
        (client) => vtop_api.fetchCoursePageDetail(
          client: client,
          semesterId: semester,
          erpId: key.$1,
          classId: key.$2,
        ),
      );
    });

class CoursePageService {
  CoursePageService(this._ref);

  final Ref _ref;

  /// Downloads a course page file (lecture material, syllabus, material
  /// bundle) the way the VTOP webview does: VTOP's headers first, then the
  /// URL, cookie and those headers to [downloadFile], so the system download
  /// manager saves it to Downloads under VTOP's own file name.
  Future<void> file(String path) async {
    final info = await withVtopClient(
      _ref,
      (client) => vtop_api.fetchCourseFileInfo(client: client, path: path),
    );
    await _download('$_vtopBase/$path', info, withCsrf: true);
  }

  /// The class's course plan (Excel), the same way.
  Future<void> coursePlan({
    required String semesterId,
    required String classId,
  }) async {
    final info = await withVtopClient(
      _ref,
      (client) => vtop_api.fetchCoursePlanInfo(
        client: client,
        semesterId: semesterId,
        classId: classId,
      ),
    );
    await _download(
      '$_vtopBase/academics/common/CoursePlanExcelDownload',
      info,
      query: {'semesterSubId': semesterId, 'classId': classId},
    );
  }

  Future<void> _download(
    String url,
    CourseFileInfo info, {
    Map<String, String> query = const {},
    bool withCsrf = false,
  }) async {
    final session = await withVtopClient(
      _ref,
      (client) async => vtop_api.exportSessionState(client: client),
    );
    final params = Uri(
      queryParameters: {
        ...query,
        'authorizedID': session.registrationNumber ?? '',
        if (withCsrf) '_csrf': session.csrfToken ?? '',
        'x': HttpDate.format(DateTime.now()),
      },
    ).query;
    await downloadFile(
      '$url?$params',
      session.cookies,
      contentDisposition: info.contentDisposition,
      mimeType: info.contentType.split(';').first.trim(),
    );
  }
}
