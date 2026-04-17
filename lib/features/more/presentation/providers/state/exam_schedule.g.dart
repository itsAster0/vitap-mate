// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'exam_schedule.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(examScheduleRepository)
final examScheduleRepositoryProvider = ExamScheduleRepositoryProvider._();

final class ExamScheduleRepositoryProvider
    extends
        $FunctionalProvider<
          AsyncValue<ExamScheduleRepository>,
          ExamScheduleRepository,
          FutureOr<ExamScheduleRepository>
        >
    with
        $FutureModifier<ExamScheduleRepository>,
        $FutureProvider<ExamScheduleRepository> {
  ExamScheduleRepositoryProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'examScheduleRepositoryProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$examScheduleRepositoryHash();

  @$internal
  @override
  $FutureProviderElement<ExamScheduleRepository> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<ExamScheduleRepository> create(Ref ref) {
    return examScheduleRepository(ref);
  }
}

String _$examScheduleRepositoryHash() =>
    r'8f539bd7a2f87d8c59861bd8daf50d7b477abbc4';

@ProviderFor(marksRepository)
final marksRepositoryProvider = MarksRepositoryProvider._();

final class MarksRepositoryProvider
    extends
        $FunctionalProvider<
          AsyncValue<MarksRepository>,
          MarksRepository,
          FutureOr<MarksRepository>
        >
    with $FutureModifier<MarksRepository>, $FutureProvider<MarksRepository> {
  MarksRepositoryProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'marksRepositoryProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$marksRepositoryHash();

  @$internal
  @override
  $FutureProviderElement<MarksRepository> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<MarksRepository> create(Ref ref) {
    return marksRepository(ref);
  }
}

String _$marksRepositoryHash() => r'62be906347e2a23f1bf20fdbb23f2797874de780';

@ProviderFor(gradesRepository)
final gradesRepositoryProvider = GradesRepositoryProvider._();

final class GradesRepositoryProvider
    extends
        $FunctionalProvider<
          AsyncValue<GradesRepository>,
          GradesRepository,
          FutureOr<GradesRepository>
        >
    with $FutureModifier<GradesRepository>, $FutureProvider<GradesRepository> {
  GradesRepositoryProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'gradesRepositoryProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$gradesRepositoryHash();

  @$internal
  @override
  $FutureProviderElement<GradesRepository> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<GradesRepository> create(Ref ref) {
    return gradesRepository(ref);
  }
}

String _$gradesRepositoryHash() => r'e8d4f5b2a020eb5eface9448a9dff7d060836b33';

@ProviderFor(gradesRepositoryForSem)
final gradesRepositoryForSemProvider = GradesRepositoryForSemFamily._();

final class GradesRepositoryForSemProvider
    extends
        $FunctionalProvider<
          AsyncValue<GradesRepository>,
          GradesRepository,
          FutureOr<GradesRepository>
        >
    with $FutureModifier<GradesRepository>, $FutureProvider<GradesRepository> {
  GradesRepositoryForSemProvider._({
    required GradesRepositoryForSemFamily super.from,
    required String super.argument,
  }) : super(
         retry: null,
         name: r'gradesRepositoryForSemProvider',
         isAutoDispose: true,
         dependencies: null,
         $allTransitiveDependencies: null,
       );

  @override
  String debugGetCreateSourceHash() => _$gradesRepositoryForSemHash();

  @override
  String toString() {
    return r'gradesRepositoryForSemProvider'
        ''
        '($argument)';
  }

  @$internal
  @override
  $FutureProviderElement<GradesRepository> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<GradesRepository> create(Ref ref) {
    final argument = this.argument as String;
    return gradesRepositoryForSem(ref, argument);
  }

  @override
  bool operator ==(Object other) {
    return other is GradesRepositoryForSemProvider &&
        other.argument == argument;
  }

  @override
  int get hashCode {
    return argument.hashCode;
  }
}

String _$gradesRepositoryForSemHash() =>
    r'4e4999017cc5a31c627bfec5f9b55cdb4f291132';

final class GradesRepositoryForSemFamily extends $Family
    with $FunctionalFamilyOverride<FutureOr<GradesRepository>, String> {
  GradesRepositoryForSemFamily._()
    : super(
        retry: null,
        name: r'gradesRepositoryForSemProvider',
        dependencies: null,
        $allTransitiveDependencies: null,
        isAutoDispose: true,
      );

  GradesRepositoryForSemProvider call(String semid) =>
      GradesRepositoryForSemProvider._(argument: semid, from: this);

  @override
  String toString() => r'gradesRepositoryForSemProvider';
}

@ProviderFor(gradeHistoryRepository)
final gradeHistoryRepositoryProvider = GradeHistoryRepositoryProvider._();

final class GradeHistoryRepositoryProvider
    extends
        $FunctionalProvider<
          AsyncValue<GradeHistoryRepository>,
          GradeHistoryRepository,
          FutureOr<GradeHistoryRepository>
        >
    with
        $FutureModifier<GradeHistoryRepository>,
        $FutureProvider<GradeHistoryRepository> {
  GradeHistoryRepositoryProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'gradeHistoryRepositoryProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$gradeHistoryRepositoryHash();

  @$internal
  @override
  $FutureProviderElement<GradeHistoryRepository> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<GradeHistoryRepository> create(Ref ref) {
    return gradeHistoryRepository(ref);
  }
}

String _$gradeHistoryRepositoryHash() =>
    r'bae07314b22cd54c4a25a9910c64361d27bede99';
