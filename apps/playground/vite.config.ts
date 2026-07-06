import { defineConfig } from "vite";

export default defineConfig({
  server: {
    // Allow serving files from workspace-linked packages outside this app's
    // own directory (packages/runtime-dom, packages/signals).
    fs: {
      allow: [".."],
    },
  },
});
