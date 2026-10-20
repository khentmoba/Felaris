(() => {
  let edited = false;
  let beforeUnload = false;
  let captures = 0;
  const activeMedia = new Set();
  const listen = window.addEventListener.bind(window);
  listen('input', () => { edited = true; }, true);
  listen('change', () => { edited = true; }, true);
  listen('play', event => { activeMedia.add(event.target); }, true);
  // WebView2 can pause muted media when hidden; keep its playback state alive.
  listen('pause', event => { if (!document.hidden) activeMedia.delete(event.target); }, true);
  listen('ended', event => { activeMedia.delete(event.target); }, true);
  const addListener = window.addEventListener;
  window.addEventListener = function(type, ...args) {
    if (type === 'beforeunload') beforeUnload = true;
    return addListener.call(this, type, ...args);
  };
  const devices = navigator.mediaDevices;
  for (const method of ['getUserMedia', 'getDisplayMedia']) {
    if (!devices?.[method]) continue;
    const original = devices[method].bind(devices);
    devices[method] = (...args) => {
      captures++;
      return original(...args).then(stream => {
        const tracks = stream.getTracks();
        captures--;
        for (const track of tracks) {
          captures++;
          track.addEventListener('ended', () => { captures--; }, { once: true });
        }
        return stream;
      }, error => { captures--; throw error; });
    };
  }
  Object.defineProperty(window, '__felarisCanSleep', { value: () => {
    if (document.readyState !== 'complete' || edited || beforeUnload || window.onbeforeunload || captures || activeMedia.size || window.frames.length) return false;
    for (const media of document.querySelectorAll('audio,video')) {
      if (!media.paused && !media.ended) return false;
    }
    for (const field of document.querySelectorAll('input,textarea,select')) {
      if (field.type === 'checkbox' || field.type === 'radio') {
        if (field.checked !== field.defaultChecked) return false;
      } else if (field.options) {
        if ([...field.options].some(option => option.selected !== option.defaultSelected)) return false;
      } else if (field.value !== field.defaultValue) return false;
    }
    return true;
  } });
})();
