import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  worker: { format: "es" },
  // Tauri serves the same build; a fixed port lets it find the dev server.
  server: { port: 5173, strictPort: true },
});
