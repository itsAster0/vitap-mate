import 'dart:async';
import 'package:app_links/app_links.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:pocketbase/pocketbase.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:uuid/uuid.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/social/presentation/pages/message_chat_page.dart';
import 'package:vitapmate/features/social/presentation/providers/pocketbase.dart';
import 'dart:io' show Platform;

class SocialPage extends HookConsumerWidget {
  const SocialPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    var pbasync = ref.watch(pbProvider);

    return pbasync.when(
      data: (pb) {
        if (pb.authStore.isValid) {
          return MessageChatPage();
        }
        return ShowLogin(pb: pb);
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
            child: CircularProgressIndicator(
              color: context.theme.colors.primary,
            ),
          ),
        );
      },
    );
  }
}

Uri buildAuthUrl(
  AuthMethodProvider provider,
  Uri redirectURL,
  String state, {
  List<String> scopes = const [],
}) {
  final baseUri = Uri.parse(provider.authURL);
  final queryParameters = <String, String>{
    ...baseUri.queryParameters,
    "redirect_uri": redirectURL.toString(),
    "state": state,
  };
  if (scopes.isNotEmpty) {
    queryParameters["scope"] = scopes.join(" ");
  }
  return baseUri.replace(queryParameters: queryParameters);
}

class ShowLogin extends HookConsumerWidget {
  final PocketBase pb;
  const ShowLogin({super.key, required this.pb});
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    var isloading = useState(false);
    final redirectURL = Uri.parse("https://api.va.kryxen.dev/social");
    final collectionsName = "users";
    var googleProvider = useRef<AuthMethodProvider?>(null);
    final appLinks = useMemoized(() => AppLinks());
    final stateH = useMemoized(() => Uuid().v4());

    useEffect(() {
      final sub = appLinks.uriLinkStream.listen((uri) async {
        final data = uri.queryParameters;
        final code = data["code"] ?? "";
        try {
          await pb
              .collection(collectionsName)
              .authWithOAuth2Code(
                googleProvider.value!.name,
                code,
                googleProvider.value!.codeVerifier,
                redirectURL.toString(),
              );
        } on ClientException catch (e) {
          if (context.mounted) {
            dispToast(context, "Error", e.response["message"]);
          }
          if (e.response["message"].toString().toLowerCase().contains("vit")) {
            ref.invalidate(pbProvider);
          }
        } finally {
          isloading.value = false;
          var _ = ref.refresh(pbProvider);
        }
      });

      return () {
        sub.cancel();
      };
    }, []);

    void handleClick(PocketBase pb) async {
      if (isloading.value) return;
      Timer timeoutTimer = Timer(Duration(seconds: 15), () {
        isloading.value = false;
      });
      isloading.value = true;
      if (!Platform.isAndroid) {
        try {
          await pb.collection(collectionsName).authWithOAuth2('google', (
            url,
          ) async {
            await launchUrl(url);
          });
          var _ = ref.refresh(pbProvider);
          await pbSetNtotification();
        } on ClientException catch (e) {
          if (context.mounted) {
            dispToast(context, "Error", e.response["message"]);
          }
          if (e.response["message"].toString().toLowerCase().contains("vit")) {
            ref.invalidate(pbProvider);
          }
        } catch (e) {
          //ref.invalidate(pbProvider);
        } finally {
          var _ = ref.refresh(pbProvider);
          timeoutTimer.cancel();
          isloading.value = false;
        }
      } else {
        final providerAuth =
            await pb.collection(collectionsName).listAuthMethods();
        googleProvider.value = providerAuth.oauth2.providers.first;
        final url = buildAuthUrl(googleProvider.value!, redirectURL, stateH);
        launchUrl(url);
      }
    }

    return !isloading.value
        ? Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            spacing: 10,
            children: [
              FButton(
                onPress: () => handleClick(pb),

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
            child: CircularProgressIndicator(
              color: context.theme.colors.primary,
            ),
          ),
        );
  }
}
