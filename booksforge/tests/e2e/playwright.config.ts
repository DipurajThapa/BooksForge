import { defineConfig } from "@playwright/test";

/**
 * Playwright config for the BooksForge end-to-end suite.
 *
 * The Tauri app is launched via `tauri-driver` — a WebDriver-compatible
 * shim that lets Playwright drive a real Tauri build (not a separate
 * web app).  Until tauri-driver is wired into release.yml, every test
 * is `test.skip`'d (see README).
 */

export default defineConfig({
  testDir: "./src",
  fullyParallel: false, // E2E exercises a real desktop binary; serialise.
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: process.env.CI ? "github" : "list",
  use: {
    actionTimeout: 10_000,
    navigationTimeout: 15_000,
    trace: "on-first-retry",
    screenshot: "only-on-failure",
  },
  projects: [
    {
      name: "tauri-app",
      use: {
        // Placeholder — `tauri-driver` config lands when the harness
        // is wired in CI.
      },
    },
  ],
});
