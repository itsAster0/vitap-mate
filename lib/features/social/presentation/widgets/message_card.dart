import 'dart:developer';

import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_parsed_text/flutter_parsed_text.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:intl/intl.dart';
import 'package:pocketbase/pocketbase.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/features/social/presentation/providers/msg_utils.dart';
import 'package:vitapmate/features/social/presentation/providers/pocketbase.dart';

class MessageCard extends HookConsumerWidget {
  final RecordModel model;
  const MessageCard({super.key, required this.model});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    Future<void> editMsg() async {
      showDialog(context: context, builder: (context) => UpdateMessage(model));
    }

    Future<void> longpress() async {
      var pb = await ref.read(pbProvider.future);
      if (pb.authStore.record!.id == model.getStringValue("user")) {
        if (context.mounted) {
          final overlay =
              // ignore: use_build_context_synchronously
              Overlay.of(context).context.findRenderObject() as RenderBox;
          final RenderBox box = context.findRenderObject() as RenderBox;
          final Offset offset = box.localToGlobal(
            Offset.zero,
            ancestor: overlay,
          );

          final result = await showMenu(
            color: context.theme.colors.primaryForeground,
            context: context,
            position: RelativeRect.fromLTRB(offset.dx, offset.dy, 0, 0),
            items: [
              PopupMenuItem(value: 'delete', child: Text('Delete')),
              PopupMenuItem(value: 'edit', child: Text('Edit')),
            ],
          );

          if (result == 'delete') {
            await pb.collection('chat_messages').delete(model.id);
          } else if (result == "edit") {
            editMsg();
          }
        }
      }
    }

    Future<void> press() async {}
    return FTappable(
      onPress: press,
      onLongPress: longpress,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 6, horizontal: 8),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Avatar
            GetAvatar(id: model.getStringValue("user")),

