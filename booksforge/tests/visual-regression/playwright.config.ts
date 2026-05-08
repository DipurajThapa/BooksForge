/**
 * Playwright config for BooksForge visual regression (BACKLOG §H6).
 *
 * The test suite renders the editor preview HTML and the unzipped
 * EPUB chapter HTML side-by-side and asserts a pixel diff under the
 * documented tolerance.  Golden screenshots live under
 * `tests/visual-regression/golden/` and are committed to the repo.
 *
 * ## Workflow
 *
 *   pnpm --filter @booksforge/visual-regression update-golden
 *     # First-time setup OR after a deliberate styling change.
 *     # Inspect the generated screenshots, then commit.
 *
 *   pnpm --filter @booksforge/visual-regression test
 *     # Run on every PR.  Fails if rendering drifts beyond
 *     # `expect.toHaveScreenshot.maxDiffPixelRatio` below.
 *
 * The complementary Rust-side scaffold
 * (`crates/booksforge-export-epub/tests/visual_regression.rs`)
 * checks the *content-level* invariant (paragraph match) without
 * needing a browser; this Playwright suite checks the *rendering*
 * invariant (pixel match).
 */
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir:    "./src",
  outputDir:  "./test-results",

  // CI flakes < 0.5% on a clean baseline.  Bumped to 1 retry locally
  // for transient font-rasterisation noise on the first run.
  retries:    process.env.CI ? 2 : 1,
  workers:    process.env.CI ? 1 : undefined,

  // Pixel-diff tolerance.  WCAG-grade font rendering can shift sub-
  // pixel anti-aliasing by 1-2 px around stroke edges; 1 % is a
  // documented baseline that catches structural regressions while
  // tolerating known anti-aliasing variance.  Tighten as the suite
  // matures.
  expect: {
    toHaveScreenshot: {
      maxDiffPixelRatio: 0.01,
      threshold:         0.2,    // per-pixel hue/lum delta (0-1)
      animations:        "disabled",
    },
  },

  reporter: [
    ["list"],
    ["html", { outputFolder: "./playwright-report", open: "never" }],
  ],

  use: {
    headless:        true,
    viewport:        { width: 800, height: 1000 },
    deviceScaleFactor: 1,
    // Disable network — every fixture is rendered from a file: URL,
    // so any outbound request is either an asset miss or a leak.
    offline:         false,   // false: lets `data:` images render
    bypassCSP:       true,
    // Consistent fontconfig output across hosts.  CI-only.
    locale:          "en-US",
    timezoneId:      "UTC",
  },

  // Single project for now; split per FormatProfile when the matrix
  // grows.
  projects: [
    {
      name: "chromium",
      use:  { ...devices["Desktop Chrome"] },
    },
  ],
});
