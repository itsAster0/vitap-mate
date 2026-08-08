import 'dart:developer' show log;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/core/utils/fcm_cookie_bridge_service.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';

final _extensionReleaseUri = Uri.parse(
  'https://github.com/itsKryxen/vitap-mate/releases/tag/extension',
);

class ChromeExtensionPage extends HookConsumerWidget {
  const ChromeExtensionPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final copying = useState(false);
    final resetting = useState(false);
    final openingGitHub = useState(false);

    Future<void> copyToken() async {
      if (copying.value) return;
      copying.value = true;
      try {
        final token = await getFcmTokenForCopy();
        if (token == null || token.trim().isEmpty) {
          if (context.mounted) {
            dispToast(
              context,
              'No Token',
              'Could not get your Token right now.',
            );
          }
          return;
        }
        await Clipboard.setData(ClipboardData(text: token));
        if (context.mounted) {
          dispToast(
            context,
            'Token copied',
            'Paste it into the Chrome extension.',
          );
        }
      } catch (error, stackTrace) {
        log(
          'Failed to copy extension token',
          error: error,
          stackTrace: stackTrace,
        );
        if (context.mounted) {
          dispToast(
            context,
            'Copy failed',
            'Could not copy your Token right now.',
          );
        }
      } finally {
        copying.value = false;
      }
    }

    Future<void> resetToken() async {
      if (resetting.value) return;
      final confirmed = await showAdaptiveDialog<bool>(
        context: context,
        builder: (dialogContext) => AppDialog(
          title: const Text('Reset Token?'),
          body: const Text(
            'Your current Token will stop working. You must paste the new Token into the Chrome extension.',
          ),
          actions: [
            FButton(
              variant: FButtonVariant.outline,
              onPress: () => Navigator.of(dialogContext).pop(false),
              child: const Text('Cancel'),
            ),
            FButton(
              onPress: () => Navigator.of(dialogContext).pop(true),
              child: const Text('Reset Token'),
            ),
          ],
        ),
      );
      if (confirmed != true) return;

      resetting.value = true;
      try {
        final token = await resetFcmTokenForCopy();
        if (token == null || token.trim().isEmpty) {
          if (context.mounted) {
            dispToast(context, 'Reset failed', 'Could not create a new Token.');
          }
          return;
        }
        await Clipboard.setData(ClipboardData(text: token));
        if (context.mounted) {
          dispToast(
            context,
            'New Token copied',
            'Replace the old Token in the Chrome extension.',
          );
        }
      } catch (error, stackTrace) {
        log(
          'Failed to reset extension token',
          error: error,
          stackTrace: stackTrace,
        );
        if (context.mounted) {
          dispToast(context, 'Reset failed', 'Could not create a new Token.');
        }
      } finally {
        resetting.value = false;
      }
    }

    Future<void> openGitHubRelease() async {
      if (openingGitHub.value) return;
      openingGitHub.value = true;
      try {
        final opened = await launchUrl(
          _extensionReleaseUri,
          mode: LaunchMode.externalApplication,
        );
        if (!opened && context.mounted) {
          dispToast(context, 'Could not open GitHub', 'Try again in a moment.');
        }
      } catch (error, stackTrace) {
        log(
          'Failed to open extension release',
          error: error,
          stackTrace: stackTrace,
        );
        if (context.mounted) {
          dispToast(context, 'Could not open GitHub', 'Try again in a moment.');
        }
      } finally {
        openingGitHub.value = false;
      }
    }

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(4, 4, 4, 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          FTileGroup(
            label: const Text('Get ready'),
            children: [
              FTile(
                prefix: const Icon(FLucideIcons.copy),
                title: const Text('Copy Token'),
                subtitle: const Text('You will paste this into the extension'),
                suffix: copying.value
                    ? const FCircularProgress.pinwheel()
                    : const Icon(FLucideIcons.chevronRight),
                onPress: copying.value ? null : copyToken,
              ),
              FTile(
                prefix: const Icon(FLucideIcons.refreshCw),
                title: const Text('Reset Token'),
                subtitle: const Text('Create and copy a new Token'),
                suffix: resetting.value
                    ? const FCircularProgress.pinwheel()
                    : const Icon(FLucideIcons.chevronRight),
                onPress: resetting.value ? null : resetToken,
              ),
              FTile(
                prefix: const Icon(FLucideIcons.externalLink),
                title: const Text('Open GitHub release'),
                subtitle: const Text('Download extension.zip on your computer'),
                suffix: openingGitHub.value
                    ? const FCircularProgress.pinwheel()
                    : const Icon(FLucideIcons.externalLink),
                onPress: openingGitHub.value ? null : openGitHubRelease,
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
            child: Column(
              children: [
                _InstructionRow(
                  number: '1',
                  title: 'Download extension.zip',
                  description:
                      'On your computer, open the GitHub release and download extension.zip.',
                ),
                const Divider(height: 1),
                _InstructionRow(
                  number: '2',
                  title: 'Extract the ZIP',
                  description:
                      'Extract extension.zip. The extracted files include a dist folder.',
                ),
                const Divider(height: 1),
                _InstructionRow(
                  number: '3',
                  title: 'Open Chrome extensions',
                  description:
                      'Enter chrome://extensions in Chrome’s address bar.',
                ),
                const Divider(height: 1),
                _InstructionRow(
                  number: '4',
                  title: 'Enable Developer mode',
                  description:
                      'Turn on Developer mode in the top-right corner.',
                ),
                const Divider(height: 1),
                _InstructionRow(
                  number: '5',
                  title: 'Load the extension',
                  description:
                      'Choose Load unpacked and select the dist folder inside the extracted extension.',
                ),
                const Divider(height: 1),
                _InstructionRow(
                  number: '6',
                  title: 'Paste your Token',
                  description:
                      'Open VITAP Mate from Chrome’s extensions menu, paste your Token, and sign in.',
                ),
              ],
            ),
          ),
          const SizedBox(height: 12),
          Text(
            'Keep your Token private. Anyone with it may be able to request your VTOP session.',
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
