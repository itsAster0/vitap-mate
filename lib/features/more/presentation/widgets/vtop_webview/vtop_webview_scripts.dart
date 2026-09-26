import 'dart:convert';
import 'package:flutter_inappwebview/flutter_inappwebview.dart';

const _darkModeScript = '''
(function() {
  if (document.getElementById('dark-mode-style')) return;

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
  (document.head || document.documentElement).appendChild(style);
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
})();
''';

const _resetDesktopModeScript = '''
(function () {
  document.querySelectorAll('meta[name=viewport]').forEach(viewport => {
    viewport.setAttribute('content', 'width=device-width, initial-scale=1');
  });
})();
''';

const _desktopModeScript = '''
(function() {
  var viewportWidth = 1024;
  var viewports = document.querySelectorAll("meta[name=viewport]");

  if (viewports.length) {
    viewports.forEach(viewport => viewport.setAttribute(
      'content', 'width=' + viewportWidth + ', user-scalable=yes'));
  } else {
    var meta = document.createElement('meta');
    meta.name = "viewport";
    meta.content = 'width=' + viewportWidth + ', user-scalable=yes';
    (document.head || document.documentElement).appendChild(meta);
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

  Future<dynamic> clickVtopMenuLink(String url) {
    return evaluateJavascript(source: _clickMenuLinkScript(url));
  }
}

String _compactSpacingScript(int padding) {
  return '''
