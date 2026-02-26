// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'exam_schedule.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

String _$examScheduleRepositoryHash() =>
    r'8f539bd7a2f87d8c59861bd8daf50d7b477abbc4';

/// See also [examScheduleRepository].
@ProviderFor(examScheduleRepository)
final examScheduleRepositoryProvider =
    AutoDisposeFutureProvider<ExamScheduleRepository>.internal(
      examScheduleRepository,
      name: r'examScheduleRepositoryProvider',
      debugGetCreateSourceHash:
          const bool.fromEnvironment('dart.vm.product')
              ? null
              : _$examScheduleRepositoryHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
typedef ExamScheduleRepositoryRef =
    AutoDisposeFutureProviderRef<ExamScheduleRepository>;
String _$marksRepositoryHash() => r'62be906347e2a23f1bf20fdbb23f2797874de780';

/// See also [marksRepository].
@ProviderFor(marksRepository)
final marksRepositoryProvider =
    AutoDisposeFutureProvider<MarksRepository>.internal(
      marksRepository,
      name: r'marksRepositoryProvider',
      debugGetCreateSourceHash:
          const bool.fromEnvironment('dart.vm.product')
              ? null
              : _$marksRepositoryHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
typedef MarksRepositoryRef = AutoDisposeFutureProviderRef<MarksRepository>;
String _$gradesRepositoryHash() => r'e8d4f5b2a020eb5eface9448a9dff7d060836b33';

/// See also [gradesRepository].
@ProviderFor(gradesRepository)
final gradesRepositoryProvider =
    AutoDisposeFutureProvider<GradesRepository>.internal(
      gradesRepository,
      name: r'gradesRepositoryProvider',
      debugGetCreateSourceHash:
          const bool.fromEnvironment('dart.vm.product')
              ? null
              : _$gradesRepositoryHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
typedef GradesRepositoryRef = AutoDisposeFutureProviderRef<GradesRepository>;
String _$gradesRepositoryForSemHash() =>
    r'4e4999017cc5a31c627bfec5f9b55cdb4f291132';

/// Copied from Dart SDK
class _SystemHash {
  _SystemHash._();

  static int combine(int hash, int value) {
    // ignore: parameter_assignments
    hash = 0x1fffffff & (hash + value);
    // ignore: parameter_assignments
    hash = 0x1fffffff & (hash + ((0x0007ffff & hash) << 10));
    return hash ^ (hash >> 6);
  }

  static int finish(int hash) {
    // ignore: parameter_assignments
    hash = 0x1fffffff & (hash + ((0x03ffffff & hash) << 3));
    // ignore: parameter_assignments
    hash = hash ^ (hash >> 11);
    return 0x1fffffff & (hash + ((0x00003fff & hash) << 15));
  }
}

/// See also [gradesRepositoryForSem].
@ProviderFor(gradesRepositoryForSem)
const gradesRepositoryForSemProvider = GradesRepositoryForSemFamily();

/// See also [gradesRepositoryForSem].
class GradesRepositoryForSemFamily
    extends Family<AsyncValue<GradesRepository>> {
  /// See also [gradesRepositoryForSem].
  const GradesRepositoryForSemFamily();

  /// See also [gradesRepositoryForSem].
  GradesRepositoryForSemProvider call(String semid) {
    return GradesRepositoryForSemProvider(semid);
  }

  @override
  GradesRepositoryForSemProvider getProviderOverride(
    covariant GradesRepositoryForSemProvider provider,
  ) {
    return call(provider.semid);
  }

  static const Iterable<ProviderOrFamily>? _dependencies = null;

  @override
  Iterable<ProviderOrFamily>? get dependencies => _dependencies;

  static const Iterable<ProviderOrFamily>? _allTransitiveDependencies = null;

  @override
  Iterable<ProviderOrFamily>? get allTransitiveDependencies =>
      _allTransitiveDependencies;

  @override
  String? get name => r'gradesRepositoryForSemProvider';
}

/// See also [gradesRepositoryForSem].
class GradesRepositoryForSemProvider
    extends AutoDisposeFutureProvider<GradesRepository> {
  /// See also [gradesRepositoryForSem].
  GradesRepositoryForSemProvider(String semid)
    : this._internal(
        (ref) =>
            gradesRepositoryForSem(ref as GradesRepositoryForSemRef, semid),
        from: gradesRepositoryForSemProvider,
        name: r'gradesRepositoryForSemProvider',
        debugGetCreateSourceHash:
            const bool.fromEnvironment('dart.vm.product')
                ? null
                : _$gradesRepositoryForSemHash,
        dependencies: GradesRepositoryForSemFamily._dependencies,
        allTransitiveDependencies:
            GradesRepositoryForSemFamily._allTransitiveDependencies,
        semid: semid,
      );

  GradesRepositoryForSemProvider._internal(
    super._createNotifier, {
    required super.name,
    required super.dependencies,
    required super.allTransitiveDependencies,
    required super.debugGetCreateSourceHash,
    required super.from,
    required this.semid,
  }) : super.internal();

  final String semid;

  @override
  Override overrideWith(
    FutureOr<GradesRepository> Function(GradesRepositoryForSemRef provider)
    create,
  ) {
    return ProviderOverride(
      origin: this,
      override: GradesRepositoryForSemProvider._internal(
        (ref) => create(ref as GradesRepositoryForSemRef),
        from: from,
        name: null,
        dependencies: null,
        allTransitiveDependencies: null,
        debugGetCreateSourceHash: null,
        semid: semid,
      ),
    );
  }

  @override
  AutoDisposeFutureProviderElement<GradesRepository> createElement() {
    return _GradesRepositoryForSemProviderElement(this);
  }

  @override
  bool operator ==(Object other) {
    return other is GradesRepositoryForSemProvider && other.semid == semid;
  }

  @override
  int get hashCode {
    var hash = _SystemHash.combine(0, runtimeType.hashCode);
    hash = _SystemHash.combine(hash, semid.hashCode);

    return _SystemHash.finish(hash);
  }
}

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
mixin GradesRepositoryForSemRef
    on AutoDisposeFutureProviderRef<GradesRepository> {
  /// The parameter `semid` of this provider.
  String get semid;
}

class _GradesRepositoryForSemProviderElement
    extends AutoDisposeFutureProviderElement<GradesRepository>
    with GradesRepositoryForSemRef {
  _GradesRepositoryForSemProviderElement(super.provider);

  @override
  String get semid => (origin as GradesRepositoryForSemProvider).semid;
}

String _$gradeHistoryRepositoryHash() =>
    r'bae07314b22cd54c4a25a9910c64361d27bede99';

/// See also [gradeHistoryRepository].
@ProviderFor(gradeHistoryRepository)
final gradeHistoryRepositoryProvider =
    AutoDisposeFutureProvider<GradeHistoryRepository>.internal(
      gradeHistoryRepository,
      name: r'gradeHistoryRepositoryProvider',
      debugGetCreateSourceHash:
          const bool.fromEnvironment('dart.vm.product')
              ? null
              : _$gradeHistoryRepositoryHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
typedef GradeHistoryRepositoryRef =
    AutoDisposeFutureProviderRef<GradeHistoryRepository>;
// ignore_for_file: type=lint
// ignore_for_file: subtype_of_sealed_class, invalid_use_of_internal_member, invalid_use_of_visible_for_testing_member, deprecated_member_use_from_same_package
