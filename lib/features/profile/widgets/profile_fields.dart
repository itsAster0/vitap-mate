import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/features/profile/logic/profile_text.dart';
import 'package:vitapmate/features/profile/widgets/profile_parts.dart';

/// A tappable row that shows or hides [child].
class Expandable extends HookWidget {
  const Expandable({
    super.key,
    required this.label,
    required this.child,
    this.count,
    this.header = false,
  });

  final String label;
  final Widget child;
  final int? count;

  /// A section title, rather than a small "More" link inside a card.
  final bool header;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final open = useState(false);
    return Column(
      crossAxisAlignment: .stretch,
      children: [
        FTappable(
          semanticsLabel: label,
          onPress: () {
            HapticFeedback.selectionClick();
            open.value = !open.value;
          },
          child: Padding(
            padding: .symmetric(vertical: header ? Space.md : 6),
            child: Row(
              children: [
                Expanded(
                  child: Text(
                    label,
                    style: header
                        ? typography.body.md.copyWith(
                            fontWeight: .w600,
                            color: colors.foreground,
                          )
                        : typography.body.xs.copyWith(
                            fontWeight: .w600,
                            color: colors.app.accentTone.base,
                          ),
                  ),
                ),
                if (count != null && !open.value) Small('$count'),
                const SizedBox(width: Space.sm),
                AnimatedRotation(
                  turns: open.value ? 0.5 : 0,
                  duration: Motion.fast,
                  child: Icon(
                    FLucideIcons.chevronDown,
                    size: 16,
                    color: colors.mutedForeground,
                  ),
                ),
              ],
            ),
          ),
        ),
        AnimatedSize(
          duration: Motion.medium,
          curve: Curves.easeOutCubic,
          alignment: .topCenter,
          child: open.value ? child : const SizedBox(width: double.infinity),
        ),
      ],
    );
  }
}

/// Label/value rows by group; empty values are left out.
class Fields extends StatelessWidget {
  const Fields({super.key, required this.groups});

  final List<ProfileGroup> groups;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final visible = [
      for (final group in groups)
        if (group.fields.any((f) => hasValue(f.value))) group,
    ];
    return Column(
      crossAxisAlignment: .stretch,
      children: [
        for (final (i, group) in visible.indexed) ...[
          if (group.title.isNotEmpty)
            Padding(
              padding: .only(top: i == 0 ? 0 : Space.md, bottom: 2),
              child: Text(
                group.title.toUpperCase(),
                style: typography.body.xs.copyWith(
                  fontWeight: .w700,
                  letterSpacing: 0.6,
                  color: colors.app.accentTone.base,
                ),
              ),
            ),
          for (final field in group.fields)
            if (hasValue(field.value)) _FieldRow(field: field),
        ],
      ],
    );
  }
}

/// A label and its value side by side, or the value under the label when it
/// is too long to share the line.
class _FieldRow extends HookWidget {
  const _FieldRow({required this.field});

  final ProfileField field;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final secret = isSecret(field.label);
    final revealed = useState(!secret);
    final value = revealed.value ? niceCase(field.value) : masked(field.value);
    final labelText = Text(
      niceLabel(field.label),
      style: typography.body.sm.copyWith(color: colors.mutedForeground),
    );
    final valueText = Text.rich(
      TextSpan(
        text: value,
        children: [
          if (secret)
            WidgetSpan(
              alignment: .middle,
              child: Padding(
                padding: const .only(left: 6),
                child: Icon(
                  revealed.value ? FLucideIcons.eyeOff : FLucideIcons.eye,
                  size: 14,
                  color: colors.mutedForeground,
                ),
              ),
            ),
        ],
      ),
      style: typography.body.sm.copyWith(
        color: colors.foreground,
        fontWeight: .w500,
      ),
    );
    final stacked = value.length > 22;

    return Copyable(
      value: field.value,
      onTap: secret ? () => revealed.value = !revealed.value : null,
      child: Padding(
        padding: const .symmetric(vertical: 7),
        child: stacked
            ? Column(
                crossAxisAlignment: .start,
                children: [labelText, const SizedBox(height: 2), valueText],
              )
            : Row(
                crossAxisAlignment: .start,
                children: [
                  Expanded(child: labelText),
                  const SizedBox(width: Space.md),
                  valueText,
                ],
              ),
      ),
    );
  }
}

/// Mobile and email rows after [gap], then the other fields behind "More".
List<Widget> contactsAndMore(
  double gap,
  String? mobile,
  String? email,
  List<ProfileField> rest,
) => [
  if (mobile != null || email != null) ...[
    SizedBox(height: gap),
    if (mobile != null) ContactRow('Mobile', mobile),
    if (email != null) ContactRow('Email', email),
  ],
  if (rest.any((f) => hasValue(f.value)))
    Expandable(
      label: 'More',
      child: Fields(
        groups: [ProfileGroup(title: '', fields: rest)],
      ),
    ),
];
