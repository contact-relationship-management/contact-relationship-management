// How long the tour's pages take to load in a browser (CRM-T262), against the production build
// served by `vite preview` on this machine, with the BFF's demo data: no service, no network beyond
// loopback. Each page is loaded three times cold (a new browser context, so no cache) and once warm
// (a reload), plain and throttled to a mid-range phone (4x slower CPU, 9 Mbit/s down, 1.5 up, 170 ms
// round trip), and the median is kept. These are readings from one machine, not limits.

import { writeFileSync } from "node:fs";
import { expect, test, type Browser, type Page } from "@playwright/test";

const PAGES = [
  "/",
  "/contacts",
  "/contacts/con-1",
  "/accounts",
  "/leads",
  "/leads/board",
  "/deals",
  "/campaigns",
  "/tickets",
  "/executive",
  "/search?q=a",
];
const RUNS = 3;

interface Reading {
  ttfb: number;
  domContentLoaded: number;
  load: number;
  lcp: number;
  requests: number;
  scriptKiB: number;
}

const PROFILES = {
  plain: null,
  phone: {
    cpu: 4,
    down: (9 * 1024 * 1024) / 8,
    up: (1.5 * 1024 * 1024) / 8,
    latency: 170,
  },
} as const;

async function throttle(
  page: Page,
  profile: (typeof PROFILES)[keyof typeof PROFILES],
) {
  if (!profile) return;
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Emulation.setCPUThrottlingRate", { rate: profile.cpu });
  await cdp.send("Network.enable");
  await cdp.send("Network.emulateNetworkConditions", {
    offline: false,
    latency: profile.latency,
    downloadThroughput: profile.down,
    uploadThroughput: profile.up,
  });
}

/** The tour's session cookie, made once and given to every fresh context. */
async function tourCookies(browser: Browser) {
  const context = await browser.newContext();
  const page = await context.newPage();
  await page.goto("/signin");
  await page.getByLabel("Email").fill("example@example.com");
  await page.getByRole("button", { name: "Send magic link" }).click();
  await expect(page.getByTestId("demo-banner")).toBeVisible();
  const cookies = await context.cookies();
  await context.close();
  return cookies;
}

async function read(page: Page): Promise<Reading> {
  return page.evaluate(
    () =>
      new Promise<Reading>((resolve) => {
        let lcp = 0;
        new PerformanceObserver((list) => {
          for (const entry of list.getEntries())
            lcp = Math.max(lcp, entry.startTime);
        }).observe({ type: "largest-contentful-paint", buffered: true });
        // LCP settles once the page is idle; give it a moment after `load`.
        setTimeout(() => {
          const [nav] = performance.getEntriesByType(
            "navigation",
          ) as PerformanceNavigationTiming[];
          const resources = performance.getEntriesByType(
            "resource",
          ) as PerformanceResourceTiming[];
          const scripts = resources.filter(
            (r) => r.initiatorType === "script" || r.name.endsWith(".js"),
          );
          resolve({
            ttfb: nav!.responseStart,
            domContentLoaded: nav!.domContentLoadedEventEnd,
            load: nav!.loadEventEnd,
            lcp,
            requests: resources.length + 1,
            scriptKiB:
              scripts.reduce((sum, r) => sum + r.encodedBodySize, 0) / 1024,
          });
        }, 500);
      }),
  );
}

const median = (xs: number[]) =>
  [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)]!;

test("page load times of the tour", async ({ browser }) => {
  const cookies = await tourCookies(browser);
  const results: Record<string, Record<string, Record<string, Reading>>> = {};
  for (const [name, profile] of Object.entries(PROFILES)) {
    results[name] = {};
    for (const path of PAGES) {
      const cold: Reading[] = [];
      let warm: Reading | undefined;
      for (let run = 0; run < RUNS; run++) {
        const context = await browser.newContext();
        await context.addCookies(cookies);
        const page = await context.newPage();
        await throttle(page, profile);
        await page.goto(path, { waitUntil: "load" });
        await expect(page.locator("h1").first()).toBeVisible();
        cold.push(await read(page));
        if (run === RUNS - 1) {
          await page.reload({ waitUntil: "load" });
          warm = await read(page);
        }
        await context.close();
      }
      const pick = (key: keyof Reading) => median(cold.map((r) => r[key]));
      results[name]![path] = {
        cold: {
          ttfb: pick("ttfb"),
          domContentLoaded: pick("domContentLoaded"),
          load: pick("load"),
          lcp: pick("lcp"),
          requests: pick("requests"),
          scriptKiB: pick("scriptKiB"),
        },
        warm: warm!,
      };
    }
  }
  writeFileSync(
    "test-results/page-load.json",
    JSON.stringify(results, null, 2),
  );
  const ms = (x: number) => Math.round(x).toLocaleString("en-GB");
  for (const [name, pages] of Object.entries(results)) {
    console.log(
      `\n${name}: | Page | TTFB | DOM ready | Load | LCP | Warm load | Requests | JS KiB |`,
    );
    for (const [path, { cold, warm }] of Object.entries(pages))
      console.log(
        `| \`${path}\` | ${ms(cold.ttfb)} | ${ms(cold.domContentLoaded)} | ${ms(cold.load)} | ${ms(cold.lcp)} | ${ms(warm.load)} | ${cold.requests} | ${cold.scriptKiB.toFixed(1)} |`,
      );
  }
});
