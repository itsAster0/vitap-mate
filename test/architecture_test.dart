// Import rules for lib/ (see docs/plans/flutter-refactor.md):
// - a feature never imports another feature;
// - ui/ imports only ui/ and the generated data types;
// - only vtop/ touches the generated Rust bindings beyond the data types.
//
// The allowlists hold what is left to untangle. They only shrink: the test
// fails when an import is added and when a listed one is gone.

import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

const _sharedBindings = {
  'src/api/vtop/types.dart',
  'src/api/vtop/vtop_errors.dart',
};

/// "feature -> feature" pairs still to untangle.
const _crossFeatureAllowed = {
  'attendance -> calendar',
  'attendance -> timetable',
  'calendar -> timetable',
  'course_page -> timetable',
  'more -> calendar',
  'more -> settings',
  'more -> timetable',
  'outing -> docs',
  'settings -> attendance',
  'settings -> calendar',
  'settings -> docs',
  'settings -> more',
  'settings -> timetable',
  'timetable -> attendance',
  'timetable -> calendar',
  'timetable -> more',
};

/// Files outside vtop/ that still call the Rust bindings directly.
const _bindingsAllowed = {
  'features/onboarding/onboarding_page.dart',
  'features/settings/presentation/email_otp_support.dart',
  'features/settings/presentation/pages/logs_page.dart',
  'features/settings/presentation/providers/server_account.dart',
  'main.dart',
  'platform/background/stale_refresh.dart',
  'platform/background/sync.dart',
  'platform/fcm_cookie_bridge_service.dart',
};

void main() {
  final imports = <String, List<String>>{};
  for (final file in Directory('lib').listSync(recursive: true)) {
    final path = file.path.substring('lib/'.length);
    if (file is! File ||
        !path.endsWith('.dart') ||
        path.startsWith('src/') ||
        path.endsWith('.g.dart') ||
        path.endsWith('.freezed.dart')) {
      continue;
    }
    imports[path] =
        RegExp(r"^import 'package:vitapmate/([^']+)'", multiLine: true)
            .allMatches(file.readAsStringSync())
            .map((match) => match.group(1)!)
            .toList();
  }

  test('features do not import each other', () {
    final pairs = <String>{
      for (final MapEntry(key: path, value: deps) in imports.entries)
        if (path.startsWith('features/'))
          for (final dep in deps)
            if (dep.startsWith('features/') &&
                dep.split('/')[1] != path.split('/')[1])
              '${path.split('/')[1]} -> ${dep.split('/')[1]}',
    };
    expect(
      pairs.difference(_crossFeatureAllowed),
      isEmpty,
      reason: 'new cross-feature imports',
    );
    expect(
      _crossFeatureAllowed.difference(pairs),
      isEmpty,
      reason: 'untangled: remove from the allowlist',
    );
  });

  test('ui/ stands alone', () {
    final bad = [
      for (final MapEntry(key: path, value: deps) in imports.entries)
        if (path.startsWith('ui/'))
          for (final dep in deps)
            if (!dep.startsWith('ui/') && dep != 'src/api/vtop/types.dart')
              '$path -> $dep',
    ];
    expect(bad, isEmpty);
  });

  test('only vtop/ calls the Rust bindings', () {
    final files = <String>{
      for (final MapEntry(key: path, value: deps) in imports.entries)
        if (!path.startsWith('vtop/'))
          for (final dep in deps)
            if (dep.startsWith('src/') && !_sharedBindings.contains(dep)) path,
    };
    expect(
      files.difference(_bindingsAllowed),
      isEmpty,
      reason: 'new direct binding calls',
    );
    expect(
      _bindingsAllowed.difference(files),
      isEmpty,
      reason: 'cleaned up: remove from the allowlist',
    );
  });
}
