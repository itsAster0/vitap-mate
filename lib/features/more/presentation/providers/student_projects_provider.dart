import 'dart:async';
import 'dart:convert';

import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';

part 'student_projects_provider.g.dart';

const defaultStudentProjectsSchemaJson = <String, Object?>{
  "schemaVersion": 2,
  "rotation": {"enabled": true, "mode": "withinCategory"},
  "cta": {
    "label": "Want your project here? Contact @itsKryxen",
    "url": "https://www.instagram.com/itsKryxen",
  },
  "categories": [
    {
      "id": "academics",
      "label": "Academics",
      "description": "Study tools, grades, GPA, planning, and coursework.",
    },
    {
      "id": "campus",
      "label": "Campus",
      "description":
          "Tools for travel, hostel life, events, and campus utilities.",
    },
    {
      "id": "miscellaneous",
      "label": "Miscellaneous",
      "description":
          "Useful student-built experiments, community tools, and utilities.",
    },
  ],
  "projects": [
    {
      "id": 1001,
      "name": "GPA Calculator",
      "summary": "Calculate GPA/CGPA quickly for VIT-AP students.",
      "type": "Website",
      "category": "academics",
      "madeBy": "Venkatsubash07",
      "url": "https://vitapcgpacalculator.vercel.app/",
      "previewImage": null,
      "sourceUrl": "https://github.com/Venkatsubash07/vitapcgpacalculator",
      "status": "active",
      "featured": false,
      "tags": ["GPA", "CGPA", "Academics"],
    },
    {
      "id": 1002,
      "name": "GPA Calculator",
      "summary": "VIT-AP GPA calculator app for VIT-AP students.",
      "type": "App",
      "category": "academics",
      "madeBy": "Maddikeri Narendranath Reddy",
      "url": "https://play.google.com/store/apps/details?id=com.gcalc.gcalcpro",
      "previewImage": null,
      "sourceUrl": null,
      "status": "active",
      "featured": false,
      "tags": ["GPA", "App", "Academics"],
    },
    {
      "id": 1003,
      "name": "VIT-AP Confessions",
      "summary": "Anonymous confessions platform for VIT-AP students.",
      "type": "Website",
      "category": "miscellaneous",
      "madeBy": "Abhay Agnihotri",
      "url": "https://vit-ap-confessions.netlify.app",
      "previewImage": null,
      "sourceUrl": "https://github.com/Notxabhay16/Vit-ap-confessions",
      "status": "active",
      "featured": false,
      "tags": ["Anonymous", "Community", "Confessions"],
    },
    {
      "id": 1004,
      "name": "VIT-AP Travel Buddy",
      "summary": "Travel planning and ride-sharing helper for VIT-AP students.",
      "type": "Website",
      "category": "campus",
      "madeBy": "Srijita",
      "url": "https://vitap-travel-buddy-beta.netlify.app/",
      "previewImage": null,
      "sourceUrl": null,
      "status": "active",
      "featured": false,
      "tags": ["Travel", "Rides", "Campus"],
    },
    {
      "id": 1006,
      "name": "KnowYourFaculty",
      "summary":
          "Anonymous faculty reviews and ratings platform for VIT-AP students.",
      "type": "Website",
      "category": "academics",
      "madeBy": "Jishith K",
      "url": "https://knowyourfaculty.vercel.app",
      "previewImage": null,
      "sourceUrl": "https://github.com/notvenu/KnowYourFaculty",
      "status": "active",
      "featured": false,
      "tags": ["Faculty", "Reviews", "Academics"],
    },
    {
      "id": 1008,
      "name": "Campus Events",
      "summary":
          "Centralized event portal for VIT-AP students to discover upcoming campus events, key details, and schedules in one place.",
      "type": "Website",
      "category": "campus",
      "madeBy": "Pranjal",
      "url": "https://events.pranjalk.tech/",
      "previewImage": null,
      "sourceUrl": null,
      "status": "active",
      "featured": false,
      "tags": ["Events", "Campus", "Schedule"],
    },
    {
      "id": 1009,
      "name": "GPA Calculator",
      "summary":
          "Simple GPA calculator for VIT-AP students to quickly compute semester GPA based on grades and credits.",
      "type": "Website",
      "category": "academics",
      "madeBy": "Sai KondaReddy",
      "url": "https://sai630414.github.io/gpa-calculator/index.html",
      "previewImage": null,
      "sourceUrl": null,
      "status": "active",
      "featured": false,
      "tags": ["GPA", "Credits", "Academics"],
    },
    {
      "id": 1010,
      "name": "Lost & Found Portal",
      "summary":
          "Campus lost-and-found portal for VIT-AP students to report, browse, and reconnect lost items with their owners.",
      "type": "Website",
      "category": "campus",
      "madeBy": "Jayasree",
      "url": "https://lost-found-cemp.vercel.app/",
      "previewImage": null,
      "sourceUrl": null,
      "status": "active",
      "featured": false,
      "tags": ["Lost Found", "Campus", "Items"],
    },
    {
      "id": 1011,
      "name": "WeGoVroom",
      "summary":
          "Student app for VIT-AP with campus-focused utility features, available on the Play Store.",
      "type": "App",
      "category": "campus",
      "madeBy": "WeGoVroom Team",
      "url": "https://play.google.com/store/apps/details?id=com.wegovroom.app",
      "previewImage": null,
      "sourceUrl": null,
      "status": "active",
      "featured": false,
      "tags": ["App", "Utilities", "Campus"],
    },
    {
      "id": 1012,
      "name": "VIT-AP Updates",
      "summary":
          "Student portal for VIT-AP with mess menu, CGPA, attendance, and student-connect utilities in one place.",
      "type": "Website",
      "category": "campus",
      "madeBy": "VIT-AP Updates Team",
      "url": "https://lovevitap.purlyedit.in/student.php",
      "previewImage": null,
      "sourceUrl": null,
      "status": "active",
      "featured": false,
      "tags": ["Mess", "Attendance", "Updates"],
    },
  ],
};

