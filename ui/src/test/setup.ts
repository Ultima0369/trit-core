import '@testing-library/jest-dom';

// jsdom does not ship ResizeObserver. Earth.tsx uses it to track container size
// and drive canvas resize. Provide a no-op stub so component tests can mount.
class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}
(globalThis as any).ResizeObserver = ResizeObserverStub;

// vitest/node 不带 localStorage，App.tsx 用它持久化字体/速度偏好。提供内存 stub。
const store = new Map<string, string>();
const localStorageStub = {
  getItem: (k: string) => store.get(k) ?? null,
  setItem: (k: string, v: string) => { store.set(k, String(v)); },
  removeItem: (k: string) => { store.delete(k); },
  clear: () => store.clear(),
  key: (i: number) => [...store.keys()][i] ?? null,
  get length() { return store.size; },
};
(globalThis as any).localStorage = localStorageStub;

// App.tsx uses isTauriEnvironment() (window.__TAURI_INTERNALS__) to decide
// whether to call the (mocked) Tauri invoke. Set it so tests exercise the
// real invoke path rather than the throw-on-no-backend branch.
(window as any).__TAURI_INTERNALS__ = true;
