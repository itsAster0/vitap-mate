import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/vtop/backend/vtop_queries.dart';
import 'package:vitapmate/vtop/vtop_page.dart';
import 'package:vitapmate/vtop/semesters.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GradesUiState {
  const GradesUiState(
    this.gradeView,
    this.semesters,
    this.selectedSemesterId,
    this.detailsByCourseId, [
    this.loadingDetailsFor = const {},
  ]);

  final GradeViewData gradeView;
  final List<SemesterInfo> semesters;
  final String selectedSemesterId;
  final Map<String, GradeDetailsData> detailsByCourseId;
  final Set<String> loadingDetailsFor;

  GradesUiState copyWith({
    String? selectedSemesterId,
    Map<String, GradeDetailsData>? detailsByCourseId,
    Set<String>? loadingDetailsFor,
  }) => GradesUiState(
    gradeView,
    semesters,
    selectedSemesterId ?? this.selectedSemesterId,
    detailsByCourseId ?? this.detailsByCourseId,
    loadingDetailsFor ?? this.loadingDetailsFor,
  );
}

final gradesProvider = AsyncNotifierProvider<GradesNotifier, GradesUiState>(
  GradesNotifier.new,
);

class GradesNotifier extends AsyncNotifier<GradesUiState> {
  @override
  Future<GradesUiState> build() async {
    final user = await ref.read(vtopUserProvider.future);
    final sems = (await ref.watch(semesterIdProvider.future)).semesters;
    final initial = await _loadSemester(user.semid ?? "", sems);
    if (initial.gradeView.courses.isNotEmpty) return initial;

    // Grades for the running semester only appear once it ends, so opening
    // the page on it is a dead end. Fall back to the semester before it.
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
    if (state.value case final current?
        when current.selectedSemesterId.isNotEmpty) {
      state = AsyncData(
        await _loadSemester(
          current.selectedSemesterId,
          current.semesters,
          forceRemote: true,
        ),
      );
    }
  }

  Future<void> selectSemester(String semId) async {
    final current = state.value;
    if (current?.selectedSemesterId == semId) return;
    if (current != null) {
      state = AsyncData(current.copyWith(selectedSemesterId: semId));
    }
    final sems =
        current?.semesters ??
        (await ref.read(semesterIdProvider.future)).semesters;
    try {
      state = AsyncData(await _loadSemester(semId, sems));
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
    state = AsyncData(
      current.copyWith(
        loadingDetailsFor: {...current.loadingDetailsFor, courseId},
      ),
    );

    GradeDetailsData? details;
    try {
      final query = VtopQueries.gradeDetails(
        current.selectedSemesterId,
        courseId: courseId,
      );
      details = await (force ? fetchPage(ref, query) : loadPage(ref, query));
    } finally {
      // Success or failure, the course stops loading.
      final now = state.value ?? current;
      state = AsyncData(
        now.copyWith(
          detailsByCourseId: {...now.detailsByCourseId, courseId: ?details},
          loadingDetailsFor: now.loadingDetailsFor.difference({courseId}),
        ),
      );
    }
  }

  Future<GradesUiState> _loadSemester(
    String semId,
    List<SemesterInfo> semesters, {
    bool forceRemote = false,
  }) async {
    final selected =
        semesters.isNotEmpty &&
            (semId.isEmpty || !semesters.any((s) => s.id == semId))
        ? semesters.first.id
        : semId;
    final query = VtopQueries.grades(selected);
    final view = await (forceRemote
        ? fetchPage(ref, query)
        : loadPage(ref, query));
    return GradesUiState(view, semesters, selected, {
      for (final course in view.courses)
        course.courseId: ?await readSavedPage(
          ref.read,
          VtopQueries.gradeDetails(selected, courseId: course.courseId),
        ),
    });
  }
}
