import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:http/http.dart' as http;
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:image_picker/image_picker.dart';
import 'package:pocketbase/pocketbase.dart';
import 'package:vitapmate/features/social/presentation/providers/msg_utils.dart';

class SocialAvatarUpdate extends HookConsumerWidget {
  final PocketBase pb;
  const SocialAvatarUpdate(this.pb, {super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final file = useState<File?>(null);
    final isUploading = useState(false);
    final picker = ImagePicker();
    final erroMsg = useState<String?>(null);

    Future<void> pickImage() async {
      final picked = await picker.pickImage(source: ImageSource.gallery);
      if (picked != null) {
        file.value = File(picked.path);
      }
    }

    Future<void> upload() async {
      if (file.value == null) {
        erroMsg.value = "Select a Image";
        return;
      }
      erroMsg.value = null;

      try {
        isUploading.value = true;
        await pb
            .collection('users')
            .update(
              pb.authStore.record!.id,
              body: {},
              files: [
                http.MultipartFile.fromBytes(
                  'avatar',
                  await file.value!.readAsBytes(),
                  filename: 'avatar.jpg',
                ),
              ],
            );
        var _ = ref.refresh(getUserrecordProvider(pb.authStore.record!.id));
        if (context.mounted) {
          Navigator.of(context).pop();
        }
      } catch (e) {
        erroMsg.value = "Upload failed ";
      } finally {
        isUploading.value = false;
      }
    }

    return FDialog(
      direction: Axis.horizontal,
      title: const Text('Upload an Image'),
      body: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          file.value != null
              ? Image.file(file.value!, height: 100)
              : const Text("No image selected."),
          const SizedBox(height: 10),
          FButton(onPress: pickImage, child: const Text("Choose Image")),
          SizedBox(height: 4),
          Text(erroMsg.value ?? ""),
        ],
      ),
      actions: [
        FButton(
          style: FButtonStyle.outline(),
          onPress: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        !isUploading.value
            ? FButton(onPress: upload, child: const Text('Update'))
            : SizedBox(
              width: 10,
              height: 10,
              child: CircularProgressIndicator(
                color: context.theme.colors.primary,
              ),
            ),
      ],
    );
  }
}
