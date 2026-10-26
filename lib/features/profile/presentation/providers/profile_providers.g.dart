// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'profile_providers.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(profileDataSource)
final profileDataSourceProvider = ProfileDataSourceProvider._();

final class ProfileDataSourceProvider
    extends
        $FunctionalProvider<
          AsyncValue<ProfileDataSource>,
          ProfileDataSource,
          FutureOr<ProfileDataSource>
        >
    with
        $FutureModifier<ProfileDataSource>,
        $FutureProvider<ProfileDataSource> {
  ProfileDataSourceProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'profileDataSourceProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$profileDataSourceHash();

  @$internal
  @override
  $FutureProviderElement<ProfileDataSource> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<ProfileDataSource> create(Ref ref) {
    return profileDataSource(ref);
  }
}

String _$profileDataSourceHash() => r'ebea3ba6724597ef9557c21458746962820657b7';

@ProviderFor(Profile)
final profileProvider = ProfileProvider._();

final class ProfileProvider
    extends $AsyncNotifierProvider<Profile, StudentProfile> {
  ProfileProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'profileProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$profileHash();

  @$internal
  @override
  Profile create() => Profile();
}

String _$profileHash() => r'400fcab484a55b362bb7aa9ad00d0db1399ba3e9';

abstract class _$Profile extends $AsyncNotifier<StudentProfile> {
  FutureOr<StudentProfile> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<AsyncValue<StudentProfile>, StudentProfile>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AsyncValue<StudentProfile>, StudentProfile>,
              AsyncValue<StudentProfile>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}

@ProviderFor(Mentor)
final mentorProvider = MentorProvider._();

final class MentorProvider extends $AsyncNotifierProvider<Mentor, MentorData> {
  MentorProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'mentorProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$mentorHash();

  @$internal
  @override
  Mentor create() => Mentor();
}

String _$mentorHash() => r'8338935035cd2fdac07aa083fd078f82e9ea3006';

abstract class _$Mentor extends $AsyncNotifier<MentorData> {
  FutureOr<MentorData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<AsyncValue<MentorData>, MentorData>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AsyncValue<MentorData>, MentorData>,
              AsyncValue<MentorData>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
