const assert = require('node:assert/strict');
const vm = require('node:vm');
let input = '';
process.stdin.on('data', chunk => input += chunk);
process.stdin.on('end', async () => {
  try {
    const scripts = JSON.parse(input);
    const events = [];
    const timers = new Map();
    let timerId = 0;
    const observers = [];
    const elements = [];
    let menuLinks = [];
    function element(tag) {
      return {
        tag, attrs: {},
        setAttribute(key, value) { this.attrs[key] = value; },
        getAttribute(key) { return this.attrs[key]; },
        remove() { const index = elements.indexOf(this); if (index >= 0) elements.splice(index, 1); },
      };
    }
    const root = { appendChild(node) { elements.push(node); } };
    const document = {
      documentElement: root, head: root, readyState: 'complete',
      createElement: element,
      getElementById(id) { return elements.find(e => e.id === id); },
      querySelector(selector) {
        if (selector === 'meta[name=viewport]') return elements.find(e => e.name === 'viewport');
        return null;
      },
      querySelectorAll(selector) {
        return selector === 'meta[name=viewport]' ? elements.filter(e => e.name === 'viewport') : menuLinks;
      },
      addEventListener() {},
    };
    class XHR {
      listeners = new Map();
      open(method, url) { this.method = method; this.url = url; }
      send(body) { this.body = body; if (body === 'throw') throw new Error('send error'); return 42; }
      addEventListener(type, fn) { this.listeners.set(type, fn); }
      removeEventListener(type, fn) { if (this.listeners.get(type) === fn) this.listeners.delete(type); }
      abort() { this.listeners.get('loadend')?.(); }
    }
    let resolveFetch;
    let rejectFetch;
    let returned;
    function originalFetch(...args) {
      if (args[0] === '/throw') throw new Error('fetch error');
      returned = new Promise((resolve, reject) => { resolveFetch = resolve; rejectFetch = reject; });
      return returned;
    }
    const context = vm.createContext({
      document, location: { origin: 'https://vtop.vitap.ac.in', href: 'https://vtop.vitap.ac.in/vtop/content', pathname: '/vtop/content' },
      performance: {timeOrigin: 123, now: () => 20, getEntriesByType: () => []},
      URL, Request: class Request { constructor(url) { this.url = url; } },
      XMLHttpRequest: XHR, fetch: originalFetch,
      flutter_inappwebview: {callHandler(...args) { events.push(args); }},
      addEventListener() {},
      setTimeout(fn) { const id = ++timerId; timers.set(id, fn); return id; },
      clearTimeout(id) { timers.delete(id); },
      MutationObserver: class {
        constructor(fn) { this.fn = fn; observers.push(this); }
        observe() { this.active = true; }
        disconnect() { this.active = false; }
      },
    });
    context.window = context;
    const run = script => vm.runInContext(script, context);
    run(scripts.stylesOn);
    // VTOP can add its own viewport after the document-start script.
    const siteViewport = element('meta');
    siteViewport.name = 'viewport';
    siteViewport.setAttribute('content', 'width=device-width');
    elements.push(siteViewport);
    run(scripts.stylesOn);
    assert.match(siteViewport.attrs.content, /1024/);
    assert.equal(elements.filter(e => e.id === 'custom-spacing-style').length, 1);
    assert.equal(elements.filter(e => e.id === 'dark-mode-style').length, 1);
    assert.match(document.querySelector('meta[name=viewport]').attrs.content, /1024/);
    assert.equal(timers.size, 0, 'styles have no delayed redraw');
    run(scripts.stylesOff);
    assert.equal(document.getElementById('custom-spacing-style'), undefined);
    assert.equal(document.getElementById('dark-mode-style'), undefined);
    assert.match(document.querySelector('meta[name=viewport]').attrs.content, /device-width/);
    assert.match(siteViewport.attrs.content, /device-width/);

    run(scripts.activity);
    const wrappedFetch = context.fetch;
    run(scripts.activity);
    assert.equal(context.fetch, wrappedFetch, 'bridge installs once');
    const activity = () => events.filter(e => e[1]?.type === 'activity').map(e => e[1].active);
    const request = context.fetch('/data', {method: 'POST', body: 'untouched'});
    assert.equal(request, returned, 'original fetch promise is returned');
    const xhr = new XHR(); xhr.open('POST', '/menu');
    assert.equal(xhr.send('payload'), 42);
    assert.equal(xhr.body, 'payload');
    assert.deepEqual(activity(), [1, 2]);
    xhr.abort();
    resolveFetch('response');
    assert.equal(await request, 'response');
    await Promise.resolve();
    assert.deepEqual(activity(), [1, 2, 1, 0]);
    xhr.abort();
    assert.equal(activity().at(-1), 0, 'completion is counted once');
    const failure = context.fetch('/failed');
    rejectFetch(new Error('network error'));
    await assert.rejects(failure, /network error/);
    await Promise.resolve();
    assert.equal(activity().at(-1), 0);
    assert.throws(() => context.fetch('/throw'), /fetch error/);
    assert.equal(activity().at(-1), 0);
    const throwingXHR = new XHR(); throwingXHR.open('POST', '/menu');
    assert.throws(() => throwingXHR.send('throw'), /send error/);
    assert.equal(activity().at(-1), 0);
    const count = activity().length;
    const external = context.fetch('https://example.com/asset'); resolveFetch('ok'); await external;
    assert.equal(activity().length, count, 'other origins are not tracked');

    let clicked = 0;
    run(scripts.menu);
    const observer = observers.at(-1);
    menuLinks = [{getAttribute: () => 'academics/test', click: () => clicked++}];
    observer.fn(); observer.fn();
    assert.equal(clicked, 1);
    assert.equal(observer.active, false);
    assert.equal(timers.size, 0);
    assert.equal(events.filter(e => e[0] === 'vtopMenuResult').at(-1)[2], true);
    menuLinks = [];
    run(scripts.menu);
    for (const callback of [...timers.values()]) callback();
    assert.equal(events.filter(e => e[0] === 'vtopMenuResult').at(-1)[2], false);
    assert.equal(observers.at(-1).active, false);
    run(scripts.quotedMenu);
    menuLinks = [{getAttribute: () => 'a"\\\n]', click: () => clicked++}];
    observers.at(-1).fn();
    assert.equal(clicked, 2, 'escaped destinations work without selector interpolation');
    console.log('WebView JavaScript checks passed');
  } catch (error) { console.error(error); process.exitCode = 1; }
});
