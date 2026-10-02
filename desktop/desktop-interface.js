(() => {
  'use strict';
  // Replaced by the shell with its configured development origin; null in release builds.
  const devOrigin = __DESKTOP_DEV_ORIGIN__;
  const local = location.protocol === 'tauri:' ||
    (['http:', 'https:'].includes(location.protocol) && location.hostname === 'tauri.localhost');
  if (window.top !== window || !(local || (devOrigin && location.origin === devOrigin))) return;
  const invoke = (command, args = {}) => window.__TAURI_INTERNALS__.invoke(command, args);
  let receiveFiles = null;
  const bridge = Object.freeze({
    version: 1,
    available: true,
    connect(handler) {
      if (typeof handler !== 'function') throw new TypeError('Expected playlist receiver');
      receiveFiles = handler;
      return () => { if (receiveFiles === handler) receiveFiles = null; };
    },
  });
  Object.defineProperty(window, '__tetoricaDesktop', {value: bridge});

  async function importSelection(selection) {
    if (receiveFiles) {
      const files = selection.tracks.map(track => ({
        name: track.name,
        size: track.size,
        arrayBuffer: async () => {
          const bytes = await invoke('library_read', {token: track.token});
          return bytes instanceof ArrayBuffer ? bytes : new Uint8Array(bytes).buffer;
        },
      }));
      await receiveFiles(files);
      return;
    }
    // Compatibility with older, unmodified Analyzer release ZIPs.
    // Their existing file-input handler accepts standard File objects.
    const input = document.getElementById('fileInput');
    if (!input) throw new Error('This Analyzer release has no file input. Update the Analyzer ZIP.');
    if (selection.tracks.reduce((size, track) => size + track.size, 0) > 256 * 1024 * 1024) {
      throw new Error('This older Analyzer release loads all selected files into memory. Choose a smaller folder or update the Analyzer ZIP.');
    }
    const transfer = new DataTransfer();
    for (const track of selection.tracks) {
      const bytes = await invoke('library_read', {token: track.token});
      transfer.items.add(new File([bytes instanceof ArrayBuffer ? bytes : new Uint8Array(bytes)], track.name, {type: 'application/octet-stream'}));
    }
    input.files = transfer.files;
    input.dispatchEvent(new Event('change', {bubbles: true}));
  }

  function confirmAction(message) {
    return new Promise(resolve => {
      const dialog = document.createElement('dialog');
      dialog.setAttribute('aria-label', 'Confirm action');
      const text = document.createElement('p');
      text.textContent = message;
      const cancel = document.createElement('button');
      cancel.type = 'button';
      cancel.textContent = 'Cancel';
      const proceed = document.createElement('button');
      proceed.type = 'button';
      proceed.textContent = 'Continue';
      let accepted = false;
      cancel.addEventListener('click', () => dialog.close());
      proceed.addEventListener('click', () => { accepted = true; dialog.close(); });
      dialog.addEventListener('close', () => {
        dialog.remove();
        resolve(accepted);
      }, {once:true});
      dialog.append(text, cancel, proceed);
      document.body.append(dialog);
      dialog.showModal();
      cancel.focus();
    });
  }

  function mount() {
    const resetCache = document.querySelector('[data-reset-offline-cache]');
    resetCache?.addEventListener('click', async (event) => {
      event.preventDefault();
      event.stopImmediatePropagation();
      if (resetCache.disabled) return;
      resetCache.disabled = true;
      if (!await confirmAction('Clear the offline cache and reload? Playback will stop.')) { resetCache.disabled = false; return; }
      resetCache.disabled = true;
      try {
        if (window.caches) {
          const keys = await window.caches.keys();
          await Promise.all(keys.filter(key => key.startsWith('hello-ymfm-docs-'))
            .map(key => window.caches.delete(key)));
        }
        const registrations = await window.navigator?.serviceWorker?.getRegistrations?.() ?? [];
        await Promise.all(registrations.map(registration => registration.unregister()));
        await invoke('window_reload');
      } catch (error) {
        resetCache.disabled = false;
        window.alert('Failed to reset offline cache: ' + String(error));
      }
    }, {capture:true});
    if (!document.getElementById('fileInput') || document.getElementById('desktop-library')) return;
    const host = document.createElement('div');
    host.id = 'desktop-library';
    document.body.append(host);
    const root = host.attachShadow({mode: 'open'});
    root.innerHTML = `
      <style>
        :host { font-family: system-ui, sans-serif; color: #34271f; font-size: 14px; }
        * { box-sizing: border-box; }
        button { font: inherit; color: inherit; cursor: pointer; border: 1px solid #d7c2b2; background: #fffaf0; border-radius: 7px; padding: 10px; }
        button:hover { background: #f4e5d4; }
        button:focus-visible { outline: 3px solid #ad4e28; outline-offset: 2px; }
        button:disabled { opacity: .55; cursor: wait; }
        #toggle { position: fixed; top: 12px; left: 12px; z-index: 1000; width: 44px; height: 44px; font-size: 24px; line-height: 1; box-shadow: 0 2px 10px #0002; }
        dialog { position: fixed; inset: 0 auto 0 0; margin: 0; height: 100dvh; max-height: 100%; width: min(420px, 100vw); max-width: 100%; border: 0; padding: 20px; background: #fff8ee; color: inherit; box-shadow: 4px 0 24px #0003; overflow-y: auto; }
        dialog::backdrop { background: #20150c66; }
        header { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
        h2 { margin: 0; font-size: 20px; } h3 { margin: 22px 0 8px; font-size: 13px; text-transform: uppercase; letter-spacing: .08em; color: #7a4b31; }
        #close { min-width: 44px; min-height: 44px; }
        .choices { display: grid; gap: 8px; margin-top: 20px; }
        .choices button { text-align: left; min-height: 44px; }
        ul { list-style: none; padding: 0; margin: 0; }
        li { display: grid; grid-template-columns: minmax(0, 1fr) 44px 44px; gap: 5px; margin: 6px 0; }
        .entry { text-align: left; overflow: hidden; }
        .label { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
        small { display: block; color: #756354; font-size: 11px; margin-top: 4px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
        [aria-pressed="true"] { background: #f6d89c; border-color: #b37928; }
        .empty, .hint { color: #756354; line-height: 1.5; }
        #status { white-space: pre-wrap; overflow-wrap: anywhere; line-height: 1.5; }
        #status.error { color: #a32d21; }
      </style>
      <button id="toggle" type="button" aria-label="Open files and recent folders" aria-haspopup="dialog" aria-expanded="false">☰</button>
      <dialog aria-labelledby="heading">
        <header><h2 id="heading">Files & Folders</h2><button id="close" type="button" aria-label="Close menu">✕</button></header>
        <div class="choices"><button id="files" type="button">Open Files…</button><button id="folder" type="button">Open Folder…</button><button id="alwaysTop" type="button" aria-pressed="false">Always on Top: Off</button></div>
        <p class="hint">VGM / VGZ / S98 · Folders include subfolders.<br>Open a collection, then press Play.</p>
        <h3>Pinned</h3><ul id="pinned"></ul>
        <h3>Recent</h3><ul id="recent"></ul>
        <p class="hint">Pin items to keep them. Recent keeps the last 20 unpinned collections. Removing an item only removes its history entry.</p>
        <p id="status" role="status" aria-live="polite"></p>
      </dialog>`;
    const get = id => root.getElementById(id);
    const dialog = root.querySelector('dialog');
    let entries = [], busy = false;
    function showWindowTop(enabled) {
      get('alwaysTop').setAttribute('aria-pressed', String(enabled));
      get('alwaysTop').textContent = `Always on Top: ${enabled ? 'On' : 'Off'}`;
    }
    function close() { if (!busy) dialog.close(); }
    dialog.addEventListener('close', () => { get('toggle').setAttribute('aria-expanded', 'false'); get('toggle').focus(); });
    dialog.addEventListener('cancel', e => { if (busy) e.preventDefault(); });
    dialog.addEventListener('click', e => { if (e.target === dialog && e.clientX > dialog.getBoundingClientRect().right) close(); });
    get('close').onclick = close;
    function render() {
      for (const pinned of [true, false]) {
        const list = get(pinned ? 'pinned' : 'recent');
        list.replaceChildren();
        const items = entries.filter(entry => entry.pinned === pinned);
        if (!items.length) {
          const empty = document.createElement('p'); empty.className = 'empty';
          empty.textContent = pinned ? 'No pinned items yet.' : 'No recent items yet.'; list.append(empty);
        }
        for (const entry of items) {
          const row = document.createElement('li');
          const open = document.createElement('button'); open.type = 'button'; open.className = 'entry';
          const label = document.createElement('span'); label.className = 'label';
          label.textContent = `${entry.kind === 'folder' ? '📁' : '♪'} ${entry.label}`;
          const path = document.createElement('small'); path.textContent = entry.paths.join(' · ');
          open.title = path.textContent; open.append(label, path);
          open.onclick = () => perform(async () => load(await invoke('library_open', {id: entry.id})));
          const pin = document.createElement('button'); pin.type = 'button'; pin.textContent = entry.pinned ? '★' : '☆';
          pin.setAttribute('aria-label', `${entry.pinned ? 'Unpin' : 'Pin'} ${entry.label}`);
          pin.setAttribute('aria-pressed', String(entry.pinned)); pin.title = entry.pinned ? 'Unpin' : 'Pin';
          pin.onclick = () => perform(async () => {
            entries = await invoke('library_pin', {id: entry.id, pinned: !entry.pinned});
            render();
          });
          const remove = document.createElement('button'); remove.type = 'button'; remove.textContent = '×';
          remove.title = 'Remove from history'; remove.setAttribute('aria-label', `Remove ${entry.label} from history`);
          remove.onclick = () => perform(async () => { entries = await invoke('library_remove', {id: entry.id}); render(); });
          for (const [button, action] of [[open, 'open'], [pin, 'pin'], [remove, 'remove']]) {
            button.dataset.entryId = entry.id; button.dataset.action = action;
          }
          row.append(open, pin, remove); list.append(row);
        }
      }
    }
    async function load(selection) {
      if (!selection) return false;
      entries = selection.entries; render();
      await importSelection(selection);
      return true;
    }
    async function perform(action) {
      if (busy) return;
      const focused = root.activeElement;
      busy = true; get('status').className = ''; get('status').textContent = 'Working…';
      root.querySelectorAll('button').forEach(b => { b.disabled = true; });
      try {
        const loaded = await action();
        get('status').textContent = '';
        if (loaded === true) dialog.close();
      } catch (error) {
        get('status').className = 'error'; get('status').textContent = String(error?.message || error);
      } finally {
        busy = false;
        root.querySelectorAll('button').forEach(b => { b.disabled = false; });
        if (!dialog.open) get('toggle').focus();
        else if (focused?.dataset.entryId) {
          const replacement = Array.from(root.querySelectorAll('button')).find(button =>
            button.dataset.entryId === focused.dataset.entryId && button.dataset.action === focused.dataset.action);
          (replacement || get('files')).focus();
        } else if (focused?.isConnected && focused !== get('toggle')) focused.focus();
        else get('files').focus();
      }
    }
    get('toggle').onclick = () => {
      dialog.showModal(); get('toggle').setAttribute('aria-expanded', 'true');
      void perform(async () => { entries = await invoke('library_list'); render(); showWindowTop(await invoke('window_top_get')); });
    };
    get('alwaysTop').onclick = () => perform(async () => {
      try {
        showWindowTop(await invoke('window_top_set', {enabled: get('alwaysTop').getAttribute('aria-pressed') !== 'true'}));
      } catch (error) {
        // Reflect the real window state even if changing it failed.
        try { showWindowTop(await invoke('window_top_get')); } catch (_) {}
        throw error;
      }
    });
    get('files').onclick = () => perform(async () => load(await invoke('library_choose', {folder: false})));
    get('folder').onclick = () => perform(async () => load(await invoke('library_choose', {folder: true})));
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', mount, {once: true});
  else mount();
})();