final defaultStudentProjectsData = StudentProjectsData.fromJson(
  defaultStudentProjectsSchemaJson,
)!;

@riverpod
class StudentProjects extends _$StudentProjects {
  static const _cacheKey = 'settings_student_projects_json';
  static const _featureKey = 'student-projects';
  static const _rotationSeedKey = 'settings_student_projects_rotation_seed';

  @override
  Future<StudentProjectsData> build() async {
    final prefs = await ref.read(settingsProvider.future);
    final rotationSeed = prefs.getInt(_rotationSeedKey) ?? 0;
    final cached = await _readFromCache();
    final initial = _applyRotation(cached, rotationSeed);
    await prefs.setInt(_rotationSeedKey, rotationSeed + 1);
    unawaited(
      _refreshFromFlagsmith(current: initial, rotationSeed: rotationSeed),
    );
    return initial;
  }

  Future<void> refresh() async {
    final current = state.value ?? defaultStudentProjectsData;
    final prefs = await ref.read(settingsProvider.future);
    final nextRotationSeed = prefs.getInt(_rotationSeedKey) ?? 0;
    final activeRotationSeed = nextRotationSeed > 0 ? nextRotationSeed - 1 : 0;
    await _refreshFromFlagsmith(
      current: current,
      forceStateUpdate: true,
      rotationSeed: activeRotationSeed,
    );
  }

  Future<StudentProjectsData> _readFromCache() async {
    try {
      final prefs = await ref.read(settingsProvider.future);
      final raw = prefs.getString(_cacheKey);
      if (raw == null || raw.isEmpty) {
        return defaultStudentProjectsData;
      }
      final decoded = jsonDecode(raw);
      return _parsePayload(decoded) ?? defaultStudentProjectsData;
    } catch (_) {
      return defaultStudentProjectsData;
    }
  }

  Future<void> _refreshFromFlagsmith({
    required StudentProjectsData current,
    required int rotationSeed,
    bool forceStateUpdate = false,
  }) async {
    try {
      final featureFlags = await ref.read(
        featureFlagsControllerProvider.future,
      );
      if (!await featureFlags.isEnabled(_featureKey)) return;

      final parsed = _parsePayload(await featureFlags.value(_featureKey));
      if (parsed == null || parsed.projects.isEmpty) return;

      final rotated = _applyRotation(parsed, rotationSeed);
      final prefs = await ref.read(settingsProvider.future);
      await prefs.setString(_cacheKey, jsonEncode(parsed.toJson()));

      if (forceStateUpdate || current != rotated) {
        state = AsyncData(rotated);
      }
    } catch (_) {}
  }

