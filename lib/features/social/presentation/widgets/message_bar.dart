import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:image_picker/image_picker.dart';
import 'package:vitapmate/features/social/presentation/providers/message_chat.dart';

class MessageBar extends HookConsumerWidget {
  final ValueNotifier toend;
  const MessageBar({super.key, required this.toend});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    var controller = useTextEditingController();
    var isloading = useState(false);
    final file = useState<List<File>>([]);
    final picker = ImagePicker();
    final erroMsg = useState<String?>(null);
    Future<void> pickImage() async {
      final picked = await picker.pickImage(source: ImageSource.gallery);
      if (picked != null && !file.value.contains(File(picked.path))) {
        file.value = [...file.value, File(picked.path)];
      }
    }

    Future<void> sendMessage() async {
      try {
        erroMsg.value = null;
        if (isloading.value) return;
        isloading.value = true;
        var value = controller.text.trim();
        if (value.isEmpty) {
          erroMsg.value = "Enter some text";
          return;
        } else if (value.length > 3600) {
          erroMsg.value = "Message too long";
          return;
        }
        await ref
            .read(messageChatProvider.notifier)
            .create(value, files: file.value);
        controller.clear();
        file.value = [];
        toend.value = !toend.value;
      } catch (e) {
        erroMsg.value = "Failed to send message";
      } finally {
        isloading.value = false;
      }
    }

    return Padding(
      padding: const EdgeInsets.all(8.0),
      child: Column(
        children: [
          if (file.value.isNotEmpty)
            Row(
              children: [
                Expanded(
                  child: SizedBox(
                    height: 100,
                    child: ListView.builder(
                      scrollDirection: Axis.horizontal,
                      itemCount: file.value.length,
                      itemBuilder: (context, index) {
                        final i = file.value[index];
                        return Padding(
                          padding: const EdgeInsets.only(right: 8),
                          child: ClipRRect(
                            borderRadius: BorderRadius.circular(8),
                            child: Image.file(
                              i,
                              height: 100,
                              fit: BoxFit.cover,
                            ),
                          ),
                        );
                      },
                    ),
                  ),
                ),
                IconButton(
                  onPressed: () {
                    file.value = [];
                  },
                  icon: Icon(FIcons.badgeX),
                ),
              ],
            ),
          Row(
            crossAxisAlignment: CrossAxisAlignment.end,
            children: [
              IconButton(onPressed: pickImage, icon: Icon(FIcons.plus)),
              Expanded(
                child: Column(
                  children: [
                    if (file.value.isNotEmpty) SizedBox(height: 4),
                    FTextField.multiline(
                      controller: controller,
                      hint: "Message",
                      // autocorrect: true,
                      onSubmit: (value) {
                        controller.text = "${controller.text}\n";
                      },
                      maxLines: 6,
                      minLines: 1,
                    ),
                  ],
                ),
              ),
              IconButton(
                onPressed: sendMessage,
                icon: Icon(isloading.value ? FIcons.loader : FIcons.send),
              ),
            ],
          ),
          if (erroMsg.value != null) Text(erroMsg.value ?? ""),
        ],
      ),
    );
  }
}