            const SizedBox(width: 8),

            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Flexible(
                        child: Row(
                          spacing: 6,
                          children: [
                            GetName(id: model.getStringValue("user")),
                            GetRole(id: model.getStringValue("user")),
                          ],
                        ),
                      ),
                      const SizedBox(width: 6),
                      Text(
                        formatPocketBaseTimestamp(
                          model.getStringValue("created"),
                        ),
                        style: const TextStyle(
                          fontSize: 12,
                          color: Colors.grey,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ),

                  const SizedBox(height: 4),
                  Container(
                    width: double.infinity,
                    padding: const EdgeInsets.all(8),
                    decoration: BoxDecoration(
                      color: context.theme.colors.primaryForeground,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: ParsedText(
                      text: model.getStringValue("text"),
                      style: const TextStyle(
                        fontSize: 16,
                        color: Colors.black87,
                      ),
                      parse: parse,
                    ),
                  ),
                  if (model.getStringValue("created") !=
                      model.getStringValue("updated"))
                    Row(
                      mainAxisAlignment: MainAxisAlignment.end,
                      children: [
                        Text(
                          "edited",
                          style: const TextStyle(
                            fontSize: 12,
                            color: Colors.grey,
                          ),
                        ),
                      ],
                    ),
                  if (model.getListValue("files").isNotEmpty)
                    SizedBox(height: 4),
                  if (model.getListValue("files").isNotEmpty)
                    SizedBox(
                      height: 100,
                      child: ListView.builder(
                        scrollDirection: Axis.horizontal,
                        itemCount: model.getListValue("files").length,
                        itemBuilder: (context, index) {
                          final i = model.getListValue("files")[index];
                          final img =
                              ref
                                  .read(pbProvider)
                                  .value!
                                  .files
                                  .getUrl(model, i)
                                  .toString();
                          return FTappable(
                            child: Padding(
                              padding: const EdgeInsets.only(right: 8),
                              child: ClipRRect(
                                borderRadius: BorderRadius.circular(8),
                                child: Image.network(
                                  img,
                                  height: 100,
                                  fit: BoxFit.cover,
                                ),
                              ),
                            ),
                            onPress: () {
                              showAdaptiveDialog(
                                context: context,
                                builder:
                                    (context) => FDialog(
                                      direction: Axis.horizontal,
                                      body: ClipRRect(
                                        borderRadius: BorderRadius.circular(8),
                                        child: Image.network(
                                          img,
                                          fit: BoxFit.cover,
                                        ),
                                      ),
                                      actions: [
                                        FButton(
                                          style: FButtonStyle.outline,
                                          onPress:
                                              () => Navigator.of(context).pop(),
                                          child: const Text('Close'),
                                        ),
                                        FButton(
                                          onPress: () {
                                            downloadFile(img, "");
                                            Navigator.of(context).pop();
                                          },
                                          child: const Text('Download'),
                                        ),
                                      ],
                                    ),
                              );
                            },
                          );
                        },
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

class GetAvatar extends HookConsumerWidget {
  final String id;
  const GetAvatar({super.key, required this.id});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final data = ref.watch(getUserrecordProvider(id));
    return data.when(
      data: (data) {
        final record = data.$1;
        final pb = data.$2;
        final avatarFilename = record.getStringValue('avatar');
        final img = pb.files.getUrl(record, avatarFilename).toString();
        return FAvatar(
          size: 40,
          image: NetworkImage(img),
          fallback: const Text('Lo'),
        );
      },
      error: (_, __) => FAvatar(image: NetworkImage(''), fallback: Text('ER')),
      loading: () => FAvatar(image: NetworkImage(''), fallback: Text('LO')),
    );
  }
}

final parse = <MatchText>[
  MatchText(
    type: ParsedType.URL,
    style: TextStyle(color: Colors.blue),
    onTap: (url) => launchUrl(Uri.parse(url)),
  ),
  MatchText(
    type: ParsedType.EMAIL,
    style: TextStyle(color: Colors.green),
    onTap: (email) => launchUrl(Uri(scheme: 'mailto', path: email)),
  ),
  MatchText(
    pattern: r"\B@(\w+)",
    style: TextStyle(color: Colors.purple),
    onTap: (mention) => log("Tapped on $mention"),
  ),
];

class GetName extends HookConsumerWidget {
  final String id;
  const GetName({super.key, required this.id});
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    var data = ref.watch(getUserrecordProvider(id));
    return data.when(
      data: (data) {
        PocketBase pb = data.$2;
        var record = data.$1;
        final name = record.getStringValue('name');
        return ConstrainedBox(
          constraints: BoxConstraints(
            maxWidth: MediaQuery.of(context).size.width * 0.6,
          ),
          child: Text(
            name,
            style: TextStyle(fontWeight: FontWeight.w600),
            overflow: TextOverflow.ellipsis,
            maxLines: 1,
          ),
        );
      },
      error: (e, se) {
        return SizedBox();
      },
      loading: () {
        return SizedBox();
      },
    );
  }
}

class GetRole extends HookConsumerWidget {
  final String id;
  const GetRole({super.key, required this.id});
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    var data = ref.watch(getRoleProvider(id));
    return data.when(
      data: (pb) {
        if (pb == null || pb == "student") {
          return SizedBox.shrink();
        }
        return FBadge(
          style: FBadgeStyle.outline,
          child: Text(pb, style: TextStyle(fontSize: 12)),
        );
      },
      error: (e, se) {
        return SizedBox.shrink();
      },
      loading: () {
        return SizedBox.shrink();
      },
    );
  }
}

class UpdateMessage extends HookConsumerWidget {
  final RecordModel model;
  const UpdateMessage(this.model, {super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final isLoading = useState(false);
    final erroMsg = useState<String?>(null);
    final controller = useTextEditingController(
      text: model.getStringValue("text"),
    );

    void update() async {
      final value = controller.text.trim();
      if (value.isEmpty) {
        erroMsg.value = "cannot be empty";
        return;
      } else if (value.length > 3600) {
        erroMsg.value = "Message too long";
        return;
      }
      try {
        isLoading.value = true;
        erroMsg.value = null;
        var pb = ref.read(pbProvider).value!;
        final body = <String, dynamic>{"text": value};

        await pb.collection('chat_messages').update(model.id, body: body);
        if (context.mounted) {
          Navigator.of(context).pop();
        }
      } catch (e) {
        erroMsg.value = "failed to edit message";
      } finally {
        isLoading.value = false;
      }
    }

    return FDialog(
      direction: Axis.horizontal,
      title: const Text('Edit Message'),

      body: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          FTextField(controller: controller, label: const Text("Message")),
          SizedBox(height: 8),
          Text(erroMsg.value ?? ""),
        ],
      ),
      actions: [
        FButton(
          style: FButtonStyle.outline,
          onPress: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        !isLoading.value
            ? FButton(onPress: update, child: const Text('Update'))
            : SizedBox(
              width: 10,
              height: 10,
              child: CircularProgressIndicator(color: Colors.black),
            ),
      ],
    );
  }
}

String formatPocketBaseTimestamp(String raw) {
  final dateTime = DateTime.parse(raw).toLocal();
  final now = DateTime.now();

  final isToday =
      now.year == dateTime.year &&
      now.month == dateTime.month &&
      now.day == dateTime.day;

  if (isToday) {
    return DateFormat.jm().format(dateTime);
  } else {
    return "${DateFormat('dd/MM/yy').format(dateTime)} ${DateFormat.jm().format(dateTime)}";
  }
}
