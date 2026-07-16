import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import tailwindcss from "@tailwindcss/vite";
import { resolve } from "node:path";

export default defineConfig({
  plugins: [tailwindcss(), solid()],
  base: "/",
  build: {
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
    cssMinify: "lightningcss",
    cssCodeSplit: false,
    modulePreload: {
      polyfill: false,
    },
    rollupOptions: {
      input: {
        app: resolve(import.meta.dirname, "index.html"),
        internal: resolve(import.meta.dirname, "internal.html"),
      },
    },
  },
  esbuild: {
    target: "es2022",
    legalComments: "none",
    drop: ["console", "debugger"],
  },
});
