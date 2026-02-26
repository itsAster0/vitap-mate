import 'dart:async';
import 'dart:convert';

import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
part 'student_projects_provider.g.dart';

const defaultStudentProjectsJson = <Map<String, Object?>>[
  {
    'id': 1001,
    'name': 'GPA Calculator',
    'type': 'Website',
    'desc': 'Calculate GPA/CGPA quickly for VIT-AP students.',
    'madeBy': 'Venkatsubash07',
    'url': 'https://vitapcgpacalculator.vercel.app/',
    'category': 'academics',
    'openSource': null,
  },
  {
    'id': 1002,
    'name': 'GPA Calculator',
    'type': 'App',
    'desc': 'VIT-AP GPA calculator app for VIT-AP students.',
    'madeBy': 'Maddikeri Narendranath Reddy',
    'url': 'https://play.google.com/store/apps/details?id=com.gcalc.gcalcpro',
    'category': 'academics',
    'openSource': null,
  },
];

const defaultStudentProjects = <StudentProject>[
  StudentProject(
    id: 1001,
    name: 'GPA Calculator',
    type: 'Website',
    desc: 'Calculate GPA/CGPA quickly for VIT-AP students.',
    madeBy: 'Venkatsubash07',
    url: 'https://vitapcgpacalculator.vercel.app/',
    category: StudentProject.categoryAcademics,
    openSource: null,
  ),
  StudentProject(
    id: 1002,
    name: 'GPA Calculator',
    type: 'App',
    desc: 'VIT-AP GPA calculator app for VIT-AP students.',
    madeBy: 'Maddikeri Narendranath Reddy',
    url: 'https://play.google.com/store/apps/details?id=com.gcalc.gcalcpro',
    category: StudentProject.categoryAcademics,
    openSource: null,
  ),
];

@riverpod
class StudentProjects extends _$StudentProjects {
  static const _cacheKey = 'settings_student_projects_json';
  static const _featureKey = 'student-projects';

  @override
  Future<List<StudentProject>> build() async {
    final cached = await _readFromCache();
    final initial = cached.isNotEmpty ? cached : defaultStudentProjects;
    unawaited(_refreshFromGb(current: initial));
    return initial;
  }

  Future<void> refresh() async {
    final current = state.valueOrNull ?? defaultStudentProjects;
    await _refreshFromGb(current: current, forceStateUpdate: true);
  }

  Future<List<StudentProject>> _readFromCache() async {
    try {
      final prefs = await ref.read(settingsProvider.future);
      final raw = prefs.getString(_cacheKey);
      if (raw == null || raw.isEmpty) {
        return _parseProjectsPayload(defaultStudentProjectsJson);
      }
      final decoded = jsonDecode(raw);
      return _parseProjectsPayload(decoded);
    } catch (_) {
      return _parseProjectsPayload(defaultStudentProjectsJson);
    }
  }

  Future<void> _refreshFromGb({
    required List<StudentProject> current,
    bool forceStateUpdate = false,
  }) async {
    try {
      final gb = await ref.read(gbProvider.future);
      final feature = gb.feature(_featureKey);
      if (!feature.on) return;

      final remoteProjects = _parseProjectsPayload(feature.value);
      if (remoteProjects.isEmpty) return;

      final prefs = await ref.read(settingsProvider.future);
      await prefs.setString(
        _cacheKey,
        jsonEncode(remoteProjects.map((p) => p.toJson()).toList()),
      );

      final shouldUpdate =
          forceStateUpdate || !_sameProjects(current, remoteProjects);
      if (shouldUpdate) {
        state = AsyncData(remoteProjects);
      }
    } catch (_) {}
  }

  List<StudentProject> _parseProjectsPayload(dynamic payload) {
    dynamic raw = payload;
    if (raw is Map<String, dynamic>) {
      raw =
          raw['projects'] ??
          raw['studentProjects'] ??
          raw['data'] ??
          raw['items'];
    }

    if (raw is! List) return const [];

    final out = <StudentProject>[];
    for (final item in raw) {
      if (item is! Map) continue;
      final project = StudentProject.fromJson(Map<String, dynamic>.from(item));
      if (project != null) {
        out.add(project);
      }
    }
    return out;
  }

  bool _sameProjects(List<StudentProject> a, List<StudentProject> b) {
    if (identical(a, b)) return true;
    if (a.length != b.length) return false;
    for (var i = 0; i < a.length; i++) {
      if (a[i] != b[i]) return false;
    }
    return true;
  }
}

class StudentProject {
  static const categoryAcademics = 'academics';
  static const categoryCampus = 'campus';
  static const categoryMiscellaneous = 'miscellaneous';

  final int id;
  final String name;
  final String type;
  final String desc;
  final String madeBy;
  final String url;
  final String category;
  final String? openSource;

  const StudentProject({
    required this.id,
    required this.name,
    required this.type,
    required this.desc,
    required this.madeBy,
    required this.url,
    required this.category,
    required this.openSource,
  });

  Map<String, dynamic> toJson() {
    return {
      'id': id,
      'name': name,
      'type': type,
      'desc': desc,
      'madeBy': madeBy,
      'url': url,
      'category': category,
      'openSource': openSource,
    };
  }

  static StudentProject? fromJson(Map<String, dynamic> json) {
    final id = json['id'];
    final name = json['name'];
    final type = json['type'];
    final desc = json['desc'];
    final madeBy = json['madeBy'];
    final url = json['url'];
    final category = json['category'];
    final openSource = json['openSource'];

    if (id is! num ||
        name is! String ||
        type is! String ||
        desc is! String ||
        madeBy is! String ||
        url is! String) {
      return null;
    }

    final openSourceUrl =
        openSource is String && openSource.trim().isNotEmpty
            ? openSource.trim()
            : null;
    final categoryKey =
        category is String
            ? _normalizeCategory(category)
            : categoryMiscellaneous;

    return StudentProject(
      id: id.toInt(),
      name: name,
      type: type,
      desc: desc,
      madeBy: madeBy,
      url: url,
      category: categoryKey,
      openSource: openSourceUrl,
    );
  }

  static String _normalizeCategory(String raw) {
    final value = raw.trim().toLowerCase();
    if (value == categoryAcademics ||
        value == 'academic' ||
        value == 'acadmiics') {
      return categoryAcademics;
    }
    if (value == categoryCampus) return categoryCampus;
    if (value == categoryMiscellaneous ||
        value == 'misc' ||
        value == 'miscilanius' ||
        value == 'miscellaneouss') {
      return categoryMiscellaneous;
    }
    return categoryMiscellaneous;
  }

  String get categoryLabel {
    switch (category) {
      case categoryAcademics:
        return 'Academics';
      case categoryCampus:
        return 'Campus';
      case categoryMiscellaneous:
      default:
        return 'Miscellaneous';
    }
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    return other is StudentProject &&
        other.id == id &&
        other.name == name &&
        other.type == type &&
        other.desc == desc &&
        other.madeBy == madeBy &&
        other.url == url &&
        other.category == category &&
        other.openSource == openSource;
  }

  @override
  int get hashCode {
    return Object.hash(id, name, type, desc, madeBy, url, category, openSource);
  }
}
