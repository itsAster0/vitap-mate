import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:pocketbase/pocketbase.dart';
import 'package:vitapmate/features/social/presentation/providers/msg_utils.dart';
import 'package:vitapmate/features/social/presentation/widgets/message_card.dart';

class SocialUsernameUpdate extends HookConsumerWidget {
  final PocketBase pb;
  const SocialUsernameUpdate(this.pb, {super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isLoading = useState(false);
    final erroMsg = useState<String?>(null);
    final controller = useTextEditingController();

    void change() async {
      final username = controller.text.trim();
      if (username.length < 4 || username.length > 12) {
        erroMsg.value = "Username must be 4–12 characters long";
        return;
      }
      try {
        isLoading.value = true;
        erroMsg.value = null;
        final updatedUser = await pb
            .collection('users')
            .update(pb.authStore.record!.id, body: {'name': username});
        var _ = ref.refresh(getUserrecordProvider(pb.authStore.record!.id));
        if (context.mounted) {
          Navigator.of(context).pop();
        }
      } catch (e) {
        erroMsg.value = "failed to change username";
      } finally {
        isLoading.value = false;
      }
    }

    return FDialog(
      direction: Axis.horizontal,
      title: const Text('Select a Name'),

      body: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          UserName(userId: pb.authStore.record!.id),
          FTextField(controller: controller, label: const Text("New Username")),
          SizedBox(height: 8),
          Text(erroMsg.value ?? ""),
        ],
      ),
      actions: [
        FButton(
          style: FButtonStyle.outline(),
          onPress: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        !isLoading.value
            ? FButton(onPress: change, child: const Text('Update'))
            : SizedBox(
              width: 10,
              height: 10,
              child: CircularProgressIndicator(color: context.theme.colors.primaryForeground),
            ),
      ],
    );
  }
}
