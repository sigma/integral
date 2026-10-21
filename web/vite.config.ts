/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { VitePWA } from "vite-plugin-pwa";

/** App background, as in index.html; also the installed window's colour. */
const BACKGROUND = "#0f0f1a";

// `--mode plugin` builds the copy the plugin embeds (`just build-web-plugin`,
// ADR 0004): served from `integral://localhost/`, so base `/`, and no
// service worker or manifest, which mean nothing inside a plugin.
export default defineConfig(({ mode }) => {
  const plugin = mode === "plugin";
  // Use repo name as base path for GitHub Pages; "/" for local dev and the plugin.
  const base =
    !plugin && process.env.GITHUB_PAGES === "true" ? "/integral/" : "/";

  return {
    plugins: [
      react(),
      VitePWA({
        // Off in the plugin build: `virtual:pwa-register` then resolves to
        // a no-op, and no sw.js or manifest is emitted.
        disable: plugin,
        // A new deploy waits until the user accepts the reload offered by
        // PwaUpdateBanner (registered in main.tsx), so JS and the WASM it
        // loads always come from one build — and no open page is reloaded
        // mid-session.
        registerType: "prompt",
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
          // Precache the whole build, WASM included, so the app launches
          // offline (the manifest icons are added by the plugin).
          globPatterns: ["**/*.{js,css,html,wasm,svg}"],
          maximumFileSizeToCacheInBytes: 8 * 1024 * 1024,
          cleanupOutdatedCaches: true,
        },
      }),
    ],
    base,
    build: plugin ? { outDir: "dist-plugin" } : {},
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
  };
});
