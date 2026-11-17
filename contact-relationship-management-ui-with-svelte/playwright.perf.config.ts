import { defineConfig, devices } from "@playwright/test";

// Page-load benchmark (CRM-T262): not a test suite and not a gate. It loads each page of the tour,
// cold and warm, plain and throttled, and writes the figures to test-results/page-load.json and a
// table to the console. Run `pnpm perf:pages`; BENCHMARKS.md records the results.
export default defineConfig({
  testDir: "tests/perf",
  timeout: 600_000,
  fullyParallel: false,
  workers: 1,
  reporter: [["line"]],
  use: { baseURL: "http://localhost:4374" },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "npm run build && npm run preview -- --port 4374 --strictPort",
    url: "http://localhost:4374",
    reuseExistingServer: false,
    timeout: 180_000,
  },
});
