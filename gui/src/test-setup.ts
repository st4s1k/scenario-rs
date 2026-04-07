// Provide a minimal mock of Tauri internals so that module-level calls
// like `getCurrentWebviewWindow()` do not crash in the Karma/Chrome
// test environment, where the Tauri runtime is absent.
let _tauriCallbackId = 0;

(window as any).__TAURI_INTERNALS__ = {
  metadata: {
    currentWindow: { label: 'main' },
    currentWebview: { label: 'main' },
  },
  invoke: () => Promise.resolve(),
  transformCallback: (callback: Function, once?: boolean) => {
    const id = _tauriCallbackId++;
    const prop = `_${id}`;
    Object.defineProperty(window, prop, {
      value: (result: any) => {
        if (once) {
          Reflect.deleteProperty(window, prop);
        }
        return callback(result);
      },
      writable: false,
      configurable: true,
    });
    return id;
  },
  convertFileSrc: (src: string) => src,
};

// Event plugin internals used by @tauri-apps/api/event for listener management
(window as any).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
  unregisterListener: () => {},
};
