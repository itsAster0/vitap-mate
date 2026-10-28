import 'package:flutter/widgets.dart';
import 'package:flutter/services.dart';
import 'package:forui/forui.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/ui/app_palette.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/ui/text_nums.dart';

/// Opens [url] outside the app; toasts when nothing can open it.
Future<void> openExternalLink(BuildContext context, String url) async {
  final opened = await launchUrl(Uri.parse(url), mode: .externalApplication);
  if (!opened && context.mounted) {
    dispToast(context, 'Could not open link', 'Try again in your browser.');
  }
}

/// A tinted note: a tip, a warning or a privacy line.
class GuideCallout extends StatelessWidget {
  const GuideCallout({
    required this.icon,
    required this.tone,
    required this.child,
    super.key,
  });

  final IconData icon;
  final Tone tone;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final typography = context.theme.typography;
    return Container(
      padding: const .all(Space.md),
      decoration: BoxDecoration(
        color: tone.subtle,
        borderRadius: .circular(Radii.sm + 2),
      ),
      child: Row(
        crossAxisAlignment: .start,
        children: [
          Padding(
            padding: const .only(top: 1),
            child: Icon(icon, size: 16, color: tone.onSubtle),
          ),
          const SizedBox(width: Space.sm + 2),
          Expanded(
            child: DefaultTextStyle.merge(
              style: typography.body.xs.copyWith(
                color: tone.onSubtle,
                height: 1.4,
              ),
              child: child,
            ),
          ),
        ],
      ),
    );
  }
}

/// One pro or con line on a method card.
class MethodPoint {
  const MethodPoint(this.text, {this.caveat = false});

  final String text;
  final bool caveat;
}

/// A way of reading the OTP, as offered on the Email OTP screen.
class OtpMethodCard extends StatelessWidget {
  const OtpMethodCard({
    required this.icon,
    required this.iconTone,
    required this.eyebrow,
    required this.title,
    required this.description,
    required this.facts,
    required this.points,
    required this.action,
    this.highlighted = false,
    this.status,
    super.key,
  });

  final IconData icon;
  final Tone iconTone;
  final String eyebrow;
  final String title;
  final String description;
  final List<(String, String)> facts;
  final List<MethodPoint> points;
  final Widget action;
  final bool highlighted;

