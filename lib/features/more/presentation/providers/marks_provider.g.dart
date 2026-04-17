// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'marks_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(Marks)
final marksProvider = MarksProvider._();

final class MarksProvider extends $AsyncNotifierProvider<Marks, MarksData> {
  MarksProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'marksProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$marksHash();

  @$internal
  @override
  Marks create() => Marks();
}

String _$marksHash() => r'644e4b4506aed0df2cf4f042d4dd3064a373ee18';

abstract class _$Marks extends $AsyncNotifier<MarksData> {
  FutureOr<MarksData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<AsyncValue<MarksData>, MarksData>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AsyncValue<MarksData>, MarksData>,
              AsyncValue<MarksData>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
