// Reading a Gmail API message (`format=full`): its headers and its text.

import 'dart:convert';

/// The value of the [message]'s first [name] header, trimmed.
String? gmailHeader(Map<String, dynamic> message, String name) {
  if (message['payload'] case <String, dynamic>{
    'headers': final List<dynamic> headers,
  }) {
    for (final item in headers) {
      if (item is Map<String, dynamic> &&
          '${item['name']}'.toLowerCase() == name.toLowerCase()) {
        return '${item['value']}'.trim();
      }
    }
  }
  return null;
}

/// True when VTOP's sender sent [message].
bool isFromVtopOtpSender(Map<String, dynamic> message) =>
    gmailHeader(
      message,
      'from',
    )?.toLowerCase().contains('noreply.sdc@vitap.ac.in') ??
    false;

/// The snippet and every body part, decoded (base64url, quoted-printable).
String gmailMessageText(Map<String, dynamic> message) {
  final buffer = StringBuffer();
  final snippet = message['snippet'];
  if (snippet is String && snippet.isNotEmpty) buffer.writeln(snippet);
  void append(Object? payload) {
    if (payload is! Map<String, dynamic>) return;
    if (payload['body'] case {'data': final String data} when data.isNotEmpty) {
      final text = _decodeBase64Url(data);
      if (text.isNotEmpty) buffer.writeln(text);
    }
    final parts = payload['parts'];
    if (parts is List<dynamic>) parts.forEach(append);
  }

  append(message['payload']);
  return _decodeQuotedPrintable(buffer.toString());
}

String _decodeBase64Url(String value) {
  try {
    return utf8.decode(
      base64Url.decode(base64Url.normalize(value)),
      allowMalformed: true,
    );
  } catch (_) {
    return '';
  }
}

String _decodeQuotedPrintable(String input) => input
    .replaceAll(RegExp(r'=\r?\n'), '')
    .replaceAllMapped(RegExp(r'=([0-9A-Fa-f]{2})'), (match) {
      final code = int.tryParse(match.group(1)!, radix: 16);
      return code == null ? '' : String.fromCharCode(code);
    });
