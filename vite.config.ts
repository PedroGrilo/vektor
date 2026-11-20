import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import pkg from "./package.json" with { type: "json" };

// Tauri expects a fixed dev port and no screen clearing.
export default defineConfig(({ mode }) => ({
  plugins: [react()],
  clearScreen: false,
  // Single source for the version shown in the UI.
  // `vite build --mode lite` builds the portable Vektor Lite UI (no games, CS2, NVIDIA, memory).
  define: { __APP_VERSION__: JSON.stringify(pkg.version), __LITE__: JSON.stringify(mode === "lite") },
  server: { port: 1420, strictPort: true },
  build: { target: "es2022", outDir: "dist" },
}));
