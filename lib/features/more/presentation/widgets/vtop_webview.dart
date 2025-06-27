import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
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

        final parts = cookieString.split('=');
        if (parts.length < 2) return;

        final name = parts[0];
        final value = parts[1];
        cookieName.value = name;
        cookieValue.value = value;
        if (Platform.isAndroid) {
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
      return const Center(
        child: CircularProgressIndicator(color: Colors.black),
      );
    }

    return SizedBox(
      width: MediaQuery.of(context).size.width,
      height: MediaQuery.of(context).size.height * 0.8,
      child: Column(
        children: <Widget>[
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
                }
                return NavigationActionPolicy.ALLOW;
              },
              onLoadStart: (controller, url) {},
              onLoadStop: (controller, url) {},
            ),
          ),
        ],
      ),
    );
  }
}
