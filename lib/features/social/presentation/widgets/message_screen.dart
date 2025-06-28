import 'dart:developer';
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
import 'dart:async';

class MessageScreen extends StatefulHookConsumerWidget {
  final ValueNotifier<bool> toend;
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
    final messagesAsync = ref.watch(messageChatProvider);

    return messagesAsync.when(
      data:
          (messages) => Column(
            children: [
              Expanded(
                child: ChatScreen(messages: messages, toend: widget.toend),
              ),
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
  final ValueNotifier<bool> toend;

  const ChatScreen({super.key, required this.messages, required this.toend});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final scrollController = useScrollController();
    final showScrollToBottom = useState(false);
    final previousMessageCount = useRef(0);
    final isUserScrolledUp = useRef(false);
    final isLoadingMore = useState(false);
    final messageChatNotifier = ref.read(messageChatProvider.notifier);
    final scrollDebouncer = useRef<Timer?>(null);

    final messageKeys = useMemoized(
      () => Map<String, GlobalKey>.fromEntries(
        messages.map((msg) => MapEntry(msg.id, GlobalKey())),
      ),
      [messages.length],
    );

    useEffect(() {
      void onScroll() {
        if (!scrollController.hasClients) return;

        final position = scrollController.position;
        final isAtBottom = position.pixels >= position.maxScrollExtent - 100;
        final isAtTop = position.pixels <= 100;

        if (!isAtBottom && position.pixels > 0) {
          isUserScrolledUp.value = true;
        } else if (isAtBottom) {
          isUserScrolledUp.value = false;
          showScrollToBottom.value = false;
        }

        if (isAtTop &&
            !isLoadingMore.value &&
            messageChatNotifier.hasMoreMessages) {
          scrollDebouncer.value?.cancel();
          scrollDebouncer.value = Timer(const Duration(milliseconds: 300), () {
            _loadMoreMessages(
              isLoadingMore,
              messageChatNotifier,
              scrollController,
            );
          });
        }

        showScrollToBottom.value =
            isUserScrolledUp.value &&
            position.pixels > position.maxScrollExtent * 0.1;
      }

      scrollController.addListener(onScroll);
      return () {
        scrollController.removeListener(onScroll);
        scrollDebouncer.value?.cancel();
      };
    }, [scrollController]);

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!scrollController.hasClients) return;

        final currentMessageCount = messages.length;
        final hasNewMessages = currentMessageCount > previousMessageCount.value;

        if (hasNewMessages) {
          final shouldAutoScroll =
              !isUserScrolledUp.value ||
              (toend.value &&
                  currentMessageCount != previousMessageCount.value);

          if (shouldAutoScroll) {
            _scrollToBottom(scrollController, animated: true);
            showScrollToBottom.value = false;
          } else if (isUserScrolledUp.value) {
            showScrollToBottom.value = true;
          }
        }

        previousMessageCount.value = currentMessageCount;
      });
      return null;
    }, [messages.length, toend.value]);

    useEffect(() {
      if (messages.isNotEmpty) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          _scrollToBottom(scrollController, animated: false);
        });
      }
      return null;
    }, []);

    return Stack(
      children: [
        CustomScrollView(
          controller: scrollController,
          slivers: [
            if (messageChatNotifier.hasMoreMessages)
              SliverToBoxAdapter(
                child: Container(
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
                ),
              ),

            SliverList(
              delegate: SliverChildBuilderDelegate((context, index) {
                final message = messages[index];
                final messageKey = messageKeys[message.id] ?? GlobalKey();

                return Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8.0,
                    vertical: 4.0,
                  ),
                  child: MessageCard(
                    key: ValueKey('message_${message.id}'),
                    model: message,
                    scrollController: scrollController,
                  ),
                );
              }, childCount: messages.length),
            ),

            const SliverToBoxAdapter(child: SizedBox(height: 16)),
          ],
        ),

        AnimatedPositioned(
          duration: const Duration(milliseconds: 200),
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
                    isUserScrolledUp,
                  ),
              backgroundColor: context.theme.colors.primaryForeground,
              elevation: 0,
              child: const Icon(Icons.keyboard_arrow_down, color: Colors.black),
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
    if (isLoadingMore.value) return;

    try {
      isLoadingMore.value = true;
      final currentScrollOffset = scrollController.offset;

      final hasMore = await messageChatNotifier.loadMoreMessages();

      if (hasMore && scrollController.hasClients) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (scrollController.hasClients) {
            final newMaxExtent = scrollController.position.maxScrollExtent;
            final targetOffset =
                newMaxExtent -
                (scrollController.position.maxScrollExtent -
                    currentScrollOffset);
            scrollController.jumpTo(targetOffset.clamp(0.0, newMaxExtent));
          }
        });
      }
    } catch (e) {
      log('Error loading more messages: $e');
    } finally {
      isLoadingMore.value = false;
    }
  }

  void _scrollToBottom(ScrollController controller, {bool animated = true}) {
    if (!controller.hasClients) return;

    final targetOffset = controller.position.maxScrollExtent;

    if (animated) {
      controller.animateTo(
        targetOffset,
        duration: const Duration(milliseconds: 300),
        curve: Curves.easeOut,
      );
    } else {
      controller.jumpTo(targetOffset);
    }
  }

  void _scrollToBottomAndReset(
    ScrollController controller,
    ValueNotifier<bool> showScrollToBottom,
    ObjectRef<bool> isUserScrolledUp,
  ) {
    _scrollToBottom(controller, animated: true);
    showScrollToBottom.value = false;
    isUserScrolledUp.value = false;
  }
}
