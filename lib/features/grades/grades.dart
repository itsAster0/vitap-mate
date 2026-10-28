import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/vtop_page.dart';
import 'package:vitapmate/vtop/semesters.dart';
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
    final initial = await _loadSemester(semId, semData.semesters);
    if (initial.gradeView.courses.isNotEmpty) return initial;

    // Grades for the running semester only appear once it ends, so opening
    // the page on it is a dead end. Fall back to the semester before it.
    final sems = semData.semesters;
    final index = sems.indexWhere((s) => s.id == initial.selectedSemesterId);
    if (index < 0 || index + 1 >= sems.length) return initial;
    try {
      final previous = await _loadSemester(sems[index + 1].id, sems);
      return previous.gradeView.courses.isNotEmpty ? previous : initial;
    } catch (_) {
      return initial;
    }
  }

  Future<void> refresh() async {
    final current = state.value;
    final semId = current?.selectedSemesterId;
    if (semId == null || semId.isEmpty) return;

    final next = await _loadSemester(
      semId,
      current!.semesters,
      forceRemote: true,
    );
    state = AsyncData(next);
  }

  Future<void> selectSemester(String semId) async {
    final current = state.value;
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
    final current = state.value;
    if (current == null || courseId.isEmpty) return;
    if (!force &&
        (current.detailsByCourseId.containsKey(courseId) ||
            current.loadingDetailsFor.contains(courseId))) {
      return;
    }

    final loadingSet = {...current.loadingDetailsFor, courseId};
    state = AsyncData(current.copyWith(loadingDetailsFor: loadingSet));

    try {
      final query = VtopQueries.gradeDetails(
        current.selectedSemesterId,
        courseId: courseId,
      );
      final details = force
          ? await fetchPage(ref, query)
          : await loadPage(ref, query);
      final now = state.value ?? current;
      final nextMap = {...now.detailsByCourseId, courseId: details};
      final nextLoading = {...now.loadingDetailsFor}..remove(courseId);
      state = AsyncData(
        now.copyWith(
          detailsByCourseId: nextMap,
          loadingDetailsFor: nextLoading,
        ),
      );
    } catch (_) {
      final now = state.value ?? current;
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

    final query = VtopQueries.grades(selected);
    final view = forceRemote
        ? await fetchPage(ref, query)
        : await loadPage(ref, query);
    final details = <String, GradeDetailsData>{
      for (final course in view.courses)
        course.courseId: ?await readSavedPage(
          ref.read,
          VtopQueries.gradeDetails(selected, courseId: course.courseId),
        ),
    };
    return GradesUiState(
      gradeView: view,
      semesters: semesters,
      selectedSemesterId: selected,
      detailsByCourseId: details,
      loadingDetailsFor: const <String>{},
    );
  }
}
