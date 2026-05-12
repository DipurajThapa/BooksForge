import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const mobile = !!/android|ios/.exec(process.env.TAURI_ENV_PLATFORM ?? "");

// Mobile HMR needs the LAN IP via `internal-ip`. Mobile is out of scope for
// v1; on desktop the `internal-ip` import is unused, so it is dropped from
// the dependency list. If/when mobile is added back, reinstate the import
// inside the `mobile` branch only.
export default defineConfig(async () => ({
  plugins: [react()],
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
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    outDir: "dist",
  },
}));
