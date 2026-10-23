import { fileURLToPath, URL } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

export default defineConfig({
  plugins: [solid(), tailwindcss()],
  resolve: {
    // `@/components/ui/button` instead of `../../components/ui/button` (shadcn convention).
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  server: {
    port: 5173,
    // In dev, Vite serves the UI with hot reload and forwards /api to Axum.
    proxy: { "/api": "http://localhost:3000" },
  },
  build: { target: "es2022" },
});