  StudentProjectsData? _parsePayload(dynamic payload) {
    if (payload is Map) {
      return StudentProjectsData.fromJson(Map<String, dynamic>.from(payload));
    }
    if (payload is List) {
      final projects = <StudentProject>[];
      for (final item in payload) {
        if (item is! Map) continue;
        final parsed = StudentProject.fromJson(Map<String, dynamic>.from(item));
        if (parsed != null) {
          projects.add(parsed);
        }
      }
      if (projects.isEmpty) return null;
      return StudentProjectsData(
        schemaVersion: 1,
        rotationEnabled: true,
        rotationMode: RotationMode.withinCategory,
        cta: const StudentProjectsCta(
          label: "Want your project here? Contact @itsKryxen",
          url: "https://www.instagram.com/itsKryxen",
        ),
        categories: StudentProjectCategory.defaultCategories,
        projects: projects,
      );
    }
    return null;
  }

  StudentProjectsData _applyRotation(StudentProjectsData input, int seed) {
    if (!input.rotationEnabled || input.projects.length < 2) return input;
    if (input.rotationMode == RotationMode.withinCategory) {
      final grouped = <String, List<StudentProject>>{};
      for (final p in input.projects) {
        grouped.putIfAbsent(p.category, () => <StudentProject>[]).add(p);
      }
      final knownOrder = [
        for (final c in input.categories) c.id,
        for (final key in grouped.keys)
          if (!input.categories.any((c) => c.id == key)) key,
      ];
      final rotated = <StudentProject>[];
      for (final key in knownOrder) {
        final items = grouped[key];
        if (items == null || items.isEmpty) continue;
        rotated.addAll(_rotateList(items, seed));
      }
      return input.copyWith(projects: rotated);
    }
    return input.copyWith(projects: _rotateList(input.projects, seed));
  }

  List<StudentProject> _rotateList(List<StudentProject> items, int seed) {
    if (items.length < 2) return items;
    final offset = seed % items.length;
    if (offset == 0) return items;
    return [...items.sublist(offset), ...items.sublist(0, offset)];
  }
}

enum RotationMode { withinCategory, global }

class StudentProjectsData {
  final int schemaVersion;
  final bool rotationEnabled;
  final RotationMode rotationMode;
  final StudentProjectsCta cta;
  final List<StudentProjectCategory> categories;
  final List<StudentProject> projects;

  const StudentProjectsData({
    required this.schemaVersion,
    required this.rotationEnabled,
    required this.rotationMode,
    required this.cta,
    required this.categories,
    required this.projects,
  });

  StudentProjectsData copyWith({
    int? schemaVersion,
    bool? rotationEnabled,
    RotationMode? rotationMode,
    StudentProjectsCta? cta,
    List<StudentProjectCategory>? categories,
    List<StudentProject>? projects,
  }) {
    return StudentProjectsData(
      schemaVersion: schemaVersion ?? this.schemaVersion,
      rotationEnabled: rotationEnabled ?? this.rotationEnabled,
      rotationMode: rotationMode ?? this.rotationMode,
      cta: cta ?? this.cta,
      categories: categories ?? this.categories,
      projects: projects ?? this.projects,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'schemaVersion': schemaVersion,
      'rotation': {
        'enabled': rotationEnabled,
        'mode': rotationMode == RotationMode.global
            ? 'global'
            : 'withinCategory',
      },
      'cta': cta.toJson(),
      'categories': [for (final c in categories) c.toJson()],
      'projects': [for (final p in projects) p.toJson()],
    };
  }