  /// Shown under the title once the method is set up.
  final Widget? status;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final app = colors.app;
    return Container(
      padding: const .all(Space.lg),
      decoration: BoxDecoration(
        color: colors.card,
        borderRadius: .circular(Radii.md),
        border: Border.all(
          color: highlighted ? app.accent : colors.border,
          width: highlighted ? 1.5 : 1,
        ),
      ),
      child: Column(
        crossAxisAlignment: .stretch,
        children: [
          Row(
            children: [
              Container(
                width: 36,
                height: 36,
                decoration: BoxDecoration(
                  color: iconTone.subtle,
                  borderRadius: .circular(9),
                ),
                child: Icon(icon, size: 20, color: iconTone.onSubtle),
              ),
              const SizedBox(width: 10),
              Expanded(
                child: Column(
                  crossAxisAlignment: .start,
                  children: [
                    Text(
                      eyebrow,
                      style: typography.body.xs.copyWith(
                        fontWeight: .w600,
                        letterSpacing: 0.3,
                        color: highlighted
                            ? app.accent
                            : colors.mutedForeground,
                      ),
                    ),
                    Text(
                      title,
                      style: typography.body.md.copyWith(fontWeight: .w600),
                    ),
                  ],
                ),
              ),
            ],
          ),
          if (status case final status?) ...[
            const SizedBox(height: Space.md),
            status,
          ],
          const SizedBox(height: Space.md),
          Text(
            description,
            style: typography.body.sm.copyWith(
              color: colors.mutedForeground,
              height: 1.45,
            ),
          ),
          const SizedBox(height: Space.md),
          Row(
            children: [
              for (final (index, (label, value)) in facts.indexed) ...[
                if (index > 0) const SizedBox(width: Space.sm),
                Expanded(
                  // A short labelled fact ("One-time setup · About 2 min").
                  child: Container(
                    padding: const .symmetric(
                      horizontal: 10,
                      vertical: Space.sm,
                    ),
                    decoration: BoxDecoration(
                      color: colors.muted,
                      borderRadius: .circular(Radii.sm),
                      border: Border.all(color: colors.border),
                    ),
                    child: Column(
                      crossAxisAlignment: .start,
                      children: [
                        Text(
                          label,
                          style: typography.body.xs.copyWith(
                            color: colors.mutedForeground,
                            fontSize: 11,
                          ),
                        ),
                        const SizedBox(height: 2),
                        Text(
                          value,
                          style: typography.body.sm.copyWith(fontWeight: .w600),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ],
          ),
          const SizedBox(height: Space.md),
          for (final point in points)
            Padding(
              padding: const .only(bottom: 6),
              child: Row(
                crossAxisAlignment: .start,
                children: [
                  Padding(
                    padding: const .only(top: 2),
                    child: Icon(
                      point.caveat
                          ? FLucideIcons.circleAlert
                          : FLucideIcons.check,
                      size: 15,
                      color: point.caveat ? app.warning.base : app.success.base,
                    ),
                  ),
                  const SizedBox(width: Space.sm),
                  Expanded(
                    child: Text(
                      point.text,
                      style: typography.body.xs.copyWith(height: 1.4),
                    ),
                  ),
                ],
              ),
            ),
          const SizedBox(height: Space.sm),
          action,
        ],
      ),
    );
  }
}

/// Steps done out of [total], as a segmented bar.
class GuideProgress extends StatelessWidget {
  const GuideProgress({
    required this.done,
    required this.total,
    required this.current,
    super.key,
  });

  final Set<int> done;
  final int total;

  /// The open step, or null when all are done.
  final int? current;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final app = colors.app;
    final typography = context.theme.typography;
    final label = current == null
        ? 'All $total steps done'
        : 'Step ${current! + 1} of $total';
    return Column(
      crossAxisAlignment: .stretch,
      children: [
        Row(
          children: [
            Text(
              label,
              style: typography.body.xs.copyWith(color: colors.mutedForeground),
            ),
            const Spacer(),
            Text(
              '${done.length} done',
              style: typography.body.xs.copyWith(color: colors.mutedForeground),
            ),
          ],
        ),
        const SizedBox(height: Space.sm),
        Row(
          children: [
            for (var index = 0; index < total; index++) ...[
              if (index > 0) const SizedBox(width: Space.xs),
              Expanded(
                child: AnimatedContainer(
                  duration: Motion.medium,
                  height: 4,
                  decoration: BoxDecoration(
                    borderRadius: .circular(2),
                    color: done.contains(index)
                        ? app.accent
                        : index == current
                        ? app.accent.withValues(alpha: 0.4)
                        : colors.border,
                  ),
                ),
              ),
            ],
          ],
        ),
      ],
    );
  }
}

/// One step of a setup guide. Collapsed it is one line with its state;
/// open it shows what to do, a warning, how to tell it worked and actions.
class GuideStepTile extends StatelessWidget {
  const GuideStepTile({
    required this.number,
    required this.title,
    required this.done,
    required this.open,
    required this.onTap,
    required this.onToggleDone,
    this.actions = const [],
    this.warning,
    this.doneWhen,
    this.link,
    this.badge,
    this.extra,
    super.key,
  });

  final int number;
  final String title;
  final bool done;
  final bool open;
  final VoidCallback onTap;
  final VoidCallback onToggleDone;
  final List<String> actions;
  final String? warning;
  final String? doneWhen;

  /// The page the step happens on, opened by its "Open" button.
  final String? link;
  final String? badge;

  /// Live content under the actions (a copyable value, a status).
  final Widget? extra;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final app = colors.app;
    final typography = context.theme.typography;

    final marker = Container(
      width: 26,
      height: 26,
      alignment: .center,
      decoration: BoxDecoration(
        shape: .circle,
        color: done
            ? app.success.base
            : open
            ? app.accent
            : null,
        border: done || open
            ? null
            : Border.all(color: colors.border, width: 1.5),
      ),
      child: done
          ? Icon(FLucideIcons.check, size: 14, color: colors.background)
          : Text(
              '$number',
              style: typography.body.xs.copyWith(
                fontWeight: .w600,
                color: open ? app.onAccent : colors.mutedForeground,
              ),
            ),
    );

    final header = Row(
      children: [
        marker,
        const SizedBox(width: Space.md),
        Expanded(
          child: Text(
            title,
            style: typography.body.sm.copyWith(
              fontWeight: open ? .w600 : .w500,
              color: done && !open ? colors.mutedForeground : null,
            ),
          ),
        ),
        if (badge case final badge? when !done) ...[
          const SizedBox(width: Space.sm),
          FBadge(variant: .secondary, child: Text(badge)),
        ],
        if (!open) ...[
          const SizedBox(width: Space.xs),
          Icon(
            FLucideIcons.chevronDown,
            size: 16,
            color: colors.mutedForeground,
          ),
        ],
      ],
    );

    final body = Padding(
      padding: const .only(left: 38, top: Space.md),
      child: Column(
        crossAxisAlignment: .stretch,
        children: [
          for (final (index, action) in actions.indexed)
            Padding(
              padding: const .only(bottom: 6),
              child: Row(
                crossAxisAlignment: .start,
                children: [
                  SizedBox(
                    width: 18,
                    child: Text(
                      '${index + 1}.',
                      style: typography.body.xs.copyWith(
                        color: colors.mutedForeground,
                        height: 1.45,
                      ),
                    ),
                  ),
                  Expanded(
                    child: Text(
                      action,
                      style: typography.body.xs.copyWith(height: 1.45),
                    ),
                  ),
                ],
              ),
            ),
          if (extra case final extra?) ...[
            const SizedBox(height: Space.xs),
            extra,
            const SizedBox(height: Space.xs),
          ],
          if (warning case final warning?) ...[
            const SizedBox(height: Space.xs),
            GuideCallout(
              icon: FLucideIcons.triangleAlert,
              tone: app.warning,
              child: Text(warning),
            ),
          ],
          if (doneWhen case final doneWhen?) ...[
            const SizedBox(height: Space.sm),
            Text.rich(
              TextSpan(
                children: [
                  TextSpan(
                    text: 'Done when ',
                    style: TextStyle(
                      fontWeight: .w600,
                      color: app.success.onSubtle,
                    ),
                  ),
                  TextSpan(text: doneWhen),
                ],
              ),
              style: typography.body.xs.copyWith(
                color: colors.mutedForeground,
                height: 1.4,
              ),
            ),
          ],
          const SizedBox(height: Space.md),
          Row(
            children: [
              if (link case final link?) ...[
                Expanded(
                  child: FButton(
                    variant: .outline,
                    size: .sm,
                    onPress: () => openExternalLink(context, link),
                    suffix: const Icon(FLucideIcons.externalLink),
                    child: const Text('Open'),
                  ),
                ),
                const SizedBox(width: Space.sm),
              ],
              Expanded(
                child: FButton(
                  variant: done ? .outline : .primary,
                  size: .sm,
                  onPress: onToggleDone,
                  child: Text(done ? 'Mark not done' : 'Mark done'),
                ),
              ),
            ],
          ),
        ],
      ),
    );

    return AnimatedContainer(
      duration: Motion.medium,
      curve: Curves.easeOutCubic,
      decoration: BoxDecoration(
        color: colors.card,
        borderRadius: .circular(open ? Radii.md : Radii.sm + 2),
        border: Border.all(
          color: open ? app.accent : colors.border,
          width: open ? 1.5 : 1,
        ),
      ),
      child: Padding(
        padding: .all(open ? 14 : Space.md),
        child: AnimatedSize(
          duration: Motion.medium,
          curve: Curves.easeOutCubic,
          alignment: .topCenter,
          child: Column(
            crossAxisAlignment: .stretch,
            children: [
              FTappable(onPress: open ? null : onTap, child: header),
              if (open) body,
            ],
          ),
        ),
      ),
    );
  }
}

/// A value to paste somewhere else, with a copy button.
class CopyValue extends StatelessWidget {
  const CopyValue({
    required this.label,
    required this.value,
    this.large = false,
    super.key,
  });

  final String label;
  final String value;
  final bool large;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Container(
      padding: const .fromLTRB(Space.md, Space.sm, Space.xs, Space.sm),
      decoration: BoxDecoration(
        color: colors.muted,
        borderRadius: .circular(Radii.sm),
      ),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: .start,
              children: [
                Text(
                  label,
                  style: typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                    fontSize: 11,
                  ),
                ),
                const SizedBox(height: 2),
                Text(
                  value,
                  style: (large ? typography.body.md : typography.body.sm).nums(
                    fontWeight: .w600,
                    letterSpacing: large ? 1 : null,
                  ),
                ),
              ],
            ),
          ),
          FButton.icon(
            variant: .ghost,
            semanticsLabel: 'Copy $label',
            onPress: () async {
              await Clipboard.setData(ClipboardData(text: value));
              if (context.mounted) dispToast(context, 'Copied', label);
            },
            child: const Icon(FLucideIcons.copy),
          ),
        ],
      ),
    );
  }
}

/// A collapsible list of problems and their fixes.
class GuideTroubleshooting extends StatelessWidget {
  const GuideTroubleshooting({required this.items, super.key});

  final List<(String, String)> items;

  @override
  Widget build(BuildContext context) {
    final typography = context.theme.typography;
    return Column(
      crossAxisAlignment: .stretch,
      children: [
        Text(
          'Something went wrong?',
          style: typography.body.sm.copyWith(fontWeight: .w600),
        ),
        const SizedBox(height: Space.xs),
        FAccordion(
          children: [
            for (final (problem, fix) in items)
              FAccordionItem(
                title: Text(problem, style: typography.body.sm),
                child: Text(
                  fix,
                  style: typography.body.xs.copyWith(
                    color: context.theme.colors.mutedForeground,
                    height: 1.45,
                  ),
                ),
              ),
          ],
        ),
      ],
    );
  }
}
