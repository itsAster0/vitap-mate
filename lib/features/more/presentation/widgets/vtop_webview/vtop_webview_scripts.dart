import 'package:flutter_inappwebview/flutter_inappwebview.dart';

const _darkModeScript = '''
(function() {
  const existingStyle = document.getElementById('dark-mode-style');
  if (existingStyle) existingStyle.remove();

  const style = document.createElement('style');
  style.id = 'dark-mode-style';
  style.textContent = `
    html {
      filter: invert(1) hue-rotate(180deg) !important;
      background-color: #000 !important;
    }
    img, video, [style*="background-image"] {
      filter: invert(1) hue-rotate(180deg) !important;
    }
  `;
  document.head.appendChild(style);
})();
''';

const _removeDarkModeScript = '''
(function() {
  const style = document.getElementById('dark-mode-style');
  if (style) style.remove();
})();
''';

const _resetSpacingScript = '''
(function() {
  const customStyle = document.getElementById('custom-spacing-style');
  if (customStyle) {
    customStyle.remove();
  }

  document.body.style.display = 'none';
  document.body.offsetHeight;
  document.body.style.display = 'block';
})();
''';

const _resetDesktopModeScript = '''
(function () {
  var viewport = document.querySelector('meta[name=viewport]');
  if (viewport) {
    viewport.setAttribute(
      'content',
      'width=device-width, initial-scale=1'
    );
  }
})();
''';

const _desktopModeScript = '''
(function() {
  var viewportWidth = 1024;
  var viewport = document.querySelector("meta[name=viewport]");

  if (viewport) {
    viewport.setAttribute('content', 'width=' + viewportWidth + ', user-scalable=yes');
  } else {
    var meta = document.createElement('meta');
    meta.name = "viewport";
    meta.content = 'width=' + viewportWidth + ', user-scalable=yes';
    document.head.appendChild(meta);
  }
})();
''';

extension VtopWebviewScripts on InAppWebViewController {
  Future<void> setVtopDarkMode(bool enabled) {
    return evaluateJavascript(
      source: enabled ? _darkModeScript : _removeDarkModeScript,
    );
  }

  Future<void> setVtopDesktopMode(bool enabled) {
    return evaluateJavascript(
      source: enabled ? _desktopModeScript : _resetDesktopModeScript,
    );
  }

  Future<void> setVtopCompactSpacing({int padding = 2}) {
    return evaluateJavascript(source: _compactSpacingScript(padding));
  }

  Future<void> resetVtopSpacing() {
    return evaluateJavascript(source: _resetSpacingScript);
  }

  Future<void> clickVtopMenuLink(String url) {
    return evaluateJavascript(source: _clickMenuLinkScript(url));
  }
}

String _compactSpacingScript(int padding) {
  return '''
(function() {
  setTimeout(function() {
    const style = document.createElement('style');
    style.id = 'custom-spacing-style';
    style.textContent = `
      * {
        margin: 0 !important;
        padding: 0 !important;
        box-sizing: border-box !important;
      }

      body {
        padding: ${padding}px !important;
        margin: 0 !important;
      }

      table {
        border-spacing: 0 !important;
        border-collapse: collapse !important;
        width: 100% !important;
        margin: ${padding}px !important;
      }

      td, th {
        padding: ${padding + 4}px !important;
      }

      .card {
        margin: ${padding}px !important;
        padding: ${padding}px !important;
        border-radius: 0 !important;
      }

      .container, .container-fluid {
        padding: ${padding}px !important;
        margin: ${padding}px !important;
        width: 100% !important;
        max-width: 100% !important;
      }

      div[style*="padding"], div[style*="margin"] {
        padding: ${padding}px !important;
        margin: ${padding}px !important;
      }

      button[data-bs-target="#expandedSideBar"] {
        padding: 10px 14px !important;
        border: none !important;
        border-radius: 6px !important;
        transition: all 0.3s ease !important;
        cursor: pointer !important;
      }
    `;

    const oldStyle = document.getElementById('custom-spacing-style');
    if (oldStyle) oldStyle.remove();

    document.head.appendChild(style);

    document.body.style.display = 'none';
    document.body.offsetHeight;
    document.body.style.display = 'block';
  }, 500);
})();
''';
}

String _clickMenuLinkScript(String url) {
  final escapedUrl = url.replaceAll('\\', '\\\\').replaceAll('"', r'\"');

  return '''
(function() {
  const link = document.querySelector('a[data-url="$escapedUrl"]');
  if (link) {
    link.click();
    return true;
  }
  return false;
})();
''';
}
