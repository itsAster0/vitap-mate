// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'msg_utils.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

String _$getUserrecordHash() => r'9111cd1449b1d05a4cbbf5972c861521854a5f6b';

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

/// See also [getUserrecord].
@ProviderFor(getUserrecord)
const getUserrecordProvider = GetUserrecordFamily();

/// See also [getUserrecord].
class GetUserrecordFamily
    extends Family<AsyncValue<(RecordModel, PocketBase)>> {
  /// See also [getUserrecord].
  const GetUserrecordFamily();

  /// See also [getUserrecord].
  GetUserrecordProvider call(String id) {
    return GetUserrecordProvider(id);
  }

  @override
  GetUserrecordProvider getProviderOverride(
    covariant GetUserrecordProvider provider,
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
  String? get name => r'getUserrecordProvider';
}

/// See also [getUserrecord].
class GetUserrecordProvider
    extends AutoDisposeStreamProvider<(RecordModel, PocketBase)> {
  /// See also [getUserrecord].
  GetUserrecordProvider(String id)
    : this._internal(
        (ref) => getUserrecord(ref as GetUserrecordRef, id),
        from: getUserrecordProvider,
        name: r'getUserrecordProvider',
        debugGetCreateSourceHash:
            const bool.fromEnvironment('dart.vm.product')
                ? null
                : _$getUserrecordHash,
        dependencies: GetUserrecordFamily._dependencies,
        allTransitiveDependencies:
            GetUserrecordFamily._allTransitiveDependencies,
        id: id,
      );

  GetUserrecordProvider._internal(
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
    Stream<(RecordModel, PocketBase)> Function(GetUserrecordRef provider)
    create,
  ) {
    return ProviderOverride(
      origin: this,
      override: GetUserrecordProvider._internal(
        (ref) => create(ref as GetUserrecordRef),
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
  AutoDisposeStreamProviderElement<(RecordModel, PocketBase)> createElement() {
    return _GetUserrecordProviderElement(this);
  }

  @override
  bool operator ==(Object other) {
    return other is GetUserrecordProvider && other.id == id;
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
mixin GetUserrecordRef
    on AutoDisposeStreamProviderRef<(RecordModel, PocketBase)> {
  /// The parameter `id` of this provider.
  String get id;
}

class _GetUserrecordProviderElement
    extends AutoDisposeStreamProviderElement<(RecordModel, PocketBase)>
    with GetUserrecordRef {
  _GetUserrecordProviderElement(super.provider);

  @override
  String get id => (origin as GetUserrecordProvider).id;
}

String _$getRoleHash() => r'0e48464bde46a46bb2ff6adcaee6cc01c737a39b';

/// See also [getRole].
@ProviderFor(getRole)
const getRoleProvider = GetRoleFamily();

/// See also [getRole].
class GetRoleFamily extends Family<AsyncValue<String?>> {
  /// See also [getRole].
  const GetRoleFamily();

  /// See also [getRole].
  GetRoleProvider call(String id) {
    return GetRoleProvider(id);
  }

  @override
  GetRoleProvider getProviderOverride(covariant GetRoleProvider provider) {
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
  String? get name => r'getRoleProvider';
}

/// See also [getRole].
class GetRoleProvider extends AutoDisposeFutureProvider<String?> {
  /// See also [getRole].
  GetRoleProvider(String id)
    : this._internal(
        (ref) => getRole(ref as GetRoleRef, id),
        from: getRoleProvider,
        name: r'getRoleProvider',
        debugGetCreateSourceHash:
            const bool.fromEnvironment('dart.vm.product')
                ? null
                : _$getRoleHash,
        dependencies: GetRoleFamily._dependencies,
        allTransitiveDependencies: GetRoleFamily._allTransitiveDependencies,
        id: id,
      );

  GetRoleProvider._internal(
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
    FutureOr<String?> Function(GetRoleRef provider) create,
  ) {
    return ProviderOverride(
      origin: this,
      override: GetRoleProvider._internal(
        (ref) => create(ref as GetRoleRef),
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
  AutoDisposeFutureProviderElement<String?> createElement() {
    return _GetRoleProviderElement(this);
  }

  @override
  bool operator ==(Object other) {
    return other is GetRoleProvider && other.id == id;
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
mixin GetRoleRef on AutoDisposeFutureProviderRef<String?> {
  /// The parameter `id` of this provider.
  String get id;
}

class _GetRoleProviderElement extends AutoDisposeFutureProviderElement<String?>
    with GetRoleRef {
  _GetRoleProviderElement(super.provider);

  @override
  String get id => (origin as GetRoleProvider).id;
}

// ignore_for_file: type=lint
// ignore_for_file: subtype_of_sealed_class, invalid_use_of_internal_member, invalid_use_of_visible_for_testing_member, deprecated_member_use_from_same_package
