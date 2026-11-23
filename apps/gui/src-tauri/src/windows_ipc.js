function preferTauriMessages(appOrigin) {
  if (window !== window.top || window.location.origin !== appOrigin) return;

  // Tauri 2.11's built-in transport switches to WebView2 postMessage on
  // a rejected fetch. Its custom-protocol fetch can stay pending instead
  // of rejecting on Windows. Reject just that transport so Tauri can
  // use its own invocation key, serializers, callbacks and capability checks.
  const fetch = window.fetch;
  window.fetch = function (input, init) {
    if (typeof input === "string" &&
        (input.startsWith("http://ipc.localhost/") || input.startsWith("https://ipc.localhost/"))) {
      return Promise.reject(new TypeError("Use Tauri's WebView2 message transport"));
    }
    return fetch.call(window, input, init);
  };
}
