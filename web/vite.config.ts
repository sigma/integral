/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { VitePWA } from "vite-plugin-pwa";

// Use repo name as base path for GitHub Pages; "/" for local dev.
const base = process.env.GITHUB_PAGES === "true" ? "/integral/" : "/";

/** App background, as in index.html; also the installed window's colour. */
const BACKGROUND = "#0f0f1a";

export default defineConfig({
  plugins: [
    react(),
    VitePWA({
      // A new deploy takes over at once (skipWaiting + clientsClaim) and
      // reloads open pages (registered in main.tsx), so JS and the WASM it
      // loads always come from one build.
      registerType: "autoUpdate",
      manifest: {
        name: "Integral",
        short_name: "Integral",
        description: "Control surface for the Roland INTEGRA-7",
        display: "standalone",
        start_url: base,
        scope: base,
        theme_color: BACKGROUND,
        background_color: BACKGROUND,
        // Rendered from public/icon.svg by `just web-icons`.
        icons: [
          { src: "pwa-192.png", sizes: "192x192", type: "image/png" },
          { src: "pwa-512.png", sizes: "512x512", type: "image/png" },
          {
            src: "pwa-512.png",
            sizes: "512x512",
            type: "image/png",
            purpose: "maskable",
          },
        ],
      },
      workbox: {
        // Precache the whole build, WASM included, so the app launches offline
        // (the manifest icons are added by the plugin).
        globPatterns: ["**/*.{js,css,html,wasm,svg}"],
        maximumFileSizeToCacheInBytes: 8 * 1024 * 1024,
        cleanupOutdatedCaches: true,
      },
    }),
  ],
  base,
  server: {
    fs: {
      // Allow serving files from the pkg directory (wasm-pack output)
      allow: [".", "pkg"],
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./vitest.setup.ts"],
  },
});
