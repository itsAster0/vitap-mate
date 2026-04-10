import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/features/more/presentation/widgets/more_color.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_cookie_service.dart';
import 'package:vitapmate/features/more/presentation/widgets/vtop_webview/vtop_webview_scripts.dart';

class VtopWebviewBody extends StatelessWidget {
  const VtopWebviewBody({
    required this.initialUrl,
    required this.isCompactMode,
    required this.isDarkMode,
    required this.loading,
    required this.onLoadingChanged,
    required this.onWebViewCreated,
    required this.cookie,
    super.key,
  });

  final WebUri initialUrl;
  final bool isCompactMode;
  final bool isDarkMode;
  final bool loading;
  final ValueChanged<bool> onLoadingChanged;
  final ValueChanged<InAppWebViewController> onWebViewCreated;
  final VtopWebviewCookie? cookie;

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      bottom: true,
      top: false,
      left: false,
      right: false,
      child: Container(
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
        child: Stack(
          children: [
            InAppWebView(
              initialSettings: InAppWebViewSettings(
                isInspectable: kDebugMode,
                useOnDownloadStart: true,
              ),
              initialUrlRequest: URLRequest(url: initialUrl),
              onWebViewCreated: onWebViewCreated,
              onDownloadStartRequest: (controller, request) async {
                final cookieHeader = cookie?.header ?? '';
                await downloadFile(
                  request.url.toString(),
                  cookieHeader,
                  contentDisposition: request.contentDisposition,
                  mimeType: request.mimeType,
                  suggestedFilename: request.suggestedFilename,
                );
              },
              shouldOverrideUrlLoading: (controller, navigationAction) async {
                final uri = navigationAction.request.url;
                if (uri == null) return NavigationActionPolicy.CANCEL;

                final url = uri.toString();
                final lowerUrl = url.toLowerCase();
                if (lowerUrl.contains('download')) {
                  final cookieHeader = cookie?.header ?? '';
                  await downloadFile(url, cookieHeader);
                  return NavigationActionPolicy.CANCEL;
                }

                if (lowerUrl.startsWith('https://vtop.vitap.ac.in')) {
                  return NavigationActionPolicy.ALLOW;
                }

                return NavigationActionPolicy.CANCEL;
              },
              onReceivedServerTrustAuthRequest: (_, _) async {
                return ServerTrustAuthResponse(
                  action: ServerTrustAuthResponseAction.PROCEED,
                );
              },
              onUpdateVisitedHistory: (controller, url, androidIsReload) {
                final visitedUrl = url.toString().toLowerCase();
                if (!visitedUrl.startsWith('https://vtop.vitap.ac.in')) {
                  controller.stopLoading();
                  controller.goBack();
                  dispToast(
                    context,
                    'Open in Chrome',
                    'Please continue in Chrome.',
                  );
                }
              },
              onLoadStart: (controller, url) async {
                onLoadingChanged(true);
                if (isDarkMode) {
                  await controller.setVtopDarkMode(true);
                }
              },
              onLoadStop: (controller, url) async {
                onLoadingChanged(false);
                if (isCompactMode) {
                  await controller.setVtopCompactSpacing(padding: 1);
                }
                if (isDarkMode) {
                  await controller.setVtopDarkMode(true);
                }
              },
            ),
            if (loading)
              const Positioned(left: 0, right: 0, top: 0, child: FProgress()),
          ],
        ),
      ),
    );
  }
}
