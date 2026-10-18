import { defineConfig, devices } from "@playwright/test";

// Live tests drive the UI against a RUNNING service (not the tour's fictitious
// data). Start the service on port 5160 first, then `pnpm test:live`. They
// create their own records and leave them behind; use a development database.
const API = process.env.CRM_API_URL ?? "http://localhost:5160";
export default defineConfig({
  testDir: "tests/live",
  timeout: 60_000,
  workers: 1,
  use: { baseURL: "http://localhost:4174" },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "npm run build && npm run preview -- --port 4174 --strictPort",
    url: "http://localhost:4174",
    reuseExistingServer: false,
    timeout: 180_000,
    // The BFF proxy appends the client path (`/tickets`) to this base, so it ends in `/api`.
    env: { CRM_API_URL: `${API}/api`, AUTH_API_URL: API, CRM_DEMO_TOUR: "off" },
  },
});
