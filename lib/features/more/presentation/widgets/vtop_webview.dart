import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';

import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';
import 'package:webview_flutter/webview_flutter.dart';

var cookieManagerAndroid = WebViewCookieManager();

class VtopWebview extends HookConsumerWidget {
  const VtopWebview({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final envState = useState(true);
    final loading = useState(false);
    final cookieName = useState<String?>(null);
    final cookieValue = useState<String?>(null);
    final webController = useState<InAppWebViewController?>(null);

    useEffect(() {
      Future(() async {
        final cookieManager = CookieManager.instance();
        final expiresDate =
            DateTime.now()
                .add(const Duration(minutes: 30))
                .millisecondsSinceEpoch;
        final url = WebUri("https://vtop.vitap.ac.in");

        final client = await ref.read(vClientProvider.future);
        final raw = await fetchCookies(client: client);
        final cookieString = String.fromCharCodes(raw);

        final parts = cookieString.split(";").first.trim().split('=');
        if (parts.length < 2) return;

        final name = parts[0].trim();
        final value = parts[1].trim();
        cookieName.value = name;
        cookieValue.value = value;

        if (Platform.isAndroid) {
          await cookieManagerAndroid.clearCookies();
          await cookieManagerAndroid.setCookie(
            WebViewCookie(
              name: name,
              value: value,
              domain: '.vitap.ac.in',
              path: '/',
            ),
          );
        } else {
          await cookieManager.setCookie(
            url: url,
            name: name,
            value: value,
            expiresDate: expiresDate,
            isSecure: true,
            domain: ".vitap.ac.in",
            path: '/',
          );
        }

        envState.value = false;
      });

      return null;
    }, []);

    if (envState.value) {
      return Container(
        color: MoreColors.tableBackground,
        child: Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              SizedBox(
                width: 50,
                height: 50,
                child: CircularProgressIndicator(
                  color: MoreColors.infoBorder,
                  strokeWidth: 3,
                ),
              ),
              const SizedBox(height: 16),
              Text(
                "Setting up VTOP...",
                style: TextStyle(
                  fontSize: 16,
                  color: MoreColors.secondaryText,
                  fontWeight: FontWeight.w500,
                ),
              ),
            ],
          ),
        ),
      );
    }

    return Container(
      width: MediaQuery.of(context).size.width,
      height: MediaQuery.of(context).size.height * 0.8,
      decoration: BoxDecoration(
        color: MoreColors.tableBackground,
        boxShadow: [
          BoxShadow(
            color: MoreColors.cardShadow,
            blurRadius: 8,
            offset: const Offset(0, 2),
          ),
        ],
      ),
      child: Column(
        children: [
          Expanded(
            child: InAppWebView(
              initialSettings: InAppWebViewSettings(isInspectable: kDebugMode),
              initialUrlRequest: URLRequest(
                url: WebUri("https://vtop.vitap.ac.in/vtop/content?"),
              ),
              onWebViewCreated: (controller) {
                webController.value = controller;
              },
              
            
              shouldOverrideUrlLoading: (controller, navigationAction) async {
                final uri = navigationAction.request.url;

                if (uri.toString().toLowerCase().contains("download")) {
                  String cookie = "${cookieName.value}=${cookieValue.value};";
                  downloadFile(uri.toString(), cookie);
                  return NavigationActionPolicy.CANCEL;
                } else if (uri.toString().toLowerCase().startsWith(
                  "https://vtop.vitap.ac.in",
                )) {
                  return NavigationActionPolicy.ALLOW;
                }

                return NavigationActionPolicy.CANCEL;
              },
              onReceivedServerTrustAuthRequest: ( _,_)async{  
                return ServerTrustAuthResponse(action: ServerTrustAuthResponseAction.PROCEED);
                },
              onUpdateVisitedHistory: (controller, url, androidIsReload) {
                final u = url.toString().toLowerCase();
                if (!u.startsWith("https://vtop.vitap.ac.in")) {
                  controller.stopLoading();
                  controller.goBack();
                  dispToast(
                    context,
                    "Open in Chrome",
                    "Please continue in Chrome.",
                  );
                }
              },
              onLoadStart: (controller, url) {
                loading.value = true;
              },
              onLoadStop: (controller, url) {
                loading.value = false;
              },
            ),
          ),
          if (loading.value)
            Container(
              padding: const EdgeInsets.all(8),
              color: MoreColors.infoBackground,
              child: Row(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(
                      strokeWidth: 2,
                      color: MoreColors.infoText,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    "Loading...",
                    style: TextStyle(
                      color: MoreColors.secondaryText,
                      fontSize: 12,
                    ),
                  ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}
