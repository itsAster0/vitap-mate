import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:intl/intl.dart';

/// Small "Updated 3 min ago" line under synced data. Refreshes itself each
/// minute and falls back to a date once the data is older than a day.
class DataUpdatedFooter extends HookWidget {
  final int updateTime;
  final EdgeInsetsGeometry padding;
  final double fontSize;
  final Color? color;

  const DataUpdatedFooter({
    super.key,
    required this.updateTime,
    this.padding = const EdgeInsets.only(top: 16, bottom: 20),
    this.fontSize = 12,
    this.color,
  });

  @override
  Widget build(BuildContext context) {
    final now = useState(DateTime.now());
    useEffect(() {
      final timer = Timer.periodic(
        const Duration(minutes: 1),
        (_) => now.value = DateTime.now(),
      );
      return timer.cancel;
    }, const []);

    if (updateTime <= 0) return const SizedBox.shrink();
    final updated = DateTime.fromMillisecondsSinceEpoch(updateTime * 1000);

    return Center(
      child: Padding(
        padding: padding,
        child: Text(
          'Updated ${relativeTime(updated, now.value)}',
          textAlign: TextAlign.center,
          style: context.theme.typography.body.xs.copyWith(
            fontSize: fontSize,
            color: color ?? context.theme.colors.mutedForeground,
          ),
        ),
      ),
    );
  }
}

/// "just now", "5 min ago", "2 hr ago", "yesterday 4:10 PM", "25 Sep".
String relativeTime(DateTime then, DateTime now) {
  final diff = now.difference(then);
  if (diff.inMinutes < 1) return 'just now';
  if (diff.inMinutes < 60) return '${diff.inMinutes} min ago';
  if (diff.inHours < 12) return '${diff.inHours} hr ago';
  final today = DateTime(now.year, now.month, now.day);
  final day = DateTime(then.year, then.month, then.day);
  final days = today.difference(day).inDays;
  final time = DateFormat('h:mm a').format(then);
  if (days == 0) return 'today $time';
  if (days == 1) return 'yesterday $time';
  return then.year == now.year
      ? DateFormat('d MMM').format(then)
      : DateFormat('d MMM y').format(then);
}
