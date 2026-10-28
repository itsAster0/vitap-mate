import 'package:flutter_test/flutter_test.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/vtop/entity/vtop_user_entity.dart';
import 'package:vitapmate/vtop/backend/vtop_backend_provider.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/features/course_page/course_page_service.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

import '../../support/recording_backend.dart';

class _User extends VtopUser {
  @override
  Future<VtopUserEntity> build() async =>
      VtopUserEntity.configured(username: 'u', password: 'p', semesterId: 'S');
}

void main() {
  test('course pages go through the backend by key, unsaved', () async {
    // Only the asking matters: every answer is an error.
    final backend = RecordingBackend((key) => throw StateError(key));
    final container = ProviderContainer(
      overrides: [
        vtopBackendProvider.overrideWithValue(backend),
        vtopUserProvider.overrideWith(_User.new),
      ],
      retry: (_, _) => null,
    );
    addTearDown(container.dispose);
    for (final provider in [
      coursePageCoursesProvider,
      coursePageClassesProvider('C1'),
      coursePageDetailProvider(('E1', 'C1')),
    ]) {
      final sub = container.listen(provider.future, (_, _) {});
      await expectLater(sub.read(), throwsStateError);
    }
    expect(backend.fetched.map((q) => q.key), [
      'course_page_courses',
      'course_page_classes?course_id=C1',
      'course_page_detail?class_id=C1&erp_id=E1',
    ]);
    expect(backend.fetched.map((q) => q.semester).toSet(), {'S'});
  });

  test('a file downloads with the session the backend gives', () async {
    final backend = RecordingBackend(
      (key) => switch (key) {
        'course_file_info?path=a%2Fb' => const CourseFileInfo(
          contentDisposition: 'attachment; filename="x.pdf"',
          contentType: 'application/pdf; charset=x',
        ),
        'vtop_session' => SessionState(
          cookies: 'JSESSIONID=s1',
          csrfToken: 'csrf-1',
          registrationNumber: '22BCE0001',
        ),
        _ => throw StateError(key),
      },
    );
    final downloads = <(String, String, String?)>[];
    final container = ProviderContainer(
      overrides: [
        vtopBackendProvider.overrideWithValue(backend),
        courseFileDownloaderProvider.overrideWithValue(
          (url, cookies, {contentDisposition, mimeType}) async =>
              downloads.add((url, cookies, mimeType)),
        ),
      ],
    );
    addTearDown(container.dispose);
    await container.read(coursePageServiceProvider).file('a/b');
    expect(backend.fetched.map((q) => q.key), [
      'course_file_info?path=a%2Fb',
      'vtop_session',
    ]);
    final (url, cookies, mime) = downloads.single;
    expect(cookies, 'JSESSIONID=s1');
    expect(mime, 'application/pdf');
    final query = Uri.parse(url).queryParameters;
    expect(url, startsWith('https://vtop.vitap.ac.in/vtop/a/b?'));
    expect(query['authorizedID'], '22BCE0001');
    expect(query['_csrf'], 'csrf-1');
  });

  test('the session query never keeps a hash and reads the server JSON', () {
    final query = VtopQueries.vtopSession();
    expect(query.key, 'vtop_session');
    final session = query.fromJson({
      'cookies': 'A=1',
      'csrfToken': 't',
      'registrationNumber': 'R',
    });
    expect(session.cookies, 'A=1');
    expect(session.csrfToken, 't');
    expect(session.registrationNumber, 'R');
  });
}
