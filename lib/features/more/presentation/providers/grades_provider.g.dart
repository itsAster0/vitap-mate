// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'grades_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(Grades)
final gradesProvider = GradesProvider._();

final class GradesProvider
    extends $AsyncNotifierProvider<Grades, GradesUiState> {
  GradesProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'gradesProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$gradesHash();

  @$internal
  @override
  Grades create() => Grades();
}

String _$gradesHash() => r'fdeb488dbc35041e6bd17db6a339206eb5ddf55c';

abstract class _$Grades extends $AsyncNotifier<GradesUiState> {
  FutureOr<GradesUiState> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<AsyncValue<GradesUiState>, GradesUiState>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AsyncValue<GradesUiState>, GradesUiState>,
              AsyncValue<GradesUiState>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
