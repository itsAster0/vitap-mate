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
    extends $AsyncNotifierProvider<StudentProjects, StudentProjectsData> {
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

String _$studentProjectsHash() => r'4819b10f8f8517a5d706eb173ced4ce7b303f5a3';

abstract class _$StudentProjects extends $AsyncNotifier<StudentProjectsData> {
  FutureOr<StudentProjectsData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref =
        this.ref as $Ref<AsyncValue<StudentProjectsData>, StudentProjectsData>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AsyncValue<StudentProjectsData>, StudentProjectsData>,
              AsyncValue<StudentProjectsData>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
