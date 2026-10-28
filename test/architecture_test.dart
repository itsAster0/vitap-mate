// Import rules for lib/ (see docs/plans/flutter-refactor.md):
// - a feature imports another feature only through its public file,
//   features/<name>/<name>.dart, never its pages, widgets or logic;
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

/// Imports of another feature's private files still to untangle (none
/// left; keep it that way).
const _privateImportsAllowed = <String>{};

/// Files outside vtop/ that still call the Rust bindings directly.
const _bindingsAllowed = {
  'features/onboarding/onboarding_page.dart',
  'features/account/email_otp_support.dart',
  'features/server/server_account.dart',
  'platform/background/stale_refresh.dart',
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

  test('features import each other only through public files', () {
    final private = <String>{
      for (final MapEntry(key: path, value: deps) in imports.entries)
        if (path.startsWith('features/'))
          for (final dep in deps)
            if (dep.split('/') case ['features', final other, ...]
                when other != path.split('/')[1] &&
                    dep != 'features/$other/$other.dart')
              '$path -> $dep',
    };
    expect(
      private.difference(_privateImportsAllowed),
      isEmpty,
      reason: 'import the feature\'s public file instead',
    );
    expect(
      _privateImportsAllowed.difference(private),
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
