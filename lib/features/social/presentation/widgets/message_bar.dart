import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:file_picker/file_picker.dart';
import 'package:image_picker/image_picker.dart';
import 'package:vitapmate/features/social/presentation/providers/message_chat.dart';
import 'package:vitapmate/features/social/presentation/widgets/message_card.dart';

class MessageBar extends HookConsumerWidget {
  const MessageBar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final controller = useTextEditingController();
    final isLoading = useState(false);
    final files = useState<List<File>>([]);
    final picker = ImagePicker();
    final errorMsg = useState<String?>(null);

    final replyingMessage = ref.watch(replyingMessageProvider);
    final editingMessage = ref.watch(editingMessageProvider);
    final isReplying = replyingMessage != null;
    final isEditing = editingMessage != null;

    final fileOperations = useMemoized(() => _FileOperations(), []);

    useEffect(() {
      if (isEditing) {
        controller.text = editingMessage.getStringValue("text");
        // WidgetsBinding.instance.addPostFrameCallback((_) {

        // });
      } else if (!isReplying) {
        controller.clear();
        files.value = [];
      }
      return null;
    }, [isEditing, isReplying]);

    useEffect(() {
      void listener() {
        if (errorMsg.value != null) {
          errorMsg.value = null;
        }
      }

      controller.addListener(listener);
      return () => controller.removeListener(listener);
    }, [controller]);

    void cancelReply() {
      ref.read(replyingMessageProvider.notifier).state = null;
      HapticFeedback.lightImpact();
    }

    void cancelEdit() {
      ref.read(editingMessageProvider.notifier).state = null;
      HapticFeedback.lightImpact();
    }

    Future<void> pickFiles() async {
      try {
        HapticFeedback.lightImpact();

        FilePickerResult? result = await FilePicker.platform.pickFiles(
          allowMultiple: true,
          type: FileType.any,
        );

        if (result != null) {
          final newFiles = await fileOperations.processFiles(result.files);

          if (newFiles.isNotEmpty) {
            files.value = [...files.value, ...newFiles];
            HapticFeedback.selectionClick();
          }

          if (newFiles.length < result.files.length) {
            errorMsg.value = "Some files were too large (max 10MB each)";
          }
        }
      } catch (e) {
        errorMsg.value = "Failed to pick files";
      }
    }

    Future<void> pickMultipleImages() async {
      try {
        HapticFeedback.lightImpact();

        final picked = await picker.pickMultiImage(
          imageQuality: 80,
          maxWidth: 1920,
          maxHeight: 1920,
        );

        if (picked.isNotEmpty) {
          final newFiles = await fileOperations.processXFiles(picked);

          if (newFiles.isNotEmpty) {
            files.value = [...files.value, ...newFiles];
            HapticFeedback.selectionClick();
          }

          if (newFiles.length < picked.length) {
            errorMsg.value = "Some files were too large (max 10MB each)";
          }
        }
      } catch (e) {
        errorMsg.value = "Failed to pick images";
      }
    }

    void removeFile(int index) {
      HapticFeedback.lightImpact();
      final newFiles = List<File>.from(files.value);
      newFiles.removeAt(index);
      files.value = newFiles;
    }

    Future<void> sendMessage() async {
      try {
        errorMsg.value = null;
        if (isLoading.value) return;

        final value = controller.text.trim();
        if (value.isEmpty) {
          errorMsg.value = "Enter some text or add a file";
          HapticFeedback.lightImpact();
          return;
        }

        if (value.length > 3600) {
          errorMsg.value = "Message too long (max 3600 characters)";
          HapticFeedback.lightImpact();
          return;
        }

        isLoading.value = true;
        HapticFeedback.mediumImpact();

        if (isEditing) {
          await ref
              .read(messageChatProvider.notifier)
              .updateMessage(
                editingMessage.id,
                value,
                files: files.value.isNotEmpty ? files.value : null,
              );

          ref.read(editingMessageProvider.notifier).state = null;
        } else {
          await ref
              .read(messageChatProvider.notifier)
              .create(
                value,
                files: files.value,
                replyToMessageId: replyingMessage?.id,
              );

          if (isReplying) {
            ref.read(replyingMessageProvider.notifier).state = null;
          }
        }

        controller.clear();
        files.value = [];

        HapticFeedback.lightImpact();
      } catch (e) {
        errorMsg.value =
            isEditing ? "Failed to update message" : "Failed to send message";
        HapticFeedback.mediumImpact();
      } finally {
        isLoading.value = false;
      }
    }

