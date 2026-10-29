import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:intl/intl.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/course_page/logic/course_page_logic.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/ui/text_nums.dart';

/// One lecture: its date, topic and material chips.
class LectureTile extends StatelessWidget {
  const LectureTile({
    super.key,
    required this.lecture,
    required this.planned,
    required this.isNext,
    required this.busy,
    required this.onMaterial,
  });

  final CourseLecture lecture;
  final bool planned;
  final bool isNext;
  final Set<String> busy;
  final ValueChanged<CourseMaterial> onMaterial;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final date = lectureDate(lecture);
    final faded = planned && !isNext;
    return Surface(
      borderColor: isNext ? colors.app.accentTone.base : null,
      padding: const .fromLTRB(
        Space.sm + 2,
        Space.sm + 2,
        Space.md,
        Space.sm + 2,
      ),
      child: Row(
        crossAxisAlignment: .start,
        children: [
          SizedBox(
            width: 44,
            child: Column(
              children: [
                Text(
                  date == null ? '—' : '${date.day}',
                  style: context.theme.typography.body.lg.nums(
                    fontWeight: .w700,
                    height: 1.1,
                    color: faded ? colors.mutedForeground : colors.foreground,
                  ),
                ),
                Text(
                  date == null
                      ? lecture.day
                      : DateFormat('MMM', 'en_US').format(date).toUpperCase(),
                  style: context.theme.typography.body.xs.copyWith(
                    fontWeight: .w600,
                    letterSpacing: 0.4,
                    color: colors.mutedForeground,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(width: Space.sm),
          Expanded(
            child: Column(
              crossAxisAlignment: .start,
              children: [
                Row(
                  children: [
                    Text(
                      '#${lecture.serial} · ${lecture.day}',
                      style: context.theme.typography.body.xs.copyWith(
                        color: colors.mutedForeground,
                      ),
                    ),
                    if (isNext) ...[
                      const SizedBox(width: Space.sm),
                      ToneBadge(
                        label: _isToday(date) ? 'TODAY' : 'NEXT',
                        tone: colors.app.accentTone,
                      ),
                    ] else if (planned) ...[
                      const SizedBox(width: Space.sm),
                      ToneBadge.neutral(context, 'PLANNED'),
                    ],
                  ],
                ),
                const SizedBox(height: 2),
                Text(
                  lecture.topic.isEmpty ? 'No topic given' : lecture.topic,
                  style: context.theme.typography.body.sm.copyWith(
                    fontWeight: .w600,
                    color: faded ? colors.mutedForeground : colors.foreground,
                  ),
                ),
                if (lecture.materials.isNotEmpty) ...[
                  const SizedBox(height: Space.sm),
                  Wrap(
                    spacing: Space.xs + 2,
                    runSpacing: Space.xs + 2,
                    children: [
                      for (final material in lecture.materials)
                        _MaterialChip(
                          label: materialLabel(material.label),
                          busy: busy.contains(material.path),
                          onPress: () => onMaterial(material),
                        ),
                    ],
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _MaterialChip extends StatelessWidget {
  const _MaterialChip({
    required this.label,
    required this.busy,
    required this.onPress,
  });

  final String label;
  final bool busy;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final tone = context.theme.colors.app.accentTone;
    return PressScale(
      scale: 0.95,
      onPress: busy ? () {} : onPress,
      semanticsLabel: label,
      child: Container(
        padding: const .symmetric(horizontal: Space.sm + 2, vertical: 5),
        decoration: BoxDecoration(
          color: tone.subtle,
          borderRadius: .circular(Radii.pill),
        ),
        child: Row(
          mainAxisSize: .min,
          children: [
            busy
                ? const SizedBox.square(
                    dimension: 12,
                    child: FCircularProgress(),
                  )
                : Icon(FLucideIcons.paperclip, size: 12, color: tone.onSubtle),
            const SizedBox(width: 5),
            Text(
              label,
              style: context.theme.typography.body.xs.copyWith(
                fontWeight: .w600,
                color: tone.onSubtle,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

bool _isToday(DateTime? date) {
  if (date == null) return false;
  final now = DateTime.now();
  return date.year == now.year &&
      date.month == now.month &&
      date.day == now.day;
}
