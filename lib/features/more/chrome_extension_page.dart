import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/vtop/http/app_http_client.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/app/app_urls.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';

const _steps = [
  (
    'Download extension.zip',
    'On your computer, open the download link and get extension.zip.',
  ),
  (
    'Extract the ZIP',
    'Extract extension.zip. The extracted files include a dist folder.',
  ),
  (
    'Open Chrome extensions',
    'Enter chrome://extensions in Chrome’s address bar and turn on Developer mode.',
  ),
  ('Load the extension', 'Choose Load unpacked and select the dist folder.'),
  (
    'Paste an access key',
    'Create a key in Connected apps. Click the VITAP Mate icon in Chrome’s toolbar (pin it from the puzzle menu) and paste the key.',
  ),
  (
    'Sign in to VTOP',
    'Press Sign in in the extension, or the VITAP Mate button next to Student on the VTOP login page. It shows each step as your phone or the server signs in.',
  ),
];

/// How to install the browser extension and sign it in with an access key.
class ChromeExtensionPage extends HookConsumerWidget {
  const ChromeExtensionPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final opening = useState(false);

    Future<void> openDownload() async {
      if (opening.value) return;
      opening.value = true;
      var opened = false;
      try {
        opened = await loggedNetCall(
          'open the extension download in the browser',
          () => launchUrl(
            Uri.parse(extensionDownloadUrl),
            mode: .externalApplication,
          ),
        );
      } catch (error, stackTrace) {
        Log.error(
          'features.more',
          'Failed to open the extension download: $error\n$stackTrace',
        );
      } finally {
        opening.value = false;
      }
      if (!opened && context.mounted) {
        dispToast(context, 'Could not open the link', 'Try again in a moment.');
      }
    }

    return SingleChildScrollView(
      padding: const .fromLTRB(4, 4, 4, 24),
      child: Column(
        crossAxisAlignment: .stretch,
        children: [
          FTileGroup(
            label: const Text('Download'),
            children: [
              FTile(
                prefix: const Icon(FLucideIcons.download),
                title: const Text('Download the extension'),
                subtitle: Text(
                  Uri.parse(extensionDownloadUrl).host,
                  maxLines: 1,
                  overflow: .ellipsis,
                ),
                suffix: opening.value
                    ? const FCircularProgress.pinwheel()
                    : const Icon(FLucideIcons.externalLink),
                onPress: opening.value ? null : openDownload,
              ),
            ],
          ),
          const SizedBox(height: 16),
          Text(
            'Install on your computer',
            style: context.theme.typography.body.lg.copyWith(fontWeight: .w600),
          ),
          const SizedBox(height: 10),
          Container(
            decoration: BoxDecoration(
              border: Border.all(color: context.theme.colors.border),
              borderRadius: .circular(16),
            ),
            clipBehavior: .antiAlias,
            child: Column(
              children: [
                for (final (i, (title, description)) in _steps.indexed) ...[
                  if (i > 0) const Divider(height: 1),
                  Padding(
                    padding: const .symmetric(horizontal: 16, vertical: 16),
                    child: Row(
                      crossAxisAlignment: .start,
                      children: [
                        CircleAvatar(
                          radius: 15,
                          backgroundColor: context.theme.colors.secondary,
                          foregroundColor:
                              context.theme.colors.secondaryForeground,
                          child: Text(
                            '${i + 1}',
                            style: context.theme.typography.body.sm,
                          ),
                        ),
                        const SizedBox(width: 14),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: .start,
                            children: [
                              Text(
                                title,
                                style: context.theme.typography.body.lg,
                              ),
                              const SizedBox(height: 3),
                              Text(
                                description,
                                style: context.theme.typography.body.sm
                                    .copyWith(
                                      color:
                                          context.theme.colors.mutedForeground,
                                    ),
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ],
            ),
          ),
          const SizedBox(height: 12),
          Text(
            'To update, download again, replace the dist folder, and press reload on the extension’s card in chrome://extensions. Keep your access key private; you can revoke it any time in Connected apps.',
            style: context.theme.typography.body.sm.copyWith(
              color: context.theme.colors.mutedForeground,
            ),
            textAlign: .center,
          ),
        ],
      ),
    );
  }
}
