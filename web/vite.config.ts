import { fileURLToPath, URL } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import solid from "vite-plugin-solid";
import { defineConfig } from "vitest/config";

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
  // `npm test`: Vitest reuses this config, so the "@" alias and Solid JSX just work.
  test: { environment: "happy-dom" },
});
