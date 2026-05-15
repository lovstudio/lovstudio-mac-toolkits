import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { lovinspPlugin } from "lovinsp";
import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";

const host = process.env.TAURI_DEV_HOST ?? "127.0.0.1";

export default defineConfig({
  clearScreen: false,
  plugins: [lovinspPlugin({ bundler: "vite" }), react(), tailwindcss()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  server: {
    host,
    port: 5141,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
