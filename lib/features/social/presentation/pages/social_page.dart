import 'dart:developer';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/social/presentation/pages/message_chat_page.dart';
import 'package:vitapmate/features/social/presentation/providers/pocketbase.dart';

class SocialPage extends HookConsumerWidget {
  const SocialPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    var isloading = useState(false);
    var pbasync = ref.watch(pbProvider);
    return pbasync.when(
      data: (pb) {
        if (pb.authStore.isValid) {
          return MessageChatPage();
        }
        return !isloading.value
            ? Center(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                spacing: 10,
                children: [
                  FButton(
                    onPress: () async {
                      if (isloading.value) return;
                      isloading.value = true;
                      try {
                        await pb.collection('users').authWithOAuth2('google', (
                          url,
                        ) async {
                          await launchUrl(url);
                        });
                        await Future.delayed(Duration(seconds: 2));
                        isloading.value = false;
                        ref.invalidate(pbProvider);
                        log("done login via mail");
                      } catch (e) {
                        isloading.value = false;
                      } finally {
                        isloading.value = false;
                      }
                    },
                    child: Row(
                      spacing: 10,
                      children: [
                        FAvatar(
                          image: NetworkImage(
                            "https://yt3.googleusercontent.com/2eI1TjX447QZFDe6R32K0V2mjbVMKT5mIfQR-wK5bAsxttS_7qzUDS1ojoSKeSP0NuWd6sl7qQ=s160-c-k-c0x00ffffff-no-rj",
                          ),
                          fallback: Text("Lo"),
                        ),
                        Text(
                          "Login",
                          style: TextStyle(fontWeight: FontWeight.bold),
                        ),
                      ],
                    ),
                  ),
                  Container(
                    decoration: BoxDecoration(
                      color: context.theme.colors.primaryForeground,
                    ),
                    child: Padding(
                      padding: const EdgeInsets.all(8.0),
                      child: Text(
                        "👋 Just a heads-up! You’ll need to use your college email to log in — other emails won’t get you in",
                        style: const TextStyle(
                          fontSize: 14,
                          color: Colors.blueGrey,
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            )
            : Center(
              child: SizedBox(
                width: 30,
                height: 30,
                child: CircularProgressIndicator(color: Colors.black),
              ),
            );
      },
      error: (e, se) {
        disCommonToast(context, e);
        String msg = commonErrorMessage(e);
        return Center(child: Text(msg));
      },
      loading: () {
        return Center(
          child: SizedBox(
            width: 50,
            height: 50,
            child: CircularProgressIndicator(color: Colors.black),
          ),
        );
      },
    );
  }
}
