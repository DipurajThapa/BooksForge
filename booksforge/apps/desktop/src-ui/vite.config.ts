import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { internalIpV4 } from "internal-ip";

const mobile = !!/android|ios/.exec(process.env.TAURI_ENV_PLATFORM ?? "");

export default defineConfig(async () => ({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: mobile ? "0.0.0.0" : false,
    hmr: mobile
      ? { protocol: "ws", host: await internalIpV4(), port: 5183 }
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
