import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The window loads this UI from disk in builds and from this dev server in `tauri dev`.
export default defineConfig({
  root: import.meta.dirname,
  plugins: [react()],
  clearScreen: false,
  server: { host: "127.0.0.1", port: 1430, strictPort: true },
  build: { outDir: "dist", emptyOutDir: true, target: "safari16" },
});
