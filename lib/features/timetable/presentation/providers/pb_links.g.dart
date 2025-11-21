// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'pb_links.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

String _$getTimetableSharesLinksHash() =>
    r'f7fd52d72f8d664a857738600936fcf0f11d2c4d';

/// See also [getTimetableSharesLinks].
@ProviderFor(getTimetableSharesLinks)
final getTimetableSharesLinksProvider =
    AutoDisposeFutureProvider<List>.internal(
      getTimetableSharesLinks,
      name: r'getTimetableSharesLinksProvider',
      debugGetCreateSourceHash:
          const bool.fromEnvironment('dart.vm.product')
              ? null
              : _$getTimetableSharesLinksHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
typedef GetTimetableSharesLinksRef = AutoDisposeFutureProviderRef<List>;
String _$createTimetabledbHash() => r'd5ccc3b8da017323ab9619102ff5cefc86e98c8d';

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

/// See also [createTimetabledb].
@ProviderFor(createTimetabledb)
const createTimetabledbProvider = CreateTimetabledbFamily();

/// See also [createTimetabledb].
class CreateTimetabledbFamily extends Family<AsyncValue<void>> {
  /// See also [createTimetabledb].
  const CreateTimetabledbFamily();

  /// See also [createTimetabledb].
  CreateTimetabledbProvider call(TimetableData ttData) {
    return CreateTimetabledbProvider(ttData);
  }

  @override
  CreateTimetabledbProvider getProviderOverride(
    covariant CreateTimetabledbProvider provider,
  ) {
    return call(provider.ttData);
  }

  static const Iterable<ProviderOrFamily>? _dependencies = null;

  @override
  Iterable<ProviderOrFamily>? get dependencies => _dependencies;

  static const Iterable<ProviderOrFamily>? _allTransitiveDependencies = null;

  @override
  Iterable<ProviderOrFamily>? get allTransitiveDependencies =>
      _allTransitiveDependencies;

  @override
  String? get name => r'createTimetabledbProvider';
}

/// See also [createTimetabledb].
class CreateTimetabledbProvider extends AutoDisposeFutureProvider<void> {
  /// See also [createTimetabledb].
  CreateTimetabledbProvider(TimetableData ttData)
    : this._internal(
        (ref) => createTimetabledb(ref as CreateTimetabledbRef, ttData),
        from: createTimetabledbProvider,
        name: r'createTimetabledbProvider',
        debugGetCreateSourceHash:
            const bool.fromEnvironment('dart.vm.product')
                ? null
                : _$createTimetabledbHash,
        dependencies: CreateTimetabledbFamily._dependencies,
        allTransitiveDependencies:
            CreateTimetabledbFamily._allTransitiveDependencies,
        ttData: ttData,
      );

  CreateTimetabledbProvider._internal(
    super._createNotifier, {
    required super.name,
    required super.dependencies,
    required super.allTransitiveDependencies,
    required super.debugGetCreateSourceHash,
    required super.from,
    required this.ttData,
  }) : super.internal();

  final TimetableData ttData;

  @override
  Override overrideWith(
    FutureOr<void> Function(CreateTimetabledbRef provider) create,
  ) {
    return ProviderOverride(
      origin: this,
      override: CreateTimetabledbProvider._internal(
        (ref) => create(ref as CreateTimetabledbRef),
        from: from,
        name: null,
        dependencies: null,
        allTransitiveDependencies: null,
        debugGetCreateSourceHash: null,
        ttData: ttData,
      ),
    );
  }

  @override
  AutoDisposeFutureProviderElement<void> createElement() {
    return _CreateTimetabledbProviderElement(this);
  }

  @override
  bool operator ==(Object other) {
    return other is CreateTimetabledbProvider && other.ttData == ttData;
  }

  @override
  int get hashCode {
    var hash = _SystemHash.combine(0, runtimeType.hashCode);
    hash = _SystemHash.combine(hash, ttData.hashCode);

    return _SystemHash.finish(hash);
  }
}

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
mixin CreateTimetabledbRef on AutoDisposeFutureProviderRef<void> {
  /// The parameter `ttData` of this provider.
  TimetableData get ttData;
}

class _CreateTimetabledbProviderElement
    extends AutoDisposeFutureProviderElement<void>
    with CreateTimetabledbRef {
  _CreateTimetabledbProviderElement(super.provider);

  @override
  TimetableData get ttData => (origin as CreateTimetabledbProvider).ttData;
}

String _$createTimetablelinkdbHash() =>
    r'd4cae54e58c90283d0b92055ef74b1c297f65637';

/// See also [createTimetablelinkdb].
@ProviderFor(createTimetablelinkdb)
const createTimetablelinkdbProvider = CreateTimetablelinkdbFamily();

/// See also [createTimetablelinkdb].
class CreateTimetablelinkdbFamily extends Family<AsyncValue<void>> {
  /// See also [createTimetablelinkdb].
  const CreateTimetablelinkdbFamily();

  /// See also [createTimetablelinkdb].
  CreateTimetablelinkdbProvider call(String id) {
    return CreateTimetablelinkdbProvider(id);
  }

  @override
  CreateTimetablelinkdbProvider getProviderOverride(
    covariant CreateTimetablelinkdbProvider provider,
  ) {
    return call(provider.id);
  }

  static const Iterable<ProviderOrFamily>? _dependencies = null;

  @override
  Iterable<ProviderOrFamily>? get dependencies => _dependencies;

  static const Iterable<ProviderOrFamily>? _allTransitiveDependencies = null;

  @override
  Iterable<ProviderOrFamily>? get allTransitiveDependencies =>
      _allTransitiveDependencies;

  @override
  String? get name => r'createTimetablelinkdbProvider';
}

/// See also [createTimetablelinkdb].
class CreateTimetablelinkdbProvider extends AutoDisposeFutureProvider<void> {
  /// See also [createTimetablelinkdb].
  CreateTimetablelinkdbProvider(String id)
    : this._internal(
        (ref) => createTimetablelinkdb(ref as CreateTimetablelinkdbRef, id),
        from: createTimetablelinkdbProvider,
        name: r'createTimetablelinkdbProvider',
        debugGetCreateSourceHash:
            const bool.fromEnvironment('dart.vm.product')
                ? null
                : _$createTimetablelinkdbHash,
        dependencies: CreateTimetablelinkdbFamily._dependencies,
        allTransitiveDependencies:
            CreateTimetablelinkdbFamily._allTransitiveDependencies,
        id: id,
      );

  CreateTimetablelinkdbProvider._internal(
    super._createNotifier, {
    required super.name,
    required super.dependencies,
    required super.allTransitiveDependencies,
    required super.debugGetCreateSourceHash,
    required super.from,
    required this.id,
  }) : super.internal();

  final String id;

  @override
  Override overrideWith(
    FutureOr<void> Function(CreateTimetablelinkdbRef provider) create,
  ) {
    return ProviderOverride(
      origin: this,
      override: CreateTimetablelinkdbProvider._internal(
        (ref) => create(ref as CreateTimetablelinkdbRef),
        from: from,
        name: null,
        dependencies: null,
        allTransitiveDependencies: null,
        debugGetCreateSourceHash: null,
        id: id,
      ),
    );
  }

  @override
  AutoDisposeFutureProviderElement<void> createElement() {
    return _CreateTimetablelinkdbProviderElement(this);
  }

  @override
  bool operator ==(Object other) {
    return other is CreateTimetablelinkdbProvider && other.id == id;
  }

  @override
  int get hashCode {
    var hash = _SystemHash.combine(0, runtimeType.hashCode);
    hash = _SystemHash.combine(hash, id.hashCode);

    return _SystemHash.finish(hash);
  }
}

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
mixin CreateTimetablelinkdbRef on AutoDisposeFutureProviderRef<void> {
  /// The parameter `id` of this provider.
  String get id;
}

class _CreateTimetablelinkdbProviderElement
    extends AutoDisposeFutureProviderElement<void>
    with CreateTimetablelinkdbRef {
  _CreateTimetablelinkdbProviderElement(super.provider);

  @override
  String get id => (origin as CreateTimetablelinkdbProvider).id;
}

String _$getTimetabledbHash() => r'b795ae6863119f6c8ccadb8610a12a5be4fd8d1d';

/// See also [getTimetabledb].
@ProviderFor(getTimetabledb)
final getTimetabledbProvider =
    AutoDisposeFutureProvider<List<RecordModel>>.internal(
      getTimetabledb,
      name: r'getTimetabledbProvider',
      debugGetCreateSourceHash:
          const bool.fromEnvironment('dart.vm.product')
              ? null
              : _$getTimetabledbHash,
      dependencies: null,
      allTransitiveDependencies: null,
    );

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
typedef GetTimetabledbRef = AutoDisposeFutureProviderRef<List<RecordModel>>;
String _$delTimetablelinkdbHash() =>
    r'9e4c4014a0004c05f0844380a11be2af1223288d';

/// See also [delTimetablelinkdb].
@ProviderFor(delTimetablelinkdb)
const delTimetablelinkdbProvider = DelTimetablelinkdbFamily();

/// See also [delTimetablelinkdb].
class DelTimetablelinkdbFamily extends Family<AsyncValue<void>> {
  /// See also [delTimetablelinkdb].
  const DelTimetablelinkdbFamily();

  /// See also [delTimetablelinkdb].
  DelTimetablelinkdbProvider call(String id) {
    return DelTimetablelinkdbProvider(id);
  }

  @override
  DelTimetablelinkdbProvider getProviderOverride(
    covariant DelTimetablelinkdbProvider provider,
  ) {
    return call(provider.id);
  }

  static const Iterable<ProviderOrFamily>? _dependencies = null;

  @override
  Iterable<ProviderOrFamily>? get dependencies => _dependencies;

  static const Iterable<ProviderOrFamily>? _allTransitiveDependencies = null;

  @override
  Iterable<ProviderOrFamily>? get allTransitiveDependencies =>
      _allTransitiveDependencies;

  @override
  String? get name => r'delTimetablelinkdbProvider';
}

/// See also [delTimetablelinkdb].
class DelTimetablelinkdbProvider extends AutoDisposeFutureProvider<void> {
  /// See also [delTimetablelinkdb].
  DelTimetablelinkdbProvider(String id)
    : this._internal(
        (ref) => delTimetablelinkdb(ref as DelTimetablelinkdbRef, id),
        from: delTimetablelinkdbProvider,
        name: r'delTimetablelinkdbProvider',
        debugGetCreateSourceHash:
            const bool.fromEnvironment('dart.vm.product')
                ? null
                : _$delTimetablelinkdbHash,
        dependencies: DelTimetablelinkdbFamily._dependencies,
        allTransitiveDependencies:
            DelTimetablelinkdbFamily._allTransitiveDependencies,
        id: id,
      );

  DelTimetablelinkdbProvider._internal(
    super._createNotifier, {
    required super.name,
    required super.dependencies,
    required super.allTransitiveDependencies,
    required super.debugGetCreateSourceHash,
    required super.from,
    required this.id,
  }) : super.internal();

  final String id;

  @override
  Override overrideWith(
    FutureOr<void> Function(DelTimetablelinkdbRef provider) create,
  ) {
    return ProviderOverride(
      origin: this,
      override: DelTimetablelinkdbProvider._internal(
        (ref) => create(ref as DelTimetablelinkdbRef),
        from: from,
        name: null,
        dependencies: null,
        allTransitiveDependencies: null,
        debugGetCreateSourceHash: null,
        id: id,
      ),
    );
  }

  @override
  AutoDisposeFutureProviderElement<void> createElement() {
    return _DelTimetablelinkdbProviderElement(this);
  }

  @override
  bool operator ==(Object other) {
    return other is DelTimetablelinkdbProvider && other.id == id;
  }

  @override
  int get hashCode {
    var hash = _SystemHash.combine(0, runtimeType.hashCode);
    hash = _SystemHash.combine(hash, id.hashCode);

    return _SystemHash.finish(hash);
  }
}

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
mixin DelTimetablelinkdbRef on AutoDisposeFutureProviderRef<void> {
  /// The parameter `id` of this provider.
  String get id;
}

class _DelTimetablelinkdbProviderElement
    extends AutoDisposeFutureProviderElement<void>
    with DelTimetablelinkdbRef {
  _DelTimetablelinkdbProviderElement(super.provider);

  @override
  String get id => (origin as DelTimetablelinkdbProvider).id;
}

String _$sharePageHash() => r'eb55dba328f33ab90d4e5e0893ae7a3ac2f73649';

/// See also [sharePage].
@ProviderFor(sharePage)
final sharePageProvider = AutoDisposeFutureProvider<
  (TimetableData, SemesterData, List, List<RecordModel>)
>.internal(
  sharePage,
  name: r'sharePageProvider',
  debugGetCreateSourceHash:
      const bool.fromEnvironment('dart.vm.product') ? null : _$sharePageHash,
  dependencies: null,
  allTransitiveDependencies: null,
);

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
typedef SharePageRef =
    AutoDisposeFutureProviderRef<
      (TimetableData, SemesterData, List, List<RecordModel>)
    >;
// ignore_for_file: type=lint
// ignore_for_file: subtype_of_sealed_class, invalid_use_of_internal_member, invalid_use_of_visible_for_testing_member, deprecated_member_use_from_same_package
