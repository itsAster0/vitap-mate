// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'json_file_storage_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(jsonFileStorage)
final jsonFileStorageProvider = JsonFileStorageProvider._();

final class JsonFileStorageProvider
    extends
        $FunctionalProvider<
          AsyncValue<JsonFileStorage>,
          JsonFileStorage,
          FutureOr<JsonFileStorage>
        >
    with $FutureModifier<JsonFileStorage>, $FutureProvider<JsonFileStorage> {
  JsonFileStorageProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'jsonFileStorageProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$jsonFileStorageHash();

  @$internal
  @override
  $FutureProviderElement<JsonFileStorage> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<JsonFileStorage> create(Ref ref) {
    return jsonFileStorage(ref);
  }
}

String _$jsonFileStorageHash() => r'c944f62bf7527cd306fe645fb8f09cc2878225d8';
