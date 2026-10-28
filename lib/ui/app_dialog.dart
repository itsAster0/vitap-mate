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

/// A yes/no question: true only when [action] is pressed. [body] is a
/// widget or a message. [destructive] colours the action red; [actionFirst]
/// puts it above [cancel].
Future<bool> confirm(
  BuildContext context, {
  required String title,
  required Object body,
  required String action,
  String cancel = 'Cancel',
  bool destructive = false,
  bool actionFirst = false,
  Axis direction = Axis.vertical,
}) async {
  final confirmed = await showFDialog<bool>(
    context: context,
    useRootNavigator: true,
    builder: (dialogContext, _, animation) {
      final yes = FButton(
        variant: destructive
            ? FButtonVariant.destructive
            : FButtonVariant.primary,
        onPress: () => Navigator.of(dialogContext).pop(true),
        child: Text(action),
      );
      final no = FButton(
        variant: FButtonVariant.outline,
        onPress: () => Navigator.of(dialogContext).pop(false),
        child: Text(cancel),
      );
      return AppDialog(
        animation: animation,
        direction: direction,
        title: Text(title),
        body: body is Widget ? body : Text('$body'),
        actions: actionFirst ? [yes, no] : [no, yes],
      );
    },
  );
  return confirmed ?? false;
}
