import 'package:flutter/material.dart';
import 'package:forui/forui.dart';

/// App-level titled card layout for Forui's builder-only card API.
class AppCard extends StatelessWidget {
  const AppCard({required this.child, this.title, this.subtitle, super.key});

  final Widget? title;
  final Widget? subtitle;
  final Widget child;

  @override
  Widget build(BuildContext context) => FCard(
    child: child,
    builder: (context, style, child) => Padding(
      padding: const EdgeInsets.all(16),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (title case final title?)
            DefaultTextStyle(style: style.titleTextStyle, child: title),
          if (subtitle case final subtitle?) ...[
            if (title != null) const SizedBox(height: 4),
            DefaultTextStyle(style: style.subtitleTextStyle, child: subtitle),
          ],
          if (title != null || subtitle != null) const SizedBox(height: 16),
          child!,
        ],
      ),
    ),
  );
}
