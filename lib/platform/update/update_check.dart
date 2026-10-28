import 'dart:convert';
import 'dart:io';

import 'package:in_app_update/in_app_update.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/vtop/http/app_http_client.dart';

/// The commit this build was made from; CI passes it with
/// `--dart-define=GIT_SHA=…`. Empty in local builds.
const buildCommit = String.fromEnvironment('GIT_SHA');

/// The GitHub release that every push to main replaces (see
/// .github/workflows/android-build.yml).
final _nightlyRelease = Uri.parse(
  'https://api.github.com/repos/itsKryxen/vitap-mate/releases/tags/nightly',
);

enum UpdateSource { playStore, github }

class AppUpdateStatus {
  const AppUpdateStatus({
    required this.source,
    required this.available,
    this.latest,
    this.url,
  });

  final UpdateSource source;

  /// Null when it cannot be told: a GitHub check from a build without
  /// [buildCommit].
  final bool? available;

  /// The newest build: its Play version code or nightly commit.
  final String? latest;

  /// Where the newest GitHub build is downloaded.
  final Uri? url;
}

/// Asks Google Play when the app was installed from it, otherwise compares
/// this build's commit with GitHub's nightly release.
Future<AppUpdateStatus> checkForAppUpdate() async {
  final info = await PackageInfo.fromPlatform();
  if (Platform.isAndroid && info.installerStore == 'com.android.vending') {
    final update = await InAppUpdate.checkForUpdate();
    return AppUpdateStatus(
      source: UpdateSource.playStore,
      available:
          update.updateAvailability == UpdateAvailability.updateAvailable,
      latest: update.availableVersionCode?.toString(),
    );
  }

  final response = await sharedAppHttpClient.get(
    _nightlyRelease,
    headers: {'accept': 'application/vnd.github+json'},
  );
  if (response.statusCode != 200) {
    throw HttpException('GitHub answered ${response.statusCode}');
  }
  final release = jsonDecode(response.body) as Map<String, dynamic>;
  final commit = release['target_commitish'] as String? ?? '';
  return AppUpdateStatus(
    source: UpdateSource.github,
    available: buildCommit.isEmpty ? null : commit != buildCommit,
    latest: commit.length > 7 ? commit.substring(0, 7) : commit,
    url: Uri.tryParse(release['html_url'] as String? ?? ''),
  );
}

/// Starts Play's update flow, or opens the GitHub release to download from.
Future<void> installAppUpdate(AppUpdateStatus status) async {
  switch (status.source) {
    case UpdateSource.playStore:
      final update = await InAppUpdate.checkForUpdate();
      if (update.immediateUpdateAllowed) {
        await InAppUpdate.performImmediateUpdate();
      } else if (update.flexibleUpdateAllowed) {
        await InAppUpdate.startFlexibleUpdate();
        await InAppUpdate.completeFlexibleUpdate();
      }
    case UpdateSource.github:
      final url = status.url;
      if (url != null) {
        await launchUrl(url, mode: LaunchMode.externalApplication);
      }
  }
}
