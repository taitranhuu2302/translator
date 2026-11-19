import { fileURLToPath, URL } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    rollupOptions: {
      // Three windows: the Translate Window, the Quick Translate popup and the
      // Replace HUD. The preview harness is deliberately absent: it is dev-only.
      input: {
        main: "index.html",
        popup: "popup.html",
        hud: "hud.html",
      },
    },
  },
});
