package vitapmate.features.vtop

import org.json.JSONObject

/**
 * Every script the VTOP web view runs, verbatim from the Flutter app's
 * `lib/features/vtop_browser/vtop_webview/vtop_webview_scripts.dart` (plus the inline checks in
 * `vtop_webview_body.dart`). [BRIDGE] maps flutter_inappwebview's `callHandler` onto this app's
 * `window.vitapmate`, so the scripts stay unchanged. None of them reads cookies or form fields.
 *
 * When each runs ([VtopBrowser]): [BRIDGE], [ACTIVITY] and [preferences] at page start, again when
 * the page first draws and when it finishes (Flutter: document start; android.webkit has no
 * document-start hook without androidx.webkit, and every script is safe to run twice);
 * [preferences] again on a menu toggle; [AUTHENTICATED] and the pending menu on the page's
 * `ready`; [CLOSE_OVERLAY], [HOME] and [waitForMenu] on Back / Home / reopening a page;
 * [PAGES] reads the sidebar.
 */
internal object VtopScripts {
    const val ORIGIN = "https://vtop.vitap.ac.in"

    /** flutter_inappwebview's JS API over this app's bridge (`call(handler, argsJson)`). */
    const val BRIDGE = """
(function() {
  if (location.origin !== 'https://vtop.vitap.ac.in' || !window.vitapmate || window.flutter_inappwebview) return;
  window.flutter_inappwebview = {
    callHandler: function(name) {
      try { window.vitapmate.call(name, JSON.stringify(Array.prototype.slice.call(arguments, 1))); } catch (_) {}
      return Promise.resolve();
    }
  };
})();
"""

    /**
     * Not in the Flutter app: reports whether anything in the page is scrolled down, so
     * pull-to-refresh starts only at the very top (the WebView can't see inner scrolled divs).
     */
    const val SCROLL = """
(function() {
  if (location.origin !== 'https://vtop.vitap.ac.in' || window.__mateScroll) return;
  window.__mateScroll = true;
  const scrolled = new Set();
  document.addEventListener('scroll', function(e) {
    const target = e.target === document ? document.scrollingElement : e.target;
    if (!target) return;
    const wasTop = scrolled.size === 0;
    if (target.scrollTop > 0) scrolled.add(target); else scrolled.delete(target);
    scrolled.forEach(function(el) { if (!el.isConnected) scrolled.delete(el); });
    if (wasTop !== (scrolled.size === 0)) window.flutter_inappwebview?.callHandler('vtopScroll', scrolled.size === 0);
  }, {capture: true, passive: true});
})();
"""

    /** `vtop_webview_body.dart` `_ready`: a signed-in page with VTOP's sidebar. */
    const val AUTHENTICATED =
        "location.origin === 'https://vtop.vitap.ac.in' && !location.pathname.startsWith('/vtop/login') && !!document.querySelector('a[data-url]')"

    /** `vtopPreferencesScript`. */
    fun preferences(dark: Boolean, compact: Boolean, desktop: Boolean) = """
(function() {
  if (location.origin !== 'https://vtop.vitap.ac.in') return;
  function apply() {
    if (!document.documentElement) return;
    ${if (dark) DARK_MODE else REMOVE_DARK_MODE}
    ${if (compact) compactSpacing(1) else RESET_SPACING}
    ${if (desktop) DESKTOP_MODE else RESET_DESKTOP_MODE}
    ${if (compact) KEEP_HEADER_SPACE else ""}
  }
  apply();
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', apply, {once: true});
  }
})();
"""

    /** `vtopActivityScript`: observes activity only. Request arguments, results and errors are preserved. */
    const val ACTIVITY = """
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
"""

    /** `vtopWaitForMenuScript`: clicks the sidebar link to [url] once it is there (10 s). */
    fun waitForMenu(url: String, requestId: String) = """
(function() {
  if (window.__mateMenuObserver) window.__mateMenuObserver();
  const target = ${JSONObject.quote(url)};
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
      ${JSONObject.quote(requestId)}, found);
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
"""

    /** `vtopPagesScript`: every page in VTOP's sidebar, grouped by its sidebar section. */
    const val PAGES = """
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
"""

    /** `vtopCloseOverlayScript`: closes the top-most VTOP overlay; true when something was closed. */
    const val CLOSE_OVERLAY = """
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
"""

    /** `vtopHomeScript`: VTOP's in-place Home, without reloading the document. */
    const val HOME = """
(function() {
  if (typeof window.home !== 'function') return false;
  window.__mateCurrentMenu = '';
  window.__mateCurrentTitle = '';
  window.home();
  return true;
})();
"""

    const val DARK_MODE = """
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
"""

    const val REMOVE_DARK_MODE = """
(function() {
  const style = document.getElementById('dark-mode-style');
  if (style) style.remove();
})();
"""

    const val RESET_SPACING = """
(function() {
  const customStyle = document.getElementById('custom-spacing-style');
  if (customStyle) {
    customStyle.remove();
  }
})();
"""

    const val RESET_DESKTOP_MODE = """
(function () {
  document.querySelectorAll('meta[name=viewport]').forEach(viewport => {
    viewport.setAttribute('content', 'width=device-width, initial-scale=1');
  });
})();
"""

    const val DESKTOP_MODE = """
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
"""

    /**
     * Not in the Flutter app: compact's `body { padding }` drops the inline `padding-top` VTOP sets
     * to clear its fixed navbar, so the page's top (Spotlight) hid under it. Puts it back.
     */
    private const val KEEP_HEADER_SPACE = """
(function() {
  const nav = document.querySelector('nav.fixed-top');
  if (nav && document.body) document.body.style.setProperty('padding-top', nav.offsetHeight + 'px', 'important');
})();
"""

    private fun compactSpacing(padding: Int) = """
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
"""
}
