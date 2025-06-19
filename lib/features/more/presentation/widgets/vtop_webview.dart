import 'dart:developer';
import 'dart:io';
import 'package:background_downloader/background_downloader.dart';
import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
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

Future<void> downloadFile(String url, String cookie) async {
  Directory? downloadsDir;
  final dio = Dio();
  final response = await dio.head(
    url,
    options: Options(
      responseType: ResponseType.stream,
      followRedirects: true,
      headers: {"Cookie": cookie},
    ),
  );

  final content = response.headers.value(HttpHeaders.contentDisposition);

  if (Platform.isAndroid) {
    if (await Permission.notification.request().isDenied) {
      log("notification permission denied");
    }
    downloadsDir = await getApplicationDocumentsDirectory();
  } else if (Platform.isIOS) {
    downloadsDir = await getApplicationDocumentsDirectory();
  }

  if (downloadsDir == null || !await downloadsDir.exists()) {
    log("Downloads directory not found");
    return;
  }
  FileDownloader().configureNotification(
    running: TaskNotification('Downloading', 'file: {filename}'),
    complete: TaskNotification('Download finished', 'file: {filename}'),
    tapOpensFile: true,
    progressBar: true,
  );

  String extractFilename(String? header) {
    if (header == null || !header.contains('filename=')) {
      return 'downloaded_file.zip';
    }
    final regex = RegExp(r'filename="?([^"]+)"?');
    final match = regex.firstMatch(header);
    if (match != null) {
      return match.group(1)!.trim();
    }
    return 'downloaded_file.zip';
  }

  final filename = extractFilename(content);

  final task = DownloadTask(
    url: url,
    headers: {"Cookie": cookie},
    retries: 5,
    directory: downloadsDir.path,
    filename: filename,
    allowPause: true,
  );
  final result = await FileDownloader().download(task);
  await FileDownloader().moveToSharedStorage(task, SharedStorage.downloads);
  log("Download started with taskId: $result");
}
