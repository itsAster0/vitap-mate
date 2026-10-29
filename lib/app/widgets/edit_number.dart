import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/ui/app_dialog.dart';

/// Asks for a whole number above 0 and saves it with [onSave].
Future<void> editNumber(
  BuildContext context, {
  required String title,
  required String prompt,
  required int current,
  required Future<void> Function(int value) onSave,
}) => editText(
  context,
  title: title,
  prompt: prompt,
  current: '$current',
  keyboardType: .number,
  inputFormatters: [FilteringTextInputFormatter.digitsOnly],
  check: (text) =>
      (int.tryParse(text) ?? 0) > 0 ? null : 'Enter a number greater than 0.',
  onSave: (text) => onSave(int.parse(text)),
);

/// Asks for a value, trimmed; [check] returns why it cannot be saved, or
/// null. Saves it with [onSave].
Future<void> editText(
  BuildContext context, {
  required String title,
  required String prompt,
  required String current,
  required String? Function(String text) check,
  required Future<void> Function(String text) onSave,
  TextInputType? keyboardType,
  List<TextInputFormatter>? inputFormatters,
}) => showAdaptiveDialog<void>(
  context: context,
  barrierDismissible: true,
  builder: (_) => HookBuilder(
    builder: (dialogContext) {
      final controller = useTextEditingController(text: current);
      final errorText = useState<String?>(null);
      return AppDialog(
        title: Text(title),
        body: Column(
          mainAxisSize: .min,
          crossAxisAlignment: .start,
          children: [
            Text(prompt),
            const SizedBox(height: 8),
            FTextField(
              control: FTextFieldControl.managed(controller: controller),
              keyboardType: keyboardType,
              inputFormatters: inputFormatters,
            ),
            if (errorText.value case final error?) ...[
              const SizedBox(height: 8),
              Text(
                error,
                style: dialogContext.theme.typography.body.sm.copyWith(
                  color: dialogContext.theme.colors.destructive,
                ),
              ),
            ],
          ],
        ),
        actions: [
          FButton(
            variant: .outline,
            onPress: () => Navigator.of(dialogContext).pop(),
            child: const Text('Cancel'),
          ),
          FButton(
            onPress: () async {
              final text = controller.text.trim();
              if (check(text) case final problem?) {
                errorText.value = problem;
                return;
              }
              await onSave(text);
              if (dialogContext.mounted) Navigator.of(dialogContext).pop();
              if (context.mounted) dispToast(context, 'Saved', '$title: $text');
            },
            child: const Text('Save'),
          ),
        ],
      );
    },
  ),
);
