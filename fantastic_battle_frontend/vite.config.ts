import { defineConfig } from "vite";

export default defineConfig({
  base: "./",
  build: {
    target: "es2020",
  },
  server: {
    proxy: {
      "/api": "http://localhost:8080",
    },
  },
});
