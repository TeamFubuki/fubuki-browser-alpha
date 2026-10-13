import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import previewData from "./dev/preview.json" with { type: "json" };

export default defineConfig({
  plugins: [
    solid(),
    {
      name: "internal-pages-preview-data",
      configureServer(server) {
        server.middlewares.use((request, response, next) => {
          if (request.method !== "GET" || request.url !== "/data.json") return next();
          response.setHeader("Content-Type", "application/json; charset=utf-8");
          response.end(JSON.stringify(previewData));
        });
      },
    },
  ],
  base: "./",
  build: { target: "es2022", outDir: "dist", modulePreload: { polyfill: false } },
});