  static StudentProjectsData? fromJson(Map<String, dynamic> json) {
    final schemaVersion = (json['schemaVersion'] as num?)?.toInt() ?? 1;
    final rotation = json['rotation'];
    final rotationEnabled = rotation is Map
        ? (rotation['enabled'] as bool?) ?? true
        : true;
    final modeRaw = rotation is Map ? rotation['mode'] : null;
    final mode = modeRaw == 'global'
        ? RotationMode.global
        : RotationMode.withinCategory;

    final cta = StudentProjectsCta.fromJson(
      json['cta'] is Map<String, dynamic>
          ? json['cta'] as Map<String, dynamic>
          : const {},
    );

    final categoriesRaw = json['categories'];
    final categories = <StudentProjectCategory>[];
    if (categoriesRaw is List) {
      for (final item in categoriesRaw) {
        if (item is! Map) continue;
        final parsed = StudentProjectCategory.fromJson(
          Map<String, dynamic>.from(item),
        );
        if (parsed != null) {
          categories.add(parsed);
        }
      }
    }

    dynamic projectsRaw = json['projects'];
    projectsRaw ??= json['studentProjects'] ?? json['items'] ?? json['data'];
    if (projectsRaw is! List) return null;

    final projects = <StudentProject>[];
    for (final item in projectsRaw) {
      if (item is! Map) continue;
      final parsed = StudentProject.fromJson(Map<String, dynamic>.from(item));
      if (parsed != null) {
        projects.add(parsed);
      }
    }
    if (projects.isEmpty) return null;

    return StudentProjectsData(
      schemaVersion: schemaVersion,
      rotationEnabled: rotationEnabled,
      rotationMode: mode,
      cta: cta,
      categories: categories.isEmpty
          ? StudentProjectCategory.defaultCategories
          : categories,
      projects: projects,
    );
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    return other is StudentProjectsData &&
        other.schemaVersion == schemaVersion &&
        other.rotationEnabled == rotationEnabled &&
        other.rotationMode == rotationMode &&
        other.cta == cta &&
        _listEquals(other.categories, categories) &&
        _listEquals(other.projects, projects);
  }

  @override
  int get hashCode {
    return Object.hash(
      schemaVersion,
      rotationEnabled,
      rotationMode,
      cta,
      Object.hashAll(categories),
      Object.hashAll(projects),
    );
  }
}

class StudentProjectsCta {
  final String label;
  final String url;

  const StudentProjectsCta({required this.label, required this.url});

  Map<String, dynamic> toJson() => {'label': label, 'url': url};

