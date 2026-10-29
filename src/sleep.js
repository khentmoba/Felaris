(() => {
  let edited = false;
  let beforeUnload = false;
  let captures = 0;
  // Chromium only shows a beforeunload prompt after the user clicked or typed in the frame or one of its children.
  let activated = false;
  let reported;
  const activeMedia = new Set();
  // Child frames report their own state to the top frame, which keeps the tab awake while any frame is busy.
  const frames = new Map();
  const top = window.top === window;
  // Back/forward navigations restore form drafts without firing input events.
  const restored = () => performance.getEntriesByType('navigation')[0]?.type === 'back_forward'
    && [...document.querySelectorAll('input,textarea,select')].some(field =>
      field.type === 'checkbox' || field.type === 'radio' ? field.checked !== field.defaultChecked
        : field.options ? [...field.options].some(option => option.selected !== option.defaultSelected)
          : field.value !== field.defaultValue);
  const busy = () => edited || (activated && (beforeUnload || Boolean(window.onbeforeunload))) || captures > 0 || activeMedia.size > 0 || restored();
  const report = () => {
    if (top) return;
    const state = { velunoFrameBusy: busy(), velunoFrameActivated: activated };
    const key = JSON.stringify(state);
    if (key === reported) return;
    reported = key;
    window.top.postMessage(state, '*');
  };
  const listen = window.addEventListener.bind(window);
  if (top) listen('message', event => {
    if (typeof event.data?.velunoFrameBusy !== 'boolean') return;
    event.stopImmediatePropagation();
    // The source is null once its frame is gone.
    if (event.source) frames.set(event.source, event.data.velunoFrameBusy);
    if (event.data.velunoFrameActivated === true) activated = true;
  }, true);
  listen('load', report);
  for (const type of ['pointerdown', 'keydown']) listen(type, event => { if (event.isTrusted) { activated = true; report(); } }, true);
  listen('input', () => { edited = true; report(); }, true);
  listen('change', () => { edited = true; report(); }, true);
  listen('play', event => { activeMedia.add(event.target); report(); }, true);
  // WebView2 can pause muted media when hidden; keep its playback state alive.
  listen('pause', event => { if (!document.hidden) activeMedia.delete(event.target); report(); }, true);
  listen('ended', event => { activeMedia.delete(event.target); report(); }, true);
  const addListener = window.addEventListener;
  window.addEventListener = function(type, ...args) {
    if (type === 'beforeunload') { beforeUnload = true; report(); }
    return addListener.call(this, type, ...args);
  };
  const devices = navigator.mediaDevices;
  for (const method of ['getUserMedia', 'getDisplayMedia']) {
    if (!devices?.[method]) continue;
    const original = devices[method].bind(devices);
    devices[method] = (...args) => {
      captures++;
      report();
      return original(...args).then(stream => {
        const tracks = stream.getTracks();
        captures--;
        for (const track of tracks) {
          captures++;
          track.addEventListener('ended', () => { captures--; report(); }, { once: true });
        }
        report();
        return stream;
      }, error => { captures--; report(); throw error; });
    };
  }
  Object.defineProperty(window, '__velunoCanSleep', { value: () => {
    if (document.readyState !== 'complete' || busy()) return false;
    for (const [frame, frameBusy] of frames) {
      if (frame.closed) frames.delete(frame);
      else if (frameBusy) return false;
    }
    for (const media of document.querySelectorAll('audio,video')) {
      if (!media.paused && !media.ended) return false;
    }
    return true;
  } });
})();
