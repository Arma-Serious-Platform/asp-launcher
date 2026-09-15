import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const host = process.env.TAURI_DEV_HOST;
const root = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig(() => ({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(root, "src"),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
    proxy: {
      "/vtg-api": {
        target: "https://service.beta.vtg.in.ua",
        changeOrigin: true,
        rewrite: (path: string) => path.replace(/^\/vtg-api/, "/api"),
      },
    },
  },
}));