    return Container(
      decoration: BoxDecoration(
        color: context.theme.colors.primaryForeground,
        border: Border(
          top: BorderSide(color: Colors.grey.withValues(alpha: 0.2), width: 1),
        ),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12.0),
        child: Column(
          children: [
            if (isReplying) ...[
              ReplyIndicator(message: replyingMessage, onCancel: cancelReply),
              const SizedBox(height: 12),
            ],

            if (isEditing) ...[
              EditIndicator(message: editingMessage, onCancel: cancelEdit),
              const SizedBox(height: 12),
            ],

            if (files.value.isNotEmpty)
              Container(
                margin: const EdgeInsets.only(bottom: 12),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Text(
                          '${files.value.length} file${files.value.length > 1 ? 's' : ''} selected',
                          style: TextStyle(
                            fontSize: 12,
                            color: Colors.grey[600],
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                        const Spacer(),
                        FTappable(
                          onPress: () => files.value = [],
                          child: Container(
                            padding: const EdgeInsets.all(4),
                            decoration: BoxDecoration(
                              color: Colors.red.withValues(alpha: 0.1),
                              borderRadius: BorderRadius.circular(12),
                            ),
                            child: const Icon(
                              Icons.close,
                              size: 16,
                              color: Colors.red,
                            ),
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 8),
                    SizedBox(
                      height: 80,
                      child: ListView.separated(
                        scrollDirection: Axis.horizontal,
                        itemCount: files.value.length,
                        separatorBuilder: (_, _) => const SizedBox(width: 8),
                        itemBuilder: (context, index) {
                          final file = files.value[index];
                          final filename = file.path.split('/').last;

                          return _FilePreview(
                            file: file,
                            filename: filename,
                            onRemove: () => removeFile(index),
                          );
                        },
                      ),
                    ),
                  ],
                ),
              ),

            Row(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                if (!isEditing)
                  PopupMenuButton<String>(
                    icon: Container(
                      padding: const EdgeInsets.all(8),
                      decoration: BoxDecoration(
                        color: context.theme.colors.primaryForeground,
                        borderRadius: BorderRadius.circular(20),
                      ),
                      child: const Icon(Icons.add, size: 20),
                    ),
                    color: context.theme.colors.primaryForeground,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(12),
                    ),
                    onSelected: (value) {
                      switch (value) {
                        case 'files':
                          pickFiles();
                          break;
                        case 'Images':
                          pickMultipleImages();
                          break;
                      }
                    },
                    itemBuilder:
                        (context) => [
                          const PopupMenuItem(
                            value: 'files',
                            child: Row(
                              children: [
                                Icon(Icons.attach_file, size: 18),
                                SizedBox(width: 8),
                                Text('Files'),
                              ],
                            ),
                          ),
                          const PopupMenuItem(
                            value: 'Images',
                            child: Row(
                              children: [
                                Icon(Icons.photo_library, size: 18),
                                SizedBox(width: 8),
                                Text('Images'),
                              ],
                            ),
                          ),
                        ],
                  ),

                if (!isEditing) const SizedBox(width: 8),

                Expanded(
                  child: FTextField.multiline(
                    controller: controller,
                    hint:
                        isEditing
                            ? "Edit message..."
                            : isReplying
                            ? "Reply to message..."
                            : "Type a message...",
                    maxLines: 6,
                    minLines: 1,
                    enabled: !isLoading.value,
                    onSubmit: (value) {
                      if (!HardwareKeyboard.instance.isShiftPressed) {
                        sendMessage();
                      }
                    },
                  ),
                ),

                const SizedBox(width: 8),
                AnimatedContainer(
                  duration: const Duration(milliseconds: 200),
                  child: FTappable(
                    onPress: isLoading.value ? null : sendMessage,
                    child: Container(
                      padding: const EdgeInsets.all(12),
                      decoration: BoxDecoration(
                        color:
                            isLoading.value
                                ? Colors.grey.withValues(alpha: 0.3)
                                : isEditing
                                ? Colors.orange
                                : isReplying
                                ? Colors.blue
                                : Colors.black,
                        borderRadius: BorderRadius.circular(20),
                      ),
                      child:
                          isLoading.value
                              ? const SizedBox(
                                width: 16,
                                height: 16,
                                child: CircularProgressIndicator(
                                  strokeWidth: 2,
                                  color: Colors.white,
                                ),
                              )
                              : Icon(
                                isEditing
                                    ? Icons.check
                                    : isReplying
                                    ? Icons.reply
                                    : Icons.send,
                                size: 18,
                                color: Colors.white,
                              ),
                    ),
                  ),
                ),
              ],
            ),

            if (errorMsg.value != null) ...[
              const SizedBox(height: 8),
              Container(
                width: double.infinity,
                padding: const EdgeInsets.all(8),
                decoration: BoxDecoration(
                  color: Colors.red.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: Colors.red.withValues(alpha: 0.3)),
                ),
                child: Row(
                  children: [
                    const Icon(Icons.error, color: Colors.red, size: 16),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        errorMsg.value!,
                        style: const TextStyle(color: Colors.red, fontSize: 12),
                      ),
                    ),
                  ],
                ),
              ),
            ],

            if (controller.text.isNotEmpty)
              Padding(
                padding: const EdgeInsets.only(top: 4),
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.end,
                  children: [
                    Text(
                      '${controller.text.length}/3600',
                      style: TextStyle(
                        fontSize: 11,
                        color:
                            controller.text.length > 3600
                                ? Colors.red
                                : Colors.grey[500],
                      ),
                    ),
                  ],
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _FileOperations {
  Future<List<File>> processFiles(List<PlatformFile> platformFiles) async {
    final newFiles = <File>[];

    for (final platformFile in platformFiles) {
      if (platformFile.path != null) {
        final file = File(platformFile.path!);
        final fileSize = await file.length();

        if (fileSize <= 10 * 1024 * 1024) {
          newFiles.add(file);
        }
      }
    }

    return newFiles;
  }

  Future<List<File>> processXFiles(List<XFile> xFiles) async {
    final newFiles = <File>[];

    for (final xFile in xFiles) {
      final file = File(xFile.path);
      final fileSize = await file.length();

      if (fileSize <= 10 * 1024 * 1024) {
        newFiles.add(file);
      }
    }

    return newFiles;
  }
}

class _FilePreview extends StatelessWidget {
  final File file;
  final String filename;
  final VoidCallback onRemove;

  const _FilePreview({
    required this.file,
    required this.filename,
    required this.onRemove,
  });

  String getFileExtension(String filename) {
    return filename.toLowerCase().split('.').last;
  }

  bool isImage(String filename) {
    final ext = getFileExtension(filename);
    return ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp'].contains(ext);
  }

  IconData getFileIcon(String filename) {
    final ext = getFileExtension(filename);

    if (['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp'].contains(ext)) {
      return Icons.image;
    } else if ([
      'mp4',
      'avi',
      'mov',
      'wmv',
      'flv',
      'webm',
      'mkv',
    ].contains(ext)) {
      return Icons.video_file;
    } else if (['mp3', 'wav', 'flac', 'aac', 'ogg', 'm4a'].contains(ext)) {
      return Icons.audio_file;
    } else if (['pdf', 'doc', 'docx', 'txt', 'rtf', 'odt'].contains(ext)) {
      return Icons.description;
    } else if (['zip', 'rar', '7z', 'tar', 'gz'].contains(ext)) {
      return Icons.archive;
    }
    return Icons.insert_drive_file;
  }

  Color getFileColor(String filename) {
    final ext = getFileExtension(filename);

    if (['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp'].contains(ext)) {
      return Colors.green;
    } else if ([
      'mp4',
      'avi',
      'mov',
      'wmv',
      'flv',
      'webm',
      'mkv',
    ].contains(ext)) {
      return Colors.red;
    } else if (['mp3', 'wav', 'flac', 'aac', 'ogg', 'm4a'].contains(ext)) {
      return Colors.purple;
    } else if (['pdf', 'doc', 'docx', 'txt', 'rtf', 'odt'].contains(ext)) {
      return Colors.blue;
    } else if (['zip', 'rar', '7z', 'tar', 'gz'].contains(ext)) {
      return Colors.orange;
    }
    return Colors.grey;
  }

  @override
  Widget build(BuildContext context) {
    return Stack(
      children: [
        Container(
          width: 80,
          height: 80,
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(12),
            border: Border.all(color: Colors.grey.withValues(alpha: 0.2)),
          ),
          child: ClipRRect(
            borderRadius: BorderRadius.circular(12),
            child:
                isImage(filename)
                    ? Image.file(file, height: 80, width: 80, fit: BoxFit.cover)
                    : Container(
                      color: getFileColor(filename).withValues(alpha: 0.1),
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          Icon(
                            getFileIcon(filename),
                            size: 24,
                            color: getFileColor(filename),
                          ),
                          const SizedBox(height: 4),
                          Padding(
                            padding: const EdgeInsets.symmetric(horizontal: 4),
                            child: Text(
                              filename.length > 10
                                  ? '${filename.substring(0, 7)}...'
                                  : filename,
                              style: TextStyle(
                                fontSize: 8,
                                color: getFileColor(filename),
                                fontWeight: FontWeight.w500,
                              ),
                              textAlign: TextAlign.center,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        ],
                      ),
                    ),
          ),
        ),
        Positioned(
          top: 4,
          right: 4,
          child: FTappable(
            onPress: onRemove,
            child: Container(
              padding: const EdgeInsets.all(2),
              decoration: const BoxDecoration(
                color: Colors.red,
                shape: BoxShape.circle,
              ),
              child: const Icon(Icons.close, size: 12, color: Colors.white),
            ),
          ),
        ),
      ],
    );
  }
}

