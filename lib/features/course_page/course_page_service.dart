import 'dart:io';

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/backend/vtop_backend_provider.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/platform/files/general_utils.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

const _vtopBase = 'https://vtop.vitap.ac.in/vtop';

/// Hands a download to the system download manager; see [downloadFile].
typedef CourseFileDownloader =
    Future<void> Function(
      String url,
      String cookies, {
      String? contentDisposition,
      String? mimeType,
    });

final courseFileDownloaderProvider = Provider<CourseFileDownloader>(
  (ref) =>
      (url, cookies, {contentDisposition, mimeType}) => downloadFile(
        url,
        cookies,
        contentDisposition: contentDisposition,
        mimeType: mimeType,
      ),
);

final coursePageServiceProvider = Provider<CoursePageService>(
  CoursePageService.new,
);

Future<String> _semesterId(Ref ref) =>
    ref.watch(vtopUserProvider.selectAsync((user) => user.semid!));

/// The courses on the course page for the selected semester.
final coursePageCoursesProvider = FutureProvider.autoDispose<CoursePageCourses>(
  (ref) async {
    final semester = await _semesterId(ref);
    return ref
        .read(vtopBackendProvider)
        .fetch(VtopQueries.coursePageCourses(semester));
  },
);

/// Every class of the course with this course page id.
final coursePageClassesProvider = FutureProvider.autoDispose
    .family<CoursePageClasses, String>((ref, courseId) async {
      final semester = await _semesterId(ref);
      return ref
          .read(vtopBackendProvider)
          .fetch(VtopQueries.coursePageClasses(semester, courseId: courseId));
    });

/// A class's lecture plan, by (faculty erp id, class id).
final coursePageDetailProvider = FutureProvider.autoDispose
    .family<CoursePageDetail, (String, String)>((ref, key) async {
      final semester = await _semesterId(ref);
      return ref
          .read(vtopBackendProvider)
          .fetch(
            VtopQueries.coursePageDetail(
              semester,
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
    final info = await _ref
        .read(vtopBackendProvider)
        .fetch(VtopQueries.courseFileInfo(path));
    await _download('$_vtopBase/$path', info, withCsrf: true);
  }

  /// The class's course plan (Excel), the same way.
  Future<void> coursePlan({
    required String semesterId,
    required String classId,
  }) async {
    final info = await _ref
        .read(vtopBackendProvider)
        .fetch(VtopQueries.coursePlanInfo(semesterId, classId: classId));
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
    // The file streams from VTOP to the system downloader, with the
    // session the mode says: the server's, so the phone need not sign in.
    final session = await _ref
        .read(vtopBackendProvider)
        .fetch(VtopQueries.vtopSession());
    final params = Uri(
      queryParameters: {
        ...query,
        'authorizedID': session.registrationNumber ?? '',
        if (withCsrf) '_csrf': session.csrfToken ?? '',
        'x': HttpDate.format(DateTime.now()),
      },
    ).query;
    await _ref.read(courseFileDownloaderProvider)(
      '$url?$params',
      session.cookies,
      contentDisposition: info.contentDisposition,
      mimeType: info.contentType.split(';').first.trim(),
    );
  }
}
