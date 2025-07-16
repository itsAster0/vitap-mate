import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/features/timetable/presentation/widgets/timetable_colors.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

enum ClassStatus { completed, ongoing, upcoming, nextClass }

class TimetableCard extends HookWidget {
  final TimetableSlot slot;
  const TimetableCard({super.key, required this.slot});

  ClassStatus getClassStatus(String startTimeStr, String endTimeStr) {
    final now = DateTime.now();
    final currentTime = Duration(hours: now.hour, minutes: now.minute);

    Duration parseTime(String timeStr) {
      final parts = timeStr.split(':').map(int.parse).toList();
      return Duration(hours: parts[0], minutes: parts[1]);
    }

    final startTime = parseTime(startTimeStr);
    final endTime = parseTime(endTimeStr);
    final nextClassThreshold = currentTime + const Duration(minutes: 50);

    if (currentTime >= startTime && currentTime <= endTime) {
      return ClassStatus.ongoing;
    } else if (currentTime > endTime) {
      return ClassStatus.completed;
    } else if (nextClassThreshold >= startTime &&
        nextClassThreshold <= endTime) {
      return ClassStatus.nextClass;
    } else {
      return ClassStatus.upcoming;
    }
  }

  Color getCardBackgroundColor() {
    if (slot.serial == "-1") return TimetableColors.freeTimeBackground;
    return slot.courseType.endsWith("LA")
        ? TimetableColors.labBackground
        : TimetableColors.lectureBackground;
  }

  (Color border, Color background, Color text, String label) getStatusStyle(
    ClassStatus status,
  ) {
    switch (status) {
      case ClassStatus.ongoing:
        return (
          TimetableColors.ongoingBorder,
          TimetableColors.ongoingBackground,
          TimetableColors.ongoingText,
          'ONGOING',
        );
      case ClassStatus.completed:
        return (
          TimetableColors.completedBorder,
          TimetableColors.completedBackground,
          TimetableColors.completedText,
          'COMPLETED',
        );
      case ClassStatus.nextClass:
        return (
          TimetableColors.nextClassBorder,
          TimetableColors.nextClassBackground,
          TimetableColors.nextClassText,
          'NEXT CLASS',
        );
      case ClassStatus.upcoming:
        return (
          TimetableColors.upcomingBorder,
          TimetableColors.upcomingBackground,
          TimetableColors.upcomingText,
          'UPCOMING',
        );
    }
  }

