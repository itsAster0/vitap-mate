// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'full_attendance_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

String _$fullAttendanceHash() => r'21f86bce9c70ed36d3be20398a86bbf4b4ecb826';

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

abstract class _$FullAttendance
    extends BuildlessAutoDisposeAsyncNotifier<FullAttendanceData> {
  late final String courseType;
  late final String courseId;

  FutureOr<FullAttendanceData> build(String courseType, String courseId);
}

/// See also [FullAttendance].
@ProviderFor(FullAttendance)
const fullAttendanceProvider = FullAttendanceFamily();

/// See also [FullAttendance].
class FullAttendanceFamily extends Family<AsyncValue<FullAttendanceData>> {
  /// See also [FullAttendance].
  const FullAttendanceFamily();

  /// See also [FullAttendance].
  FullAttendanceProvider call(String courseType, String courseId) {
    return FullAttendanceProvider(courseType, courseId);
  }

  @override
  FullAttendanceProvider getProviderOverride(
    covariant FullAttendanceProvider provider,
  ) {
    return call(provider.courseType, provider.courseId);
  }

  static const Iterable<ProviderOrFamily>? _dependencies = null;

  @override
  Iterable<ProviderOrFamily>? get dependencies => _dependencies;

  static const Iterable<ProviderOrFamily>? _allTransitiveDependencies = null;

  @override
  Iterable<ProviderOrFamily>? get allTransitiveDependencies =>
      _allTransitiveDependencies;

  @override
  String? get name => r'fullAttendanceProvider';
}

/// See also [FullAttendance].
class FullAttendanceProvider
    extends
        AutoDisposeAsyncNotifierProviderImpl<
          FullAttendance,
          FullAttendanceData
        > {
  /// See also [FullAttendance].
  FullAttendanceProvider(String courseType, String courseId)
    : this._internal(
        () =>
            FullAttendance()
              ..courseType = courseType
              ..courseId = courseId,
        from: fullAttendanceProvider,
        name: r'fullAttendanceProvider',
        debugGetCreateSourceHash:
            const bool.fromEnvironment('dart.vm.product')
                ? null
                : _$fullAttendanceHash,
        dependencies: FullAttendanceFamily._dependencies,
        allTransitiveDependencies:
            FullAttendanceFamily._allTransitiveDependencies,
        courseType: courseType,
        courseId: courseId,
      );

  FullAttendanceProvider._internal(
    super._createNotifier, {
    required super.name,
    required super.dependencies,
    required super.allTransitiveDependencies,
    required super.debugGetCreateSourceHash,
    required super.from,
    required this.courseType,
    required this.courseId,
  }) : super.internal();

  final String courseType;
  final String courseId;

  @override
  FutureOr<FullAttendanceData> runNotifierBuild(
    covariant FullAttendance notifier,
  ) {
    return notifier.build(courseType, courseId);
  }

  @override
  Override overrideWith(FullAttendance Function() create) {
    return ProviderOverride(
      origin: this,
      override: FullAttendanceProvider._internal(
        () =>
            create()
              ..courseType = courseType
              ..courseId = courseId,
        from: from,
        name: null,
        dependencies: null,
        allTransitiveDependencies: null,
        debugGetCreateSourceHash: null,
        courseType: courseType,
        courseId: courseId,
      ),
    );
  }

  @override
  AutoDisposeAsyncNotifierProviderElement<FullAttendance, FullAttendanceData>
  createElement() {
    return _FullAttendanceProviderElement(this);
  }

  @override
  bool operator ==(Object other) {
    return other is FullAttendanceProvider &&
        other.courseType == courseType &&
        other.courseId == courseId;
  }

  @override
  int get hashCode {
    var hash = _SystemHash.combine(0, runtimeType.hashCode);
    hash = _SystemHash.combine(hash, courseType.hashCode);
    hash = _SystemHash.combine(hash, courseId.hashCode);

    return _SystemHash.finish(hash);
  }
}

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
mixin FullAttendanceRef
    on AutoDisposeAsyncNotifierProviderRef<FullAttendanceData> {
  /// The parameter `courseType` of this provider.
  String get courseType;

  /// The parameter `courseId` of this provider.
  String get courseId;
}

class _FullAttendanceProviderElement
    extends
        AutoDisposeAsyncNotifierProviderElement<
          FullAttendance,
          FullAttendanceData
        >
    with FullAttendanceRef {
  _FullAttendanceProviderElement(super.provider);

  @override
  String get courseType => (origin as FullAttendanceProvider).courseType;
  @override
  String get courseId => (origin as FullAttendanceProvider).courseId;
}

// ignore_for_file: type=lint
// ignore_for_file: subtype_of_sealed_class, invalid_use_of_internal_member, invalid_use_of_visible_for_testing_member, deprecated_member_use_from_same_package