class ReplyIndicator extends ConsumerWidget {
  final dynamic message;
  final VoidCallback onCancel;

  const ReplyIndicator({
    super.key,
    required this.message,
    required this.onCancel,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final messageText = message.getStringValue("text");
    final userId = message.getStringValue("user");

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Colors.blue.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: Colors.blue.withValues(alpha: 0.3)),
      ),
      child: Row(
        children: [
          Container(
            width: 4,
            height: 40,
            decoration: BoxDecoration(
              color: Colors.blue,
              borderRadius: BorderRadius.circular(2),
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Icon(Icons.reply, size: 16, color: Colors.blue[700]),
                    const SizedBox(width: 4),
                    Text(
                      'Replying to',
                      style: TextStyle(
                        fontSize: 12,
                        fontWeight: FontWeight.w600,
                        color: Colors.blue[700],
                      ),
                    ),
                    const SizedBox(width: 4),
                    Flexible(child: UserName(userId: userId)),
                  ],
                ),
                const SizedBox(height: 4),
                Text(
                  messageText.length > 50
                      ? '${messageText.substring(0, 50)}...'
                      : messageText,
                  style: TextStyle(fontSize: 12, color: Colors.blue[600]),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
          FTappable(
            onPress: onCancel,
            child: Container(
              padding: const EdgeInsets.all(4),
              decoration: BoxDecoration(
                color: Colors.red.withValues(alpha: 0.1),
                borderRadius: BorderRadius.circular(12),
              ),
              child: const Icon(Icons.close, size: 16, color: Colors.red),
            ),
          ),
        ],
      ),
    );
  }
}

