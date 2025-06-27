import 'dart:io';
import 'package:path/path.dart' as p;
import 'package:http/http.dart' as http;
import 'package:pocketbase/pocketbase.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/features/social/presentation/providers/pocketbase.dart';
part 'message_chat.g.dart';

@riverpod
class MessageChat extends _$MessageChat {
  int _page = 1;
  int _totalPages = 0;
  final int _perpage = 10;
  List<RecordModel> _messages = [];
  @override
  Future<List<RecordModel>> build() async {
    PocketBase pb = await ref.watch(pbProvider.future);
    final resultList = await pb
        .collection('chat_messages')
        .getList(page: 1, perPage: _perpage, sort: '-created');
    _messages = resultList.items.reversed.toList();
    _totalPages = resultList.totalPages;
    pb.collection('chat_messages').subscribe('*', (e) {
      switch (e.action) {
        case 'create':
          _messages.add(e.record!);
          break;
        case 'update':
          final index = _messages.indexWhere((msg) => msg.id == e.record!.id);
          if (index != -1) {
            _messages[index] = e.record!;
          }
          break;
        case 'delete':
          _messages.removeWhere((msg) => msg.id == e.record!.id);
          break;
      }
      state = AsyncData([..._messages]);
    });

    ref.onDispose(() {
      pb.collection('chat_messages').unsubscribe('*');
    });

    return _messages;
  }

  Future<void> refreshMessages() async {
    final pb = await ref.read(pbProvider.future);
    final resultList = await pb
        .collection('chat_messages')
        .getList(page: 1, perPage: _perpage, sort: '-created');
    _page = 1;
    _totalPages = resultList.totalPages;
    _messages = resultList.items.reversed.toList();
    state = AsyncData([..._messages]);
  }

  Future create(String text, {required List<File> files, String? reply}) async {
    if (text.isEmpty) return;
    PocketBase pb = await ref.read(pbProvider.future);
    final body = <String, dynamic>{
      "text": text.trim(),
      "user": pb.authStore.record!.id,
    };

    var sfiles = [
      for (final image in files)
        http.MultipartFile.fromBytes(
          "files",
          await image.readAsBytes(),
          filename: p.basename(image.path),
        ),
    ];

    await pb.collection('chat_messages').create(body: body, files: sfiles);
  }

  Future<bool> next() async {
    final pb = await ref.read(pbProvider.future);
    if (_page >= _totalPages) return false;

    _page += 1;
    final resultList = await pb
        .collection('chat_messages')
        .getList(page: _page, perPage: _perpage, sort: '-created');

    final olderMessages = resultList.items.reversed.toList();
    _messages = [...olderMessages, ..._messages];
    state = AsyncData([..._messages]);
    return true;
  }
}
