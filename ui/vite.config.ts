import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    rollupOptions: {
      // Two windows: the Translate Window and the Quick Translate popup.
      input: {
        main: "index.html",
        popup: "popup.html",
      },
    },
  },
});
