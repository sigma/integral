import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    fs: {
      // Allow serving files from the pkg directory (wasm-pack output)
      allow: [".", "pkg"],
    },
  },
});
