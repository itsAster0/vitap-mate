import 'dart:developer';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:scroll_to_index/scroll_to_index.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/social/presentation/providers/message_chat.dart';
import 'package:vitapmate/features/social/presentation/providers/pocketbase.dart';
import 'package:vitapmate/features/social/presentation/widgets/message_bar.dart';
import 'package:vitapmate/features/social/presentation/widgets/message_card.dart';
import 'dart:async';

class MessageScreen extends StatefulHookConsumerWidget {
  const MessageScreen({super.key});

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
    final messagesAsync = ref.watch(messageChatProvider);

    return messagesAsync.when(
      data:
          (messages) => Column(
            children: [
              Expanded(child: ChatScreen(messages: messages)),
              const MessageBar(),
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
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(Icons.error_outline, size: 64, color: Colors.grey[400]),
                  const SizedBox(height: 16),
                  Text(
                    msg,
                    textAlign: TextAlign.center,
                    style: TextStyle(fontSize: 16, color: Colors.grey[600]),
                  ),
                  const SizedBox(height: 24),
                  FButton(
                    onPress: () => ref.refresh(pbProvider.future),
                    child: const Text('Retry'),
                  ),
                ],
              ),
            ),
          ),
        );
      },
      loading:
          () => const Center(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                SizedBox(
                  width: 50,
                  height: 50,
                  child: CircularProgressIndicator(color: Colors.black),
                ),
                SizedBox(height: 16),
                Text(
                  'Loading messages...',
                  style: TextStyle(color: Colors.grey),
                ),
              ],
            ),
          ),
    );
  }
}

class ChatScreen extends HookConsumerWidget {
  final List messages;

  const ChatScreen({super.key, required this.messages});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final scrollController = useMemoized(
      () => AutoScrollController(
        axis: Axis.vertical,
        suggestedRowHeight: 120,
        viewportBoundaryGetter:
            () => Rect.fromLTRB(0, 0, 0, MediaQuery.of(context).padding.bottom),
      ),
    );
    final isAtBottom = useState(true);
    final showScrollToBottom = useState(false);
    final isLoadingMore = useState(false);
    final messageChatNotifier = ref.read(messageChatProvider.notifier);
    final scrollTimer = useRef<Timer?>(null);
    final hasNewMessage = useRef(false);

    useEffect(() {
      void onScroll() {
        if (!scrollController.hasClients) return;

        final position = scrollController.position;
        final isNearBottom = position.pixels <= 100;
        final isNearTop = position.pixels >= position.maxScrollExtent - 100;

        scrollTimer.value?.cancel();
        scrollTimer.value = Timer(const Duration(milliseconds: 100), () {
          if (!context.mounted) return;

          isAtBottom.value = isNearBottom;
          showScrollToBottom.value = !isNearBottom && position.pixels > 200;

          if (isNearTop &&
              !isLoadingMore.value &&
              messageChatNotifier.hasMoreMessages) {
            _loadMoreMessages(
              isLoadingMore,
              messageChatNotifier,
              scrollController,
            );
          }
        });
      }

      scrollController.addListener(onScroll);
      return () {
        scrollController.removeListener(onScroll);
        scrollTimer.value?.cancel();
      };
    }, [scrollController]);

    if (messages.isEmpty) {
      return const Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(Icons.chat_bubble_outline, size: 64, color: Colors.grey),
            SizedBox(height: 16),
            Text(
              'No messages yet',
              style: TextStyle(fontSize: 18, color: Colors.grey),
            ),
            SizedBox(height: 8),
            Text(
              'Start a conversation!',
              style: TextStyle(fontSize: 14, color: Colors.grey),
            ),
          ],
        ),
      );
    }

    return Stack(
      children: [
        ListView.separated(
          controller: scrollController,
          reverse: true,
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 16),
          itemCount:
              messages.length + (messageChatNotifier.hasMoreMessages ? 1 : 0),
          separatorBuilder: (context, index) => const SizedBox(height: 8),
          itemBuilder: (context, index) {
            if (index == messages.length) {
              return Container(
                padding: const EdgeInsets.all(16),
                child: Center(
                  child:
                      isLoadingMore.value
                          ? const SizedBox(
                            width: 20,
                            height: 20,
                            child: CircularProgressIndicator(
                              strokeWidth: 2,
                              color: Colors.black,
                            ),
                          )
                          : FButton(
                            style: FButtonStyle.secondary,
                            onPress:
                                () => _loadMoreMessages(
                                  isLoadingMore,
                                  messageChatNotifier,
                                  scrollController,
                                ),
                            child: const Text('Load more messages'),
                          ),
                ),
              );
            }

            final message = messages[index];
            return AutoScrollTag(
              key: ValueKey(index),
              controller: scrollController,
              index: index,
              child: MessageCard(
                model: message,
                scrollController: scrollController,
              ),
            );
          },
        ),

        AnimatedPositioned(
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeInOut,
          bottom: showScrollToBottom.value ? 16 : -60,
          right: 16,
          child: Material(
            elevation: 4,
            borderRadius: BorderRadius.circular(28),
            child: FloatingActionButton.small(
              onPressed:
                  () => _scrollToBottomAndReset(
                    scrollController,
                    showScrollToBottom,
                    isAtBottom,
                    hasNewMessage,
                  ),
              backgroundColor: context.theme.colors.primaryForeground,
              elevation: 0,
              child: Stack(
                alignment: Alignment.center,
                children: [
                  const Icon(Icons.keyboard_arrow_down, color: Colors.black),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }

  Future<void> _loadMoreMessages(
    ValueNotifier<bool> isLoadingMore,
    dynamic messageChatNotifier,
    ScrollController scrollController,
  ) async {
    if (isLoadingMore.value || !scrollController.hasClients) return;

    try {
      isLoadingMore.value = true;

      final position = scrollController.position;
      final distanceFromBottom = position.maxScrollExtent - position.pixels;

      final hasMore = await messageChatNotifier.loadMoreMessages();

      if (hasMore && scrollController.hasClients) {
        if (scrollController.hasClients) {
          final newPosition =
              scrollController.position.maxScrollExtent - distanceFromBottom;
          scrollController.jumpTo(
            newPosition.clamp(
              scrollController.position.minScrollExtent,
              scrollController.position.maxScrollExtent,
            ),
          );
        }
      }
    } catch (e) {
      log('Error loading more messages: $e');
    } finally {
      isLoadingMore.value = false;
    }
  }

  void _scrollToBottom(ScrollController controller, {bool animated = true}) {
    if (!controller.hasClients) return;

    if (animated) {
      controller.animateTo(
        0,
        duration: const Duration(milliseconds: 300),
        curve: Curves.easeOut,
      );
    } else {
      controller.jumpTo(0);
    }
  }

  void _scrollToBottomAndReset(
    ScrollController controller,
    ValueNotifier<bool> showScrollToBottom,
    ValueNotifier<bool> isAtBottom,
    ObjectRef<bool> hasNewMessage,
  ) {
    _scrollToBottom(controller, animated: true);
    showScrollToBottom.value = false;
    isAtBottom.value = true;
    hasNewMessage.value = false;
  }
}
