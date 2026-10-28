import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/server/widgets/icon_box.dart';

/// One choice of an [ExpandablePicker].
class PickerOption<T> {
  const PickerOption({
    required this.value,
    required this.title,
    required this.summary,
    required this.detail,
    required this.icon,
    required this.tone,
  });

  final T value;
  final String title;

  /// Short; shown under the title while the picker is closed.
  final String summary;

  /// Shown after [summary] while it is open.
  final String detail;
  final IconData icon;
  final Tone tone;
}

/// A setting as one row showing the current choice, which opens in place to
/// every option with what it does; choosing one closes it.
///
/// While [pending] is set (a choice being saved), the row shows it with a
/// spinner and the picker cannot be opened, so a second choice cannot race
/// the first. [enabled] false shows the choice without letting it change.
class ExpandablePicker<T> extends HookWidget {
  const ExpandablePicker({
    super.key,
    required this.value,
    required this.options,
    required this.onSelect,
    this.label,
    this.pending,
    this.enabled = true,
  });

  final T value;
  final List<PickerOption<T>> options;
  final void Function(T value) onSelect;

  /// What the setting is, shown above the choice; null for the choice alone.
  final String? label;
  final T? pending;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final open = useState(false);
    final colors = context.theme.colors;
    final saving = pending != null;
    final shown = options.firstWhere(
      (option) => option.value == (pending ?? value),
      orElse: () => options.first,
    );
    final canOpen = enabled && !saving;
    // Closed while it cannot be used; reopens as it was once it can.
    final isOpen = open.value && canOpen;

    final header = FTile(
      prefix: ServerIconBox(icon: shown.icon, tone: shown.tone),
      title: Text(label ?? shown.title, maxLines: 2),
      subtitle: Text(
        saving
            ? 'Saving…'
            : label == null
            ? shown.summary
            : '${shown.title} · ${shown.summary}',
        maxLines: 2,
      ),
      suffix: saving
          ? const FCircularProgress.pinwheel()
          : AnimatedRotation(
              turns: isOpen ? 0.5 : 0,
              duration: const Duration(milliseconds: 220),
              curve: Curves.easeOutCubic,
              child: Icon(
                FLucideIcons.chevronDown,
                color: canOpen ? null : colors.mutedForeground,
              ),
            ),
      onPress: canOpen ? () => open.value = !isOpen : null,
    );

    return AnimatedSize(
      duration: const Duration(milliseconds: 220),
      curve: Curves.easeOutCubic,
      alignment: .topCenter,
      child: FTileGroup(
        divider: .indented,
        children: [
          header,
          if (isOpen)
            for (final option in options)
              FTile(
                prefix: ServerIconBox(icon: option.icon, tone: option.tone),
                title: Text(option.title, maxLines: 2),
                subtitle: Text(
                  '${option.summary}. ${option.detail}',
                  maxLines: 4,
                ),
                suffix: option.value == value
                    ? Icon(FLucideIcons.check, color: option.tone.onSubtle)
                    : null,
                onPress: () {
                  open.value = false;
                  if (option.value != value) onSelect(option.value);
                },
              ),
        ],
      ),
    );
  }
}
