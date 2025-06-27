import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/social/presentation/providers/message_chat.dart';
import 'package:vitapmate/features/social/presentation/providers/pocketbase.dart';
import 'package:vitapmate/features/social/presentation/widgets/message_bar.dart';
import 'package:vitapmate/features/social/presentation/widgets/message_card.dart';

class MessageScreen extends StatefulHookConsumerWidget {
  final ValueNotifier toend;
  const MessageScreen({super.key, required this.toend});

  @override
  ConsumerState<MessageScreen> createState() => _MessageScreenState();
}

class _MessageScreenState extends ConsumerState<MessageScreen>
    with WidgetsBindingObserver {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      ref.read(messageChatProvider.notifier).refreshMessages();
    }
  }

  @override
  Widget build(BuildContext context) {
    var data = ref.watch(messageChatProvider);

    return data.when(
      data:
          (data) => Column(
            children: [
              Expanded(child: ChatScreen(data: data, toend: widget.toend)),
              MessageBar(toend: widget.toend),
            ],
          ),
      error: (e, se) {
        disCommonToast(context, e);
        String msg = commonErrorMessage(e);
        return RefreshIndicator(
          backgroundColor: Colors.white,
          color: Colors.black,
          onRefresh: () async {
            var _ = await ref.refresh(pbProvider.future);
          },
          child: SingleChildScrollView(
            physics: const AlwaysScrollableScrollPhysics(),
            child: ConstrainedBox(
              constraints: BoxConstraints(
                minHeight: MediaQuery.of(context).size.height * 0.8,
              ),
              child: Center(child: Text(msg)),
            ),
          ),
        );
      },
      loading:
          () => const Center(
            child: SizedBox(
              width: 50,
              height: 50,
              child: CircularProgressIndicator(color: Colors.black),
            ),
          ),
    );
  }
}

class ChatScreen extends HookConsumerWidget {
  final List data;
  final ValueNotifier toend;
  const ChatScreen({super.key, required this.data, required this.toend});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final scrollController = useScrollController();
    final showScrollToBottom = useState(false);
    final previousMessageCount = useRef(0);
    final isUserScrolledUp = useRef(false);

    useEffect(() {
      void onScroll() {
        if (scrollController.hasClients) {
          final isAtBottom =
              scrollController.position.pixels >=
              scrollController.position.maxScrollExtent - 100;

          // Only update isUserScrolledUp when user manually scrolls
          if (!isAtBottom && scrollController.position.pixels > 0) {
            isUserScrolledUp.value = true;
          } else if (isAtBottom) {
            isUserScrolledUp.value = false;
            showScrollToBottom.value = false;
          }
        }
      }

      scrollController.addListener(onScroll);
      return () => scrollController.removeListener(onScroll);
    }, [scrollController]);

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (scrollController.hasClients) {
          final currentMessageCount = data.length;
          final hasNewMessages =
              currentMessageCount > previousMessageCount.value;

          if (hasNewMessages) {
            final isNewMessage = toend.value != previousMessageCount.value;
            if (isUserScrolledUp.value && isNewMessage) {
              showScrollToBottom.value = true;
            } else if (!isUserScrolledUp.value) {
              scrollController.jumpTo(
                scrollController.position.maxScrollExtent,
              );
            }
          }

          previousMessageCount.value = currentMessageCount;
        }
      });
      return null;
    }, [data.length, toend.value]);

    void scrollToBottom() {
      if (scrollController.hasClients) {
        scrollController.animateTo(
          scrollController.position.maxScrollExtent,
          duration: const Duration(milliseconds: 300),
          curve: Curves.easeOut,
        );
        showScrollToBottom.value = false;
        isUserScrolledUp.value = false;
      }
    }

    return Stack(
      children: [
        RefreshIndicator.noSpinner(
          onRefresh: () async {
            await ref.read(messageChatProvider.notifier).next();
          },
          child: ListView.separated(
            controller: scrollController,
            padding: const EdgeInsets.all(1),
            itemCount: data.length,
            separatorBuilder: (_, __) => const SizedBox(height: 8),
            itemBuilder: (context, index) {
              return MessageCard(model: data[index]);
            },
          ),
        ),
        if (showScrollToBottom.value)
          Positioned(
            bottom: 16,
            right: 16,
            child: FloatingActionButton.small(
              onPressed: scrollToBottom,
              backgroundColor: context.theme.colors.primaryForeground,
              child: const Icon(Icons.keyboard_arrow_down, color: Colors.black),
            ),
          ),
      ],
    );
  }
}
