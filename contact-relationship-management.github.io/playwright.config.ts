import { defineConfig, devices } from "@playwright/test";

// The built site, served by `vite preview` (the prerendered files, as GitHub Pages would serve them). Another
// project on a shared machine may hold 4173: run with PW_PORT=4189 (or any free port).
const PORT = Number(process.env.PW_PORT ?? 4173);

export default defineConfig({
  testDir: "tests/e2e",
  timeout: 30_000,
  fullyParallel: true,
  use: { baseURL: `http://localhost:${PORT}`, trace: "on-first-retry" },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: `pnpm run build && pnpm run preview --port ${PORT} --strictPort`,
    url: `http://localhost:${PORT}`,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
