import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri attend un port fixe en dev et ne doit pas voir son propre dossier Rust surveillé.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "chrome120", // WebView2 uniquement : pas de polyfills, bundle minimal
    sourcemap: false,
    reportCompressedSize: false,
    rollupOptions: {
      input: {
        main: fileURLToPath(new URL("./index.html", import.meta.url)),
        quick: fileURLToPath(new URL("./quick.html", import.meta.url)),
      },
    },
  },
});
