import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { visualizer } from "rollup-plugin-visualizer";
import path from "node:path";

const mobile = !!/android|ios/.exec(process.env.TAURI_ENV_PLATFORM ?? "");

// Mobile HMR needs the LAN IP via `internal-ip`. Mobile is out of scope for
// v1; on desktop the `internal-ip` import is unused, so it is dropped from
// the dependency list. If/when mobile is added back, reinstate the import
// inside the `mobile` branch only.

// Audit #52 — bundle-size visualisation.  When BOOKSFORGE_BUNDLE_REPORT=1
// (or running under CI), emit a treemap to `dist/bundle-report.html`.
// Local dev runs skip this so HMR stays cheap.
const reportBundle = !!process.env.BOOKSFORGE_BUNDLE_REPORT || !!process.env.CI;

export default defineConfig(async () => ({
  plugins: [
    react(),
    ...(reportBundle
      ? [visualizer({
          filename: path.resolve(__dirname, "dist/bundle-report.html"),
          template: "treemap",
          gzipSize: true,
          brotliSize: true,
          // No `open: true` — CI shouldn't try to launch a browser.
        })]
      : []),
  ],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: mobile ? "0.0.0.0" : false,
    hmr: mobile
      ? { protocol: "ws", host: "localhost", port: 5183 }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    // Audit #51 — source maps ALWAYS emitted, even for release builds.
    // Required to symbolicate production stack traces from crash
    // reports.  They live alongside the JS in `dist/` for the local
    // build step; the release pipeline uploads them to a private
    // artefact store and strips them out of the shipped Tauri bundle
    // (see `docs/DISTRIBUTION.md §sourcemaps`).
    sourcemap: true,
    outDir: "dist",
  },
}));
