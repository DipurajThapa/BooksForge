import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    // jsdom 25 requires a concrete origin before localStorage is exposed
    // on globalThis — without `url`, `localStorage` is `undefined` and
    // every test that touches it fails with "Cannot read properties of
    // undefined (reading 'clear')". Pinning to an http origin (rather
    // than `about:blank`) makes both Storage and the structured-clone
    // base URL available.
    environmentOptions: {
      jsdom: {
        url: "http://localhost/",
      },
    },
    setupFiles: ["./vitest.setup.ts"],
    globals: false,
    setupFiles: ["./src/test-setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
    coverage: {
      reporter: ["text", "html"],
      include: ["src/**/*.{ts,tsx}"],
      exclude: [
        "src/**/*.test.{ts,tsx}",
        "src/main.tsx",
        "src/test-setup.ts",
      ],
    },
  },
});
