import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:vitapmate/core/router/paths.dart';

class SyncGoogleCalendarButton extends HookConsumerWidget {
  const SyncGoogleCalendarButton({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return FButton.icon(
      onPress: () => GoRouter.of(context).pushNamed(Paths.calendarSync),
      child: const Icon(Icons.calendar_month),
    );
  }
}
