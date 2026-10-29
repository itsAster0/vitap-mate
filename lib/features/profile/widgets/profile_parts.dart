import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/platform/files/general_utils.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/ui/ui.dart';

/// Loading, error and data states shared by both tabs.
class Loaded<T> extends StatelessWidget {
  const Loaded({
    super.key,
    required this.value,
    required this.what,
    required this.onRetry,
    required this.builder,
  });

  final AsyncValue<T> value;
  final String what;
  final Future<void> Function() onRetry;
  final Widget Function(T data) builder;

  @override
  Widget build(BuildContext context) => value.when(
    skipLoadingOnRefresh: true,
    skipLoadingOnReload: true,
    loading: () => const Column(
      children: [
        Skeleton(height: 150, radius: Radii.lg),
        SizedBox(height: Space.lg),
        SkeletonList(count: 4, height: 72),
      ],
    ),
    error: (e, _) => EmptyState(
      icon: FLucideIcons.cloudAlert,
      title: "Couldn't load $what",
      message: commonErrorMessage(e),
      action: FButton(
        variant: .outline,
        mainAxisSize: .min,
        onPress: () async {
          try {
            await onRetry();
          } catch (e) {
            if (context.mounted) disCommonToast(context, e);
          }
        },
        child: const Text('Try again'),
      ),
    ),
    data: builder,
  );
}

/// A phone number or email with what it is; a tap calls, mails or opens
/// WhatsApp, a long-press copies it.
class ContactRow extends StatelessWidget {
  const ContactRow(this.label, this.value, {super.key});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final digits = value.replaceAll(RegExp(r'\D'), '');
    final (icon, uri) = switch (value) {
      _ when value.contains('@') => (
        FLucideIcons.mail,
        Uri(scheme: 'mailto', path: value),
      ),
      _ when label.toLowerCase().contains('whatsapp') => (
        FLucideIcons.messageCircle,
        // wa.me takes the number with its country code.
        Uri.https('wa.me', digits.length == 10 ? '91$digits' : digits),
      ),
      _ => (FLucideIcons.phone, Uri(scheme: 'tel', path: digits)),
    };
    return Copyable(
      value: value,
      onTap: () => launchUrl(uri, mode: .externalApplication),
      child: Padding(
        padding: const .symmetric(vertical: 7),
        child: Row(
          children: [
            Icon(icon, size: 16, color: colors.app.accentTone.base),
            const SizedBox(width: Space.sm + 2),
            Expanded(
              child: Column(
                crossAxisAlignment: .start,
                children: [
                  Text(
                    value,
                    maxLines: 1,
                    overflow: .ellipsis,
                    style: typography.body.sm.copyWith(
                      color: colors.foreground,
                      fontWeight: .w500,
                    ),
                  ),
                  Small(label),
                ],
              ),
            ),
            Icon(
              FLucideIcons.chevronRight,
              size: 16,
              color: colors.mutedForeground,
            ),
          ],
        ),
      ),
    );
  }
}

/// A rounded square in [tone] with [icon], or a circle with [initials]
/// ([large] for the mentor), then [title] in bold and smaller [lines].
class Headline extends StatelessWidget {
  const Headline({
    super.key,
    required this.tone,
    required this.title,
    required this.lines,
    this.icon,
    this.initials,
    this.large = false,
  });

  final Tone tone;
  final String title;
  final List<String> lines;
  final IconData? icon;
  final String? initials;
  final bool large;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final body = context.theme.typography.body;
    return Row(
      children: [
        Container(
          width: large ? 52 : 44,
          height: large ? 52 : 44,
          alignment: .center,
          decoration: BoxDecoration(
            color: tone.subtle,
            shape: icon == null ? .circle : .rectangle,
            borderRadius: icon == null ? null : .circular(Radii.md),
          ),
          child: icon != null
              ? Icon(icon, size: 20, color: tone.onSubtle)
              : Text(
                  initials!,
                  style: (large ? body.md : body.sm).copyWith(
                    fontWeight: .w700,
                    color: tone.onSubtle,
                  ),
                ),
        ),
        const SizedBox(width: Space.md),
        Expanded(
          child: Column(
            crossAxisAlignment: .start,
            children: [
              Text(
                title,
                style: body.md.copyWith(
                  fontWeight: .w700,
                  color: colors.foreground,
                ),
              ),
              for (final line in lines) Small(line),
            ],
          ),
        ),
      ],
    );
  }
}

/// Small grey text: a caption, count or detail.
class Small extends StatelessWidget {
  const Small(this.text, {super.key});

  final String text;

  @override
  Widget build(BuildContext context) => Text(
    text,
    style: context.theme.typography.body.xs.copyWith(
      color: context.theme.colors.mutedForeground,
    ),
  );
}

/// Long-press copies [value]; [onTap], if given, runs on a tap.
class Copyable extends StatelessWidget {
  const Copyable({
    super.key,
    required this.value,
    required this.child,
    this.onTap,
  });

  final String value;
  final Widget child;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) => GestureDetector(
    behavior: .opaque,
    onTap: onTap,
    onLongPress: () async {
      HapticFeedback.mediumImpact();
      await Clipboard.setData(ClipboardData(text: value));
      if (context.mounted) dispToast(context, 'Copied', value);
    },
    child: child,
  );
}
