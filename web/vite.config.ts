import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // Use repo name as base path for GitHub Pages; "/" for local dev.
  base: process.env.GITHUB_PAGES === "true" ? "/integral/" : "/",
  server: {
    fs: {
      // Allow serving files from the pkg directory (wasm-pack output)
      allow: [".", "pkg"],
    },
  },
});
