import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/vtop/global_async_queue_provider.dart';
import 'package:vitapmate/platform/storage/json_file_storage_provider.dart';
import 'package:vitapmate/vtop/vtop_controller.dart';
import 'package:vitapmate/vtop/backend/vtop_backend_provider.dart';
import 'package:vitapmate/features/profile/data/profile_data_source.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

part 'profile_providers.g.dart';

@Riverpod(keepAlive: true)
Future<ProfileDataSource> profileDataSource(Ref ref) async {
  return ProfileDataSource(
    await ref.watch(jsonFileStorageProvider.future),
    () => ref.read(vtopBackendProvider),
    ref.read(globalAsyncQueueProvider.notifier),
  );
}

@Riverpod(keepAlive: true)
class Profile extends _$Profile {
  VtopController<StudentProfile> _controller(ProfileDataSource source) =>
      VtopController(
        ref: ref,
        repository: SavedPageRepository(source.profile),
        featureName: 'fetch-profile',
      );

  @override
  Future<StudentProfile> build() async =>
      _controller(await ref.watch(profileDataSourceProvider.future)).load();

  Future<void> refresh() async {
    final source = await ref.read(profileDataSourceProvider.future);
    state = AsyncData(await _controller(source).refresh());
  }
}

@Riverpod(keepAlive: true)
class Mentor extends _$Mentor {
  VtopController<MentorData> _controller(ProfileDataSource source) =>
      VtopController(
        ref: ref,
        repository: SavedPageRepository(source.mentor),
        featureName: 'fetch-mentor',
      );

  @override
  Future<MentorData> build() async =>
      _controller(await ref.watch(profileDataSourceProvider.future)).load();

  Future<void> refresh() async {
    final source = await ref.read(profileDataSourceProvider.future);
    state = AsyncData(await _controller(source).refresh());
  }
}
