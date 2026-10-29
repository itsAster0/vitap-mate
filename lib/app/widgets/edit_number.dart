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
}) => showAdaptiveDialog<void>(
  context: context,
  barrierDismissible: true,
  builder: (_) => HookBuilder(
    builder: (dialogContext) {
      final controller = useTextEditingController(text: '$current');
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
              keyboardType: .number,
              inputFormatters: [FilteringTextInputFormatter.digitsOnly],
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
              final value = int.tryParse(controller.text.trim());
              if (value == null || value <= 0) {
                errorText.value = 'Enter a number greater than 0.';
                return;
              }
              await onSave(value);
              if (dialogContext.mounted) Navigator.of(dialogContext).pop();
              if (context.mounted) {
                dispToast(context, 'Saved', '$title: $value');
              }
            },
            child: const Text('Save'),
          ),
        ],
      );
    },
  ),
);