class EditIndicator extends ConsumerWidget {
  final dynamic message;
  final VoidCallback onCancel;

  const EditIndicator({
    super.key,
    required this.message,
    required this.onCancel,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final messageText = message.getStringValue("text");
    final userId = message.getStringValue("user");

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: Colors.orange.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: Colors.orange.withValues(alpha: 0.3)),
      ),
      child: Row(
        children: [
          Container(
            width: 4,
            height: 40,
            decoration: BoxDecoration(
              color: Colors.orange,
              borderRadius: BorderRadius.circular(2),
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Icon(Icons.edit, size: 16, color: Colors.orange[700]),
                    const SizedBox(width: 4),
                    Text(
                      'Editing message from',
                      style: TextStyle(
                        fontSize: 12,
                        fontWeight: FontWeight.w600,
                        color: Colors.orange[700],
                      ),
                    ),
                    const SizedBox(width: 4),
                    Flexible(child: UserName(userId: userId)),
                  ],
                ),
                const SizedBox(height: 4),
                Text(
                  messageText.length > 50
                      ? '${messageText.substring(0, 50)}...'
                      : messageText,
                  style: TextStyle(fontSize: 12, color: Colors.orange[600]),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
          FTappable(
            onPress: onCancel,
            child: Container(
              padding: const EdgeInsets.all(4),
              decoration: BoxDecoration(
                color: Colors.red.withValues(alpha: 0.1),
                borderRadius: BorderRadius.circular(12),
              ),
              child: const Icon(Icons.close, size: 16, color: Colors.red),
            ),
          ),
        ],
      ),
    );
  }
}
