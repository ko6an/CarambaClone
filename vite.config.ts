import { defineConfig } from "vite";
import tailwindcss from "@tailwindcss/vite";

// Vite для интерфейса CarambaClone; порт 1420 закреплён за Tauri.
export default defineConfig({
  plugins: [tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: "safari15",
    outDir: "dist",
  },
});
