// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'student_projects_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(StudentProjects)
final studentProjectsProvider = StudentProjectsProvider._();

final class StudentProjectsProvider
    extends $AsyncNotifierProvider<StudentProjects, List<StudentProject>> {
  StudentProjectsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'studentProjectsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$studentProjectsHash();

  @$internal
  @override
  StudentProjects create() => StudentProjects();
}

String _$studentProjectsHash() => r'f4b6c665ca78a20e8cee07201793db6935af0a75';

abstract class _$StudentProjects extends $AsyncNotifier<List<StudentProject>> {
  FutureOr<List<StudentProject>> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref =
        this.ref
            as $Ref<AsyncValue<List<StudentProject>>, List<StudentProject>>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<
                AsyncValue<List<StudentProject>>,
                List<StudentProject>
              >,
              AsyncValue<List<StudentProject>>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
