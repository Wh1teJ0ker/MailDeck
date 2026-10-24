import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri 固定端口,避免 devUrl 漂移
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5180,
    strictPort: true,
  },
  build: {
    target: "safari15",
  },
});
