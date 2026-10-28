// Accessibility gate (CRM-T37, CRM-R77): axe-core with the WCAG 2.0/2.1/2.2 A and AA rule tags over
// **every page**, the signed-in ones in the tour with its sample data (so tables, forms and
// records are on the page, not empty lists). The pages and how to reach them are in
// `a11y-routes.ts`, which a unit test holds to the file tree. Violations fail the build; a page
// left out carries a written reason there.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

import { ROUTES } from "./a11y-routes";

const WCAG_TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"];

/** Sign in to the tour, as the sign-in page's own hint says to. */
async function startTour(page: Page) {
  await page.goto("/signin");
  await page.getByLabel("Email").fill("example@example.com");
  await page.getByRole("button", { name: "Send magic link" }).click();
  await expect(page.getByTestId("demo-banner")).toBeVisible();
}

async function scan(page: Page) {
  return new AxeBuilder({ page }).withTags(WCAG_TAGS).analyze();
}

function summarize(
  violations: Awaited<ReturnType<typeof scan>>["violations"],
): string[] {
  return violations.map(
    (v) => `${v.id} (${v.impact}): ${v.nodes.length} node(s) — ${v.help}`,
  );
}

for (const { route, path, mode, excluded } of ROUTES) {
  const title = `${route} (${path}) has no WCAG A/AA violations`;
  if (excluded !== undefined) {
    test.skip(`${title}: ${excluded}`, () => {});
    continue;
  }
  test(title, async ({ page }) => {
    if (mode === "tour") await startTour(page);
    await page.goto(path);
    await page.waitForLoadState("networkidle");
    expect(summarize((await scan(page)).violations)).toEqual([]);
  });
}
