import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/features/social/presentation/providers/pocketbase.dart';
import 'package:vitapmate/features/social/presentation/widgets/message_screen.dart';

class MessageChatPage extends HookWidget {
  const MessageChatPage({super.key});

  @override
  Widget build(BuildContext context) {
    notifCheck();
    return MessageScreen();
  }
}

//for backword compatability
void notifCheck() {
  Future.microtask(() async {
    final pref = SharedPreferencesAsync();
    final bool done = await pref.getBool("chat_notif_toke_send") ?? false;
    if (!done) {
      pbSetNtotification();
    }
  });
}
