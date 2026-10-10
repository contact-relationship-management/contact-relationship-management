// Accessibility gate (CRM-T37): axe-core with the WCAG 2.0/2.1/2.2 A and AA
// rule tags over the public pages and the list pages. Violations fail the
// build; anything excluded below carries a written reason.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

const WCAG_TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"];

async function signIn(page: Page) {
  await page.context().addCookies([
    {
      name: "__Host-mxi_session",
      value: "a11y-test-session",
      domain: "localhost",
      path: "/",
      httpOnly: true,
      secure: true,
      sameSite: "Lax",
    },
  ]);
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

test.describe("public pages", () => {
  for (const path of ["/", "/tour", "/signin"]) {
    test(`${path} has no WCAG A/AA violations`, async ({ page }) => {
      await page.goto(path);
      await page.waitForLoadState("networkidle");
      expect(summarize((await scan(page)).violations)).toEqual([]);
    });
  }
});

test.describe("signed-in list pages", () => {
  test.beforeEach(async ({ page }) => {
    await signIn(page);
    // Empty lists are enough to exercise the page chrome, headings, tables
    // and controls; data-heavy states are covered by the functional suite.
    await page.route("**/api/proxy/**", (route) => route.fulfill({ json: [] }));
  });

  for (const path of ["/contacts", "/accounts", "/leads", "/tickets", "/articles", "/campaigns"]) {
    test(`${path} has no WCAG A/AA violations`, async ({ page }) => {
      await page.goto(path);
      await page.waitForLoadState("networkidle");
      expect(summarize((await scan(page)).violations)).toEqual([]);
    });
  }
});