  static StudentProjectsCta fromJson(Map<String, dynamic> json) {
    final label = (json['label'] as String?)?.trim();
    final url = (json['url'] as String?)?.trim();
    return StudentProjectsCta(
      label: (label == null || label.isEmpty)
          ? "Want your project here? Contact @itsKryxen"
          : label,
      url: (url == null || url.isEmpty)
          ? "https://www.instagram.com/itsKryxen"
          : url,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is StudentProjectsCta && other.label == label && other.url == url;

  @override
  int get hashCode => Object.hash(label, url);
}

class StudentProjectCategory {
  static const categoryAcademics = 'academics';
  static const categoryCampus = 'campus';
  static const categoryMiscellaneous = 'miscellaneous';

  final String id;
  final String label;
  final String description;

  const StudentProjectCategory({
    required this.id,
    required this.label,
    required this.description,
  });

  static const defaultCategories = [
    StudentProjectCategory(
      id: categoryAcademics,
      label: 'Academics',
      description: 'Study tools, grades, GPA, planning, and coursework.',
    ),
    StudentProjectCategory(
      id: categoryCampus,
      label: 'Campus',
      description:
          'Tools for travel, hostel life, events, and campus utilities.',
    ),
    StudentProjectCategory(
      id: categoryMiscellaneous,
      label: 'Miscellaneous',
      description:
          'Useful student-built experiments, community tools, and utilities.',
    ),
  ];

  Map<String, dynamic> toJson() => {
    'id': id,
    'label': label,
    'description': description,
  };

  static StudentProjectCategory? fromJson(Map<String, dynamic> json) {
    final rawId = json['id'];
    final rawLabel = json['label'];
    final rawDescription = json['description'];
    if (rawId is! String || rawLabel is! String || rawDescription is! String) {
      return null;
    }
    return StudentProjectCategory(
      id: StudentProject.normalizeCategory(rawId),
      label: rawLabel,
      description: rawDescription,
    );
  }

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is StudentProjectCategory &&
          other.id == id &&
          other.label == label &&
          other.description == description;

  @override
  int get hashCode => Object.hash(id, label, description);
}

class StudentProject {
  final int id;
  final String name;
  final String type;
  final String summary;
  final String madeBy;
  final String url;
  final String category;
  final String? previewImage;
  final String? sourceUrl;
  final String status;
  final bool featured;
  final List<String> tags;

  const StudentProject({
    required this.id,
    required this.name,
    required this.type,
    required this.summary,
    required this.madeBy,
    required this.url,
    required this.category,
    required this.previewImage,
    required this.sourceUrl,
    required this.status,
    required this.featured,
    required this.tags,
  });

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'type': type,
    'summary': summary,
    'madeBy': madeBy,
    'url': url,
    'category': category,
    'previewImage': previewImage,
    'sourceUrl': sourceUrl,
    'status': status,
    'featured': featured,
    'tags': tags,
  };

  static StudentProject? fromJson(Map<String, dynamic> json) {
    final id = json['id'];
    final name = json['name'];
    final type = json['type'];
    final summary = json['summary'] ?? json['desc'];
    final madeBy = json['madeBy'];
    final url = json['url'];
    final category = json['category'];

    if (id is! num ||
        name is! String ||
        type is! String ||
        summary is! String ||
        madeBy is! String ||
        url is! String) {
      return null;
    }

    final sourceRaw = json['sourceUrl'] ?? json['openSource'];
    final sourceUrl = sourceRaw is String && sourceRaw.trim().isNotEmpty
        ? sourceRaw.trim()
        : null;
    final previewRaw = json['previewImage'];
    final previewImage = previewRaw is String && previewRaw.trim().isNotEmpty
        ? previewRaw.trim()
        : null;
    final statusRaw = json['status'];
    final featuredRaw = json['featured'];
    final tagsRaw = json['tags'];

    return StudentProject(
      id: id.toInt(),
      name: name,
      type: type,
      summary: summary,
      madeBy: madeBy,
      url: url,
      category: category is String
          ? normalizeCategory(category)
          : StudentProjectCategory.categoryMiscellaneous,
      previewImage: previewImage,
      sourceUrl: sourceUrl,
      status: statusRaw is String && statusRaw.trim().isNotEmpty
          ? statusRaw.trim()
          : 'active',
      featured: featuredRaw is bool ? featuredRaw : false,
      tags: [
        if (tagsRaw is List)
          for (final tag in tagsRaw)
            if (tag is String && tag.trim().isNotEmpty) tag.trim(),
      ],
    );
  }

  static String normalizeCategory(String raw) {
    final value = raw.trim().toLowerCase();
    if (value == StudentProjectCategory.categoryAcademics ||
        value == 'academic' ||
        value == 'acadmiics') {
      return StudentProjectCategory.categoryAcademics;
    }
    if (value == StudentProjectCategory.categoryCampus) {
      return StudentProjectCategory.categoryCampus;
    }
    if (value == StudentProjectCategory.categoryMiscellaneous ||
        value == 'misc' ||
        value == 'miscilanius' ||
        value == 'miscellaneouss') {
      return StudentProjectCategory.categoryMiscellaneous;
    }
    return StudentProjectCategory.categoryMiscellaneous;
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    return other is StudentProject &&
        other.id == id &&
        other.name == name &&
        other.type == type &&
        other.summary == summary &&
        other.madeBy == madeBy &&
        other.url == url &&
        other.category == category &&
        other.previewImage == previewImage &&
        other.sourceUrl == sourceUrl &&
        other.status == status &&
        other.featured == featured &&
        _listEquals(other.tags, tags);
  }

  @override
  int get hashCode {
    return Object.hash(
      id,
      name,
      type,
      summary,
      madeBy,
      url,
      category,
      previewImage,
      sourceUrl,
      status,
      featured,
      Object.hashAll(tags),
    );
  }
}

bool _listEquals<T>(List<T> a, List<T> b) {
  if (identical(a, b)) return true;
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}
