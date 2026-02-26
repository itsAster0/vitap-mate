import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/features/more/domine/usecases/get_grades.dart';
import 'package:vitapmate/features/more/domine/repositories/grades_repo.dart';
import 'package:vitapmate/features/more/domine/usecases/update_grades.dart';
import 'package:vitapmate/features/more/presentation/providers/state/exam_schedule.dart';
import 'package:vitapmate/features/settings/presentation/providers/semester_id_provider.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GradesUiState {
  final GradeViewData gradeView;
  final List<SemesterInfo> semesters;
  final String selectedSemesterId;
  final Map<String, GradeDetailsData> detailsByCourseId;
  final Set<String> loadingDetailsFor;

  const GradesUiState({
    required this.gradeView,
    required this.semesters,
    required this.selectedSemesterId,
    required this.detailsByCourseId,
    required this.loadingDetailsFor,
  });

  GradesUiState copyWith({
    GradeViewData? gradeView,
    List<SemesterInfo>? semesters,
    String? selectedSemesterId,
    Map<String, GradeDetailsData>? detailsByCourseId,
    Set<String>? loadingDetailsFor,
  }) {
    return GradesUiState(
      gradeView: gradeView ?? this.gradeView,
      semesters: semesters ?? this.semesters,
      selectedSemesterId: selectedSemesterId ?? this.selectedSemesterId,
      detailsByCourseId: detailsByCourseId ?? this.detailsByCourseId,
      loadingDetailsFor: loadingDetailsFor ?? this.loadingDetailsFor,
    );
  }
}

final gradesProvider = AsyncNotifierProvider<GradesNotifier, GradesUiState>(
  GradesNotifier.new,
);

class GradesNotifier extends AsyncNotifier<GradesUiState> {
  @override
  Future<GradesUiState> build() async {
    final user = await ref.read(vtopUserProvider.future);
    final semData = await ref.watch(semesterIdProvider.future);
    final semId = user.semid ?? "";
    return _loadSemester(semId, semData.semesters);
  }

  Future<void> refresh() async {
    final current = state.valueOrNull;
    final semId = current?.selectedSemesterId;
    if (semId == null || semId.isEmpty) return;

    final hasLocalData = current?.gradeView.courses.isNotEmpty ?? false;
    final repo = await ref.read(gradesRepositoryForSemProvider(semId).future);
    final didFetchRemote = await _updateView(repo, hasLocalData: hasLocalData);
    final next = await _loadSemester(
      semId,
      current!.semesters,
      forceRemote: false,
    );
    state = AsyncData(next);
    if (!didFetchRemote) {
      throw FeatureDisabledException("Grades Feature Disabled");
    }
  }

  Future<void> selectSemester(String semId) async {
    final current = state.valueOrNull;
    if (current != null && current.selectedSemesterId == semId) return;
    if (current != null) {
      state = AsyncData(current.copyWith(selectedSemesterId: semId));
    }
    final fallbackSems =
        current?.semesters ??
        (await ref.read(semesterIdProvider.future)).semesters;
    try {
      state = AsyncData(await _loadSemester(semId, fallbackSems));
    } catch (e, st) {
      state = AsyncError(e, st);
      rethrow;
    }
  }

  Future<void> loadDetails(String courseId, {bool force = false}) async {
    final current = state.valueOrNull;
    if (current == null || courseId.isEmpty) return;
    if (!force &&
        (current.detailsByCourseId.containsKey(courseId) ||
            current.loadingDetailsFor.contains(courseId))) {
      return;
    }

    final loadingSet = {...current.loadingDetailsFor, courseId};
    state = AsyncData(current.copyWith(loadingDetailsFor: loadingSet));

    try {
      await ref.read(vClientProvider.notifier).tryLogin();
      final repo = await ref.read(
        gradesRepositoryForSemProvider(current.selectedSemesterId).future,
      );
      final details = await FetchGradeDetailsUsecase(repo).call(courseId);

      final now = state.valueOrNull ?? current;
      final nextMap = {...now.detailsByCourseId, courseId: details};
      final nextLoading = {...now.loadingDetailsFor}..remove(courseId);
      state = AsyncData(
        now.copyWith(
          detailsByCourseId: nextMap,
          loadingDetailsFor: nextLoading,
        ),
      );
    } catch (_) {
      final now = state.valueOrNull ?? current;
      final nextLoading = {...now.loadingDetailsFor}..remove(courseId);
      state = AsyncData(now.copyWith(loadingDetailsFor: nextLoading));
      rethrow;
    }
  }

  Future<GradesUiState> _loadSemester(
    String semId,
    List<SemesterInfo> semesters, {
    bool forceRemote = false,
  }) async {
    var selected = semId;
    if (selected.isEmpty && semesters.isNotEmpty) {
      selected = semesters.first.id;
    } else if (semesters.isNotEmpty &&
        !semesters.any((s) => s.id == selected)) {
      selected = semesters.first.id;
    }

    final repo = await ref.read(
      gradesRepositoryForSemProvider(selected).future,
    );

    var view = await GetGradeViewUsecase(repo).call();
    final details = await GetGradeDetailsMapUsecase(repo).call();

    if (forceRemote) {
      await _updateView(repo, hasLocalData: view.courses.isNotEmpty);
      view = await GetGradeViewUsecase(repo).call();
    }

    if (view.courses.isEmpty) {
      await _updateView(repo, hasLocalData: false);
      view = await GetGradeViewUsecase(repo).call();
    }

    return GradesUiState(
      gradeView: view,
      semesters: semesters,
      selectedSemesterId: selected,
      detailsByCourseId: details,
      loadingDetailsFor: const <String>{},
    );
  }

  Future<bool> _updateView(
    GradesRepository repo, {
    required bool hasLocalData,
  }) async {
    final gb = await ref.read(gbProvider.future);
    final feature = gb.feature("fetch-grades");
    if (feature.on && feature.value) {
      await ref.read(vClientProvider.notifier).tryLogin();
      await UpdateGradeViewUsecase(repo).call();
      return true;
    } else {
      if (hasLocalData) {
        return false;
      }
      throw FeatureDisabledException("Grades Feature Disabled");
    }
  }
}
