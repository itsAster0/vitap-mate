import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:vitapmate/features/social/presentation/providers/pocketbase.dart';
import 'package:vitapmate/features/social/presentation/widgets/message_screen.dart';

class MessageChatPage extends HookWidget {
  const MessageChatPage({super.key});

  @override
  Widget build(BuildContext context) {
    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        Future(() async {
          try {
            await pbSetNtotification();
          } catch (e, _) {
            ();
          }
        });
      });
      return null;
    }, []);
    return MessageScreen();
  }
}
