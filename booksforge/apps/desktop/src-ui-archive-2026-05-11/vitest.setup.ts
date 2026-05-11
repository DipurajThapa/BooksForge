/**
 * Vitest setup file — runs before every test file.
 *
 * Two cross-cutting concerns are configured here so every test file
 * inherits them:
 *
 * 1. **`Storage` polyfill** — some vitest 2 + jsdom 25 combinations
 *    fail to expose `localStorage` / `sessionStorage` on `globalThis`
 *    (the JSDOM `Storage` object is bound to a `window`, but
 *    `globalThis` doesn't always proxy them when the test file
 *    imports modules that read them at module-load time). The
 *    polyfill below is a minimal in-memory impl that's idempotent —
 *    if jsdom already exposed real Storage, we do nothing.
 *
 * 2. **React Testing Library auto-cleanup** — with `globals: false`
 *    in `vitest.config.ts`, `@testing-library/react`'s built-in
 *    after-each cleanup hook does NOT auto-register, so successive
 *    `render(...)` calls in the same test file leak DOM state. Tests
 *    then fail with "Found multiple elements with the text…" because
 *    `screen.getByText` queries the accumulated DOM. Calling
 *    `afterEach(cleanup)` here (once) restores the default behaviour
 *    everywhere without touching individual test files.
 */
import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";

afterEach(() => {
  cleanup();
});

class MemoryStorage {
  private store = new Map<string, string>();
  get length(): number { return this.store.size; }
  clear(): void { this.store.clear(); }
  getItem(key: string): string | null {
    return this.store.has(key) ? (this.store.get(key) as string) : null;
  }
  key(i: number): string | null {
    return Array.from(this.store.keys())[i] ?? null;
  }
  removeItem(key: string): void { this.store.delete(key); }
  setItem(key: string, value: string): void { this.store.set(key, String(value)); }
}

function ensureStorage(name: "localStorage" | "sessionStorage") {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const g = globalThis as any;
  if (g[name] && typeof g[name].clear === "function") return; // jsdom provided one
  Object.defineProperty(g, name, {
    value: new MemoryStorage(),
    writable: true,
    configurable: true,
  });
}

ensureStorage("localStorage");
ensureStorage("sessionStorage");
