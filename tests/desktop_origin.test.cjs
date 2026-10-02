const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const source = fs.readFileSync(path.join(__dirname, '../desktop/desktop-interface.js'), 'utf8');

function boot(url, devOrigin = null, subframe = false) {
  const window = {};
  window.top = subframe ? {} : window;
  const listeners = [];
  const document = {readyState: 'loading', addEventListener: (...args) => listeners.push(args)};
  vm.runInNewContext(source.replace('__DESKTOP_DEV_ORIGIN__', JSON.stringify(devOrigin)), {
    window, location: new URL(url), document,
  });
  return {window, listeners};
}

test('packaged macOS and Windows URLs initialize the menu', () => {
  for (const url of ['tauri://localhost/index.html', 'http://tauri.localhost/index.html']) {
    const result = boot(url);
    assert.equal(result.window.__tetoricaDesktop.available, true);
    assert.equal(result.listeners[0][0], 'DOMContentLoaded');
  }
});
test('configured Tauri development server initializes the same menu', () => {
  for (const origin of ['http://localhost:1430', 'http://127.0.0.1:1430']) {
    const result = boot(origin + '/index.html', origin);
    assert.equal(result.window.__tetoricaDesktop.available, true);
    assert.equal(result.listeners[0][0], 'DOMContentLoaded');
  }
});
test('other localhost ports, remote pages and subframes do not receive the bridge', () => {
  for (const [url, dev, frame] of [
    ['http://localhost:1430', null, false],
    ['http://localhost:1431', 'http://localhost:1430', false],
    ['https://example.com', 'http://localhost:1430', false],
    ['tauri://localhost/index.html', null, true],
  ]) {
    const result = boot(url, dev, frame);
    assert.equal(result.window.__tetoricaDesktop, undefined);
    assert.equal(result.listeners.length, 0);
  }
});

test('offline cache reset invokes native reload, supports missing APIs and recovers from errors', async () => {
  for (const available of [false, true]) {
    let listener, confirmed = false, fail = false;
    const actions = [];
    const button = {addEventListener: (name, fn) => { listener = fn; }};
    const document = {readyState:'complete', querySelector:() => button, getElementById:() => null,
      createElement:() => ({
        listeners:{}, setAttribute(){}, append(...children){this.children = children;},
        addEventListener(name, fn){this.listeners[name] = fn;},
        close(){this.listeners.close();}, remove(){}, showModal(){}, focus(){}
      }),
      body:{append(dialog){setImmediate(() => dialog.children[confirmed ? 2 : 1].listeners.click());}}
    };
    const window = {confirm:() => confirmed, alert:message => actions.push(message),
      __TAURI_INTERNALS__:{invoke:async command => {
        assert.equal(command,'window_reload');
        if (fail) throw Error('reload failed');
        actions.push('reload');
      }}};
    window.top = window;
    if (available) {
      window.caches = {keys:async () => ['hello-ymfm-docs-v1','other'],
        delete:async key => actions.push(key)};
      window.navigator = {serviceWorker:{getRegistrations:async () => [
        {unregister:async () => actions.push('unregister')}
      ]}};
    }
    vm.runInNewContext(source.replace('__DESKTOP_DEV_ORIGIN__','null'), {
      window,document,location:new URL('tauri://localhost/index.html')
    });
    const event = {preventDefault(){},stopImmediatePropagation(){}};
    await listener(event);
    assert.deepEqual(actions,[]);
    confirmed = true;
    await listener(event);
    assert.deepEqual(actions,available ? ['hello-ymfm-docs-v1','unregister','reload'] : ['reload']);
    fail = true;
    button.disabled = false;
    await listener(event);
    assert.equal(button.disabled,false);
    assert.match(actions.at(-1),/reload failed/);
  }
});
