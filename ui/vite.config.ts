import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import tailwindcss from "@tailwindcss/vite";

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
  },
  esbuild: {
    target: "es2022",
    legalComments: "none",
    drop: ["console", "debugger"],
  },
});
