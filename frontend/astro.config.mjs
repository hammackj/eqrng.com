import { defineConfig } from "astro/config";
import tailwind from "@astrojs/tailwind";

export default defineConfig({
  integrations: [tailwind()],
  outDir: "../dist",
  publicDir: "../public",
  server: {
    port: 4321,
    host: true,
  },
  vite: {
    server: {
      proxy: {
        "/random_zone": "http://localhost:3000",
        "/random_race": "http://localhost:3000",
        "/random_class": "http://localhost:3000",
        "/version": "http://localhost:3000",
        "/zones": "http://localhost:3000",
      },
    },
  },
});