  @override
  Widget build(BuildContext context) {
    final status =
        slot.serial != "-1"
            ? getClassStatus(slot.startTime, slot.endTime)
            : null;

    final statusStyle = status != null ? getStatusStyle(status) : null;
    final isHighlighted =
        status == ClassStatus.ongoing || status == ClassStatus.nextClass;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8.0, vertical: 4.0),
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 300),
        decoration: BoxDecoration(
          color: getCardBackgroundColor(),
          borderRadius: BorderRadius.circular(12),
          border:
              statusStyle != null && isHighlighted
                  ? Border.all(color: statusStyle.$1, width: 2)
                  : null,
          boxShadow: [
            BoxShadow(
              color:
                  isHighlighted
                      ? TimetableColors.statusShadow
                      : TimetableColors.cardShadow,
              blurRadius: isHighlighted ? 8 : 6,
              offset: const Offset(0, 2),
              spreadRadius: isHighlighted ? 1 : 0,
            ),
          ],
        ),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child:
              slot.serial != "-1"
                  ? _buildClassCard(context, statusStyle)
                  : _buildFreeTimeCard(context),
        ),
      ),
    );
  }

  Widget _buildClassCard(
    BuildContext context,
    (Color, Color, Color, String)? statusStyle,
  ) {
   
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
              decoration: BoxDecoration(
                color:
                    slot.courseType.endsWith("LA")
                        ? const Color(0xFFE1F5FE)
                        : const Color(0xFFE8F4FD),
                borderRadius: BorderRadius.circular(12),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(
                    slot.courseType.endsWith("LA")
                        ? FIcons.flaskConical
                        : FIcons.libraryBig,
                    size: 16,
                    color:
                        slot.courseType.endsWith("LA")
                            ? const Color(0xFF0277BD)
                            : const Color(0xFF1565C0),
                  ),
                  const SizedBox(width: 4),
                  Text(
                    slot.courseType.endsWith("LA") ? 'LAB' : 'LECTURE',
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                      color:
                          slot.courseType.endsWith("LA")
                              ? const Color(0xFF0277BD)
                              : const Color(0xFF1565C0),
                    ),
                  ),
                ],
              ),
            ),
            const Spacer(),
            if (statusStyle != null)
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: statusStyle.$2,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: statusStyle.$1, width: 1),
                ),
                child: Text(
                  statusStyle.$4,
                  style: TextStyle(
                    fontSize: 10,
                    fontWeight: FontWeight.bold,
                    color: statusStyle.$3,
                  ),
                ),
              ),
          ],
        ),
        const SizedBox(height: 8),

        Text(
          slot.name,
          style: const TextStyle(
            fontSize: 16,
            fontWeight: FontWeight.w600,
            //height: 1.2,
          ),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        const SizedBox(height: 8),

        Row(
          children: [
            Expanded(
              child: _buildDetailChip(
                icon: FIcons.mapPin,
                text: "${slot.block} - ${slot.roomNo}",
                color: const Color(0xFF6B7280),
              ),
            ),
            const SizedBox(width: 8),
            _buildDetailChip(
              icon: FIcons.hash,
              text: slot.courseCode,
              color: const Color(0xFF6B7280),
            ),
          ],
        ),
        const SizedBox(height: 8),

        Row(
          children: [
            Expanded(
              child: _buildDetailChip(
                icon: FIcons.clock,
                text: "${to12H(slot.startTime)} - ${to12H(slot.endTime)}",
                color: const Color(0xFF374151),
                isBold: true,
              ),
            ),
            const SizedBox(width: 8),
            _buildDetailChip(
              icon: FIcons.calendar,
              text: slot.slot,
              color: const Color(0xFF6B7280),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildFreeTimeCard(BuildContext context) {
    return Column(
      children: [
        Row(
          children: [
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
              decoration: BoxDecoration(
                color: const Color(0xFFFFF8E1),
                borderRadius: BorderRadius.circular(12),
              ),
              child: const Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(FIcons.hourglass, size: 16, color: Color(0xFFE65100)),
                  SizedBox(width: 4),
                  Text(
                    'FREE TIME',
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                      color: Color(0xFFE65100),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
        const SizedBox(height: 8),

        Row(
          children: [
            Expanded(
              child: _buildDetailChip(
                icon: FIcons.clock,
                text: "${to12H(slot.startTime)} - ${to12H(slot.endTime)}",
                color: const Color(0xFF374151),
                isBold: true,
              ),
            ),
            const SizedBox(width: 8),
            _buildDetailChip(
              icon: FIcons.timer,
              text:
                  slot.slot == "1" ? "${slot.slot} hour" : "${slot.slot} hours",
              color: const Color(0xFF6B7280),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildDetailChip({
    required IconData icon,
    required String text,
    required Color color,
    bool isBold = false,
  }) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      decoration: BoxDecoration(
        color: Colors.white.withValues(alpha: 0.7),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 14, color: color),
          const SizedBox(width: 4),
          Flexible(
            child: Text(
              text,
              style: TextStyle(
                fontSize: 13,
                fontWeight: isBold ? FontWeight.w600 : FontWeight.w400,
                color: color,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}

String to12H(String time) {
  var list = time.split(":");
  int hours = int.parse(list[0]);
  String period;

  if (hours > 12) {
    hours -= 12;
    period = "PM";
  } else if (hours == 12) {
    period = "PM";
  } else if (hours == 0) {
    hours = 12;
    period = "AM";
  } else {
    period = "AM";
  }

  return "$hours:${list[1]} $period";
}
