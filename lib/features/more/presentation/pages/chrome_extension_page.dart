import 'dart:developer' show log;

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/core/utils/app_urls.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';

/// How to install the browser extension and sign it in with an access key.
class ChromeExtensionPage extends HookConsumerWidget {
  const ChromeExtensionPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final opening = useState(false);

    Future<void> openDownload() async {
      if (opening.value) return;
      opening.value = true;
      try {
        final opened = await launchUrl(
          Uri.parse(extensionDownloadUrl),
          mode: LaunchMode.externalApplication,
        );
        if (!opened && context.mounted) {
          dispToast(
            context,
            'Could not open the link',
            'Try again in a moment.',
          );
        }
      } catch (error, stackTrace) {
        log(
          'Failed to open the extension download',
          error: error,
          stackTrace: stackTrace,
        );
        if (context.mounted) {
          dispToast(
            context,
            'Could not open the link',
            'Try again in a moment.',
          );
        }
      } finally {
        opening.value = false;
      }
    }

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(4, 4, 4, 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
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
                  overflow: TextOverflow.ellipsis,
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
            style: context.theme.typography.body.lg.copyWith(
              fontWeight: FontWeight.w600,
            ),
          ),
          const SizedBox(height: 10),
          Container(
            decoration: BoxDecoration(
              border: Border.all(color: context.theme.colors.border),
              borderRadius: BorderRadius.circular(16),
            ),
            clipBehavior: Clip.antiAlias,
            child: const Column(
              children: [
                _InstructionRow(
                  number: '1',
                  title: 'Download extension.zip',
                  description:
                      'On your computer, open the download link and get extension.zip.',
                ),
                Divider(height: 1),
                _InstructionRow(
                  number: '2',
                  title: 'Extract the ZIP',
                  description:
                      'Extract extension.zip. The extracted files include a dist folder.',
                ),
                Divider(height: 1),
                _InstructionRow(
                  number: '3',
                  title: 'Open Chrome extensions',
                  description:
                      'Enter chrome://extensions in Chrome’s address bar and turn on Developer mode.',
                ),
                Divider(height: 1),
                _InstructionRow(
                  number: '4',
                  title: 'Load the extension',
                  description:
                      'Choose Load unpacked and select the dist folder.',
                ),
                Divider(height: 1),
                _InstructionRow(
                  number: '5',
                  title: 'Paste an access key',
                  description:
                      'Go back to Connected apps, create a key, and paste it into the extension.',
                ),
              ],
            ),
          ),
          const SizedBox(height: 12),
          Text(
            'Keep your access key private. You can revoke it any time in Connected apps.',
            style: context.theme.typography.body.sm.copyWith(
              color: context.theme.colors.mutedForeground,
            ),
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }
}

class _InstructionRow extends StatelessWidget {
  const _InstructionRow({
    required this.number,
    required this.title,
    required this.description,
  });

  final String number;
  final String title;
  final String description;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 16),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          CircleAvatar(
            radius: 15,
            backgroundColor: context.theme.colors.secondary,
            foregroundColor: context.theme.colors.secondaryForeground,
            child: Text(number, style: context.theme.typography.body.sm),
          ),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: context.theme.typography.body.lg),
                const SizedBox(height: 3),
                Text(
                  description,
                  style: context.theme.typography.body.sm.copyWith(
                    color: context.theme.colors.mutedForeground,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
