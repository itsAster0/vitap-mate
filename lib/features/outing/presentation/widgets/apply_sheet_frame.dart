import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';

/// Collapses runs of whitespace, as VTOP's own form effectively does.
String tidyText(String value) => value.trim().split(RegExp(r'\s+')).join(' ');

/// Shows what is about to be sent and asks to send it.
Future<bool> confirmApplication(
  BuildContext context, {
  required String title,
  required List<(String, String)> rows,
  required String footnote,
}) async {
  final confirmed = await showFDialog<bool>(
    context: context,
    // Above the tab shell, so the keyboard inset is counted once.
    useRootNavigator: true,
    builder: (dialogContext, _, animation) {
      final colors = dialogContext.theme.colors;
      final typography = dialogContext.theme.typography;
      return AppDialog(
        animation: animation,
        title: Text(title),
        body: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            for (final (label, value) in rows)
              Padding(
                padding: const EdgeInsets.only(bottom: Space.xs + 2),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SizedBox(
                      width: 72,
                      child: Text(
                        label,
                        style: typography.body.sm.copyWith(
                          color: colors.mutedForeground,
                        ),
                      ),
                    ),
                    Expanded(
                      child: Text(
                        value,
                        style: typography.body.sm.copyWith(
                          fontWeight: FontWeight.w600,
                          color: colors.foreground,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            const SizedBox(height: Space.sm),
            Text(
              footnote,
              style: typography.body.xs.copyWith(color: colors.mutedForeground),
            ),
          ],
        ),
        actions: [
          FButton(
            onPress: () => Navigator.of(dialogContext).pop(true),
            child: const Text('Apply'),
          ),
          FButton(
            variant: FButtonVariant.outline,
            onPress: () => Navigator.of(dialogContext).pop(false),
            child: const Text('Edit'),
          ),
        ],
      );
    },
  );
  return confirmed ?? false;
}

/// The shell of an apply sheet: handle, title, scrolling fields, VTOP's
/// refusal when there is one, and the submit button above the keyboard.
class ApplySheetFrame extends StatelessWidget {
  const ApplySheetFrame({
    super.key,
    required this.title,
    required this.children,
    required this.onSubmit,
    required this.sending,
    this.subtitle,
    this.failure,
  });

  final String title;
  final String? subtitle;
  final List<Widget> children;
  final VoidCallback onSubmit;
  final bool sending;
  final String? failure;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final media = MediaQuery.of(context);
    // The sheet keeps its height when the keyboard opens and slides under
    // it. Capping it to the space above the keyboard makes the fields
    // scroll instead, with Review and the focused field in view.
    final maxHeight =
        media.size.height -
        media.viewInsets.bottom -
        media.padding.top -
        Space.lg;
    return Container(
      constraints: BoxConstraints(maxHeight: maxHeight),
      decoration: BoxDecoration(
        color: colors.background,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(10)),
      ),
      child: SafeArea(
        top: false,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(
                Space.lg,
                Space.md,
                Space.lg,
                0,
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const SheetHandle(),
                  const SizedBox(height: Space.md),
                  Text(
                    title,
                    style: typography.body.lg.copyWith(
                      fontWeight: FontWeight.w700,
                      color: colors.foreground,
                    ),
                  ),
                  if (subtitle != null)
                    Text(
                      subtitle!,
                      style: typography.body.xs.copyWith(
                        color: colors.mutedForeground,
                      ),
                    ),
                ],
              ),
            ),
            Flexible(
              child: SingleChildScrollView(
                padding: const EdgeInsets.fromLTRB(
                  Space.lg,
                  Space.lg,
                  Space.lg,
                  Space.sm,
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: children,
                ),
              ),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(
                Space.lg,
                Space.xs,
                Space.lg,
                Space.md,
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  if (failure != null) ...[
                    Container(
                      padding: const EdgeInsets.all(Space.sm + 2),
                      decoration: BoxDecoration(
                        color: colors.app.danger.subtle,
                        borderRadius: BorderRadius.circular(Radii.md),
                      ),
                      child: Text(
                        failure!,
                        style: typography.body.xs.copyWith(
                          color: colors.app.danger.onSubtle,
                        ),
                      ),
                    ),
                    const SizedBox(height: Space.sm),
                  ],
                  FButton(
                    onPress: sending ? null : onSubmit,
                    child: sending
                        ? const SizedBox.square(
                            dimension: 16,
                            child: FCircularProgress(),
                          )
                        : const Text('Review request'),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Scrolls [child] into view when a field inside it takes focus, once the
/// keyboard has opened, so the field being typed in is never left behind
/// the keyboard. Wrap the label, field and its error together.
class KeepVisibleOnFocus extends StatelessWidget {
  const KeepVisibleOnFocus({super.key, required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => Focus(
    canRequestFocus: false,
    skipTraversal: true,
    onFocusChange: (focused) {
      if (!focused) return;
      // Wait for the keyboard to finish opening and the sheet to shrink.
      Future<void>.delayed(const Duration(milliseconds: 350), () {
        if (!context.mounted) return;
        Scrollable.ensureVisible(
          context,
          alignment: 0.5,
          duration: Motion.medium,
          curve: Curves.easeOutCubic,
        );
      });
    },
    child: child,
  );
}