(function() {
  if (document.getElementById('custom-spacing-style')) return;
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

    (document.head || document.documentElement).appendChild(style);
})();
''';
}

String _clickMenuLinkScript(String url) =>
    """
(function() {
  const target = ${jsonEncode(url)};
  const link = Array.from(document.querySelectorAll('a[data-url]'))
    .find(link => link.getAttribute('data-url') === target);
  if (!link) return false;
  link.click();
  return true;
})();
""";

String vtopPreferencesScript({
  required bool dark,
  required bool compact,
  required bool desktop,
}) =>
    """
(function() {
  if (location.origin !== 'https://vtop.vitap.ac.in') return;
  function apply() {
    if (!document.documentElement) return;
    ${dark ? _darkModeScript : _removeDarkModeScript}
    ${compact ? _compactSpacingScript(1) : _resetSpacingScript}
    ${desktop ? _desktopModeScript : _resetDesktopModeScript}
  }
  apply();
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', apply, {once: true});
  }
})();
""";

/// Observes activity only. Request arguments, results and errors are preserved.
const vtopActivityScript = r'''
(function() {
  if (location.origin !== 'https://vtop.vitap.ac.in' || window.__mateActivity) return;
  const id = String(performance.timeOrigin);
  window.__mateActivity = id;
  document.addEventListener('click', function(event) {
    const target = event.target instanceof Element
      ? event.target : event.target?.parentElement;
    const link = target?.closest('a[data-url]');
    // VTOP's own Home icon loads the dashboard in place; report it as ''.
    const home = !link && target?.closest('[onclick*="home()"]');
    if (link || home) {
      const menu = link ? link.getAttribute('data-url') || '' : '';
      const title = link ? (link.textContent || '').trim().replace(/\s+/g, ' ') : '';
      if (!/\/download/i.test(menu)) {
        window.__mateCurrentMenu = menu;
        window.__mateCurrentTitle = title;
        try { window.flutter_inappwebview?.callHandler('vtopMenuChanged', menu, title); }
        catch (_) {}
      }
    }
  }, true);
  let active = 0;
  function notify(type, extra) {
    try {
      const bridge = window.flutter_inappwebview;
      if (bridge) bridge.callHandler('vtopActivity',
        Object.assign({type: type, id: id, active: active}, extra || {}));
    } catch (_) {}
  }
  function sameOrigin(input) {
    try {
      return new URL(input instanceof Request ? input.url : input,
        location.href).origin === location.origin;
    } catch (_) { return false; }
  }
  function begin() {
    active++;
    notify('activity');
    let ended = false;
    return function() {
      if (ended) return;
      ended = true;
      active = Math.max(0, active - 1);
      notify('activity');
    };
  }
  const originalFetch = window.fetch;
  if (originalFetch) window.fetch = function() {
    const done = sameOrigin(arguments[0]) ? begin() : function() {};
    try {
      const result = originalFetch.apply(this, arguments);
      result.then(done, done);
      return result;
    } catch (error) { done(); throw error; }
  };
  const open = XMLHttpRequest.prototype.open;
  const send = XMLHttpRequest.prototype.send;
  const tracked = new WeakMap();
  XMLHttpRequest.prototype.open = function(method, url) {
    const result = open.apply(this, arguments);
    tracked.set(this, sameOrigin(url));
    return result;
  };
  XMLHttpRequest.prototype.send = function() {
    if (!tracked.get(this)) return send.apply(this, arguments);
    const done = begin();
    this.addEventListener('loadend', done, {once: true});
    try { return send.apply(this, arguments); }
    catch (error) { this.removeEventListener('loadend', done); done(); throw error; }
  };
  function ready() { notify('ready'); }
  document.addEventListener('DOMContentLoaded', ready, {once: true});
  window.addEventListener('flutterInAppWebViewPlatformReady', ready, {once: true});
  if (document.readyState !== 'loading') ready();
  window.addEventListener('load', function() {
    const resources = performance.getEntriesByType('resource')
      .filter(r => sameOrigin(r.name) &&
        ['script', 'css', 'img', 'link'].includes(r.initiatorType));
    notify('timing', {
      duration: Math.round(performance.now()),
      resources: resources.length,
      transferBytes: resources.reduce((sum, r) => sum + (r.transferSize || 0), 0),
      // Zero transfer is a cache candidate, not proof of a cache hit.
      zeroTransfer: resources.filter(r => r.transferSize === 0).length
    });
  }, {once: true});
})();
''';

String vtopWaitForMenuScript(String url, String requestId) =>
    """
(function() {
  if (window.__mateMenuObserver) window.__mateMenuObserver();
  const target = ${jsonEncode(url)};
  let finished = false;
  let timer;
  const observer = new MutationObserver(attempt);
  function finish(found) {
    if (finished) return;
    finished = true;
    observer.disconnect();
    clearTimeout(timer);
    window.__mateMenuObserver = null;
    window.flutter_inappwebview?.callHandler('vtopMenuResult',
      ${jsonEncode(requestId)}, found);
  }
  function attempt() {
    if (finished) return;
    const link = Array.from(document.querySelectorAll('a[data-url]'))
      .find(link => link.getAttribute('data-url') === target);
    if (link) { finish(true); link.click(); }
  }
  window.__mateMenuObserver = () => { finished = true; observer.disconnect(); clearTimeout(timer); };
  observer.observe(document.documentElement, {childList: true, subtree: true});
  timer = setTimeout(() => finish(false), 10000);
  attempt();
})();
""";

/// Every page in VTOP's sidebar, grouped by its sidebar section.
const vtopPagesScript = r'''
(function() {
  const sidebar = document.getElementById('expandedSideBar');
  if (!sidebar) return [];
  const seen = new Set();
  return Array.from(sidebar.querySelectorAll('a[data-url]')).flatMap(link => {
    const url = link.getAttribute('data-url') || '';
    const title = (link.textContent || '').trim().replace(/\s+/g, ' ');
    if (!url || !title || seen.has(url) || /\/download/i.test(url)) return [];
    seen.add(url);
    const section = link.closest('.accordion-item')
      ?.querySelector('.accordion-header')?.textContent?.trim()
      .replace(/\s+/g, ' ') || '';
    return [{url: url, title: title, section: section}];
  });
})();
''';

/// Closes the top-most VTOP overlay; true when something was closed.
const vtopCloseOverlayScript = r'''
(function() {
  const bs = window.bootstrap;
  const modals = Array.from(document.querySelectorAll('.modal.show'));
  if (modals.length) {
    const modal = modals[modals.length - 1];
    const instance = bs?.Modal?.getInstance(modal);
    if (instance) instance.hide();
    else modal.querySelector('[data-bs-dismiss="modal"]')?.click();
    return true;
  }
  const canvas = document.querySelector('.offcanvas.show');
  if (canvas) {
    const instance = bs?.Offcanvas?.getInstance(canvas);
    if (instance) instance.hide();
    else canvas.querySelector('[data-bs-dismiss="offcanvas"]')?.click();
    return true;
  }
  const dropdown = document.querySelector('.dropdown-menu.show');
  if (dropdown) {
    const toggle = dropdown.parentElement
      ?.querySelector('[data-bs-toggle="dropdown"]');
    const instance = toggle && bs?.Dropdown?.getInstance(toggle);
    if (instance) instance.hide();
    else dropdown.classList.remove('show');
    return true;
  }
  const panel = document.getElementById('sidePanel');
  if (panel && !panel.classList.contains('d-none')) {
    panel.classList.add('d-none');
    return true;
  }
  return false;
})();
''';

/// VTOP's in-place Home, without reloading the document.
const vtopHomeScript = r'''
(function() {
  if (typeof window.home !== 'function') return false;
  window.__mateCurrentMenu = '';
  window.__mateCurrentTitle = '';
  window.home();
  return true;
})();
''';
