import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const API_PROXY = {
  target: "http://127.0.0.1:3456",
  changeOrigin: true,
};

// Pull the canonical version from Cargo.toml so the UI never drifts from the
// binary. Cargo.toml is the single source of truth (see AGENTS.md).
function readCargoVersion(): string {
  try {
    const cargoToml = readFileSync(resolve(__dirname, "../Cargo.toml"), "utf8");
    const match = cargoToml.match(/^version\s*=\s*"([^"]+)"/m);
    return match?.[1] ?? "0.0.0";
  } catch {
    return "0.0.0";
  }
}
const APP_VERSION = readCargoVersion();

export default defineConfig({
  plugins: [
    tailwindcss(),
    svelte(),
    {
      name: "local-only",
      configResolved(config) {
        for (const host of [config.server.host, config.preview.host]) {
          if (typeof host !== "string" || !["127.0.0.1", "localhost", "::1"].includes(host)) {
            throw new Error("Local-only policy: Vite must bind to a loopback address");
          }
        }
      },
    },
  ],
  define: {
    __APP_VERSION__: JSON.stringify(APP_VERSION),
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  preview: { host: "127.0.0.1", allowedHosts: [], cors: false },
  server: {
    host: "127.0.0.1",
    allowedHosts: [],
    cors: false,
    // If 5173 is taken, fail fast instead of switching ports (avoids "module load failed"
    // when the browser tab still points at the old URL).
    port: Number(process.env.VITE_PORT ?? 5173),
    strictPort: true,
    proxy: {
      "/api": API_PROXY,
      "/public/api": API_PROXY,
    },
  },
});
