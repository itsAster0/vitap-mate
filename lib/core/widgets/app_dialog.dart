import 'package:flutter/material.dart';
import 'package:forui/forui.dart';

/// App-level dialog layout retained across Forui's builder-only dialog API.
class AppDialog extends StatelessWidget {
  const AppDialog({
    required this.actions,
    this.title,
    this.body,
    this.direction = Axis.vertical,
    this.animation,
    this.semanticsLabel,
    this.constraints = const BoxConstraints(minWidth: 280, maxWidth: 560),
    this.resizeToAvoidInsets = true,
    super.key,
  });

  final Widget? title;
  final Widget? body;
  final List<Widget> actions;
  final Axis direction;
  final Animation<double>? animation;
  final String? semanticsLabel;
  final BoxConstraints constraints;
  final bool resizeToAvoidInsets;

  @override
  Widget build(BuildContext context) => FDialog(
    animation: animation,
    semanticsLabel: semanticsLabel,
    constraints: constraints,
    resizeToAvoidInsets: resizeToAvoidInsets,
    builder: (context, style) => Padding(
      padding: const EdgeInsets.all(24),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (title case final title?)
            DefaultTextStyle(style: style.titleTextStyle, child: title),
          if (title != null && body != null) const SizedBox(height: 8),
          if (body case final body?)
            Flexible(
              child: DefaultTextStyle(style: style.bodyTextStyle, child: body),
            ),
          if ((title != null || body != null) && actions.isNotEmpty)
            const SizedBox(height: 24),
          if (direction == Axis.horizontal)
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              spacing: 8,
              children: actions,
            )
          else
            Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              spacing: 8,
              children: actions,
            ),
        ],
      ),
    ),
  );
}
