import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/utils/general_utils.dart';

class DataUpdatedFooter extends StatelessWidget {
  final int updateTime;
  final EdgeInsetsGeometry padding;
  final double fontSize;
  final Color? color;

  const DataUpdatedFooter({
    super.key,
    required this.updateTime,
    this.padding = const EdgeInsets.only(top: 12, bottom: 20),
    this.fontSize = 13,
    this.color,
  });

  @override
  Widget build(BuildContext context) {
    if (updateTime <= 0) return const SizedBox.shrink();

    return Center(
      child: Padding(
        padding: padding,
        child: Text(
          "Data updated on ${formatUnixTimestamp(updateTime)}",
          textAlign: TextAlign.center,
          style: TextStyle(
            fontSize: fontSize,
            color: color ?? context.theme.colors.mutedForeground,
            fontStyle: FontStyle.italic,
          ),
        ),
      ),
    );
  }
}
