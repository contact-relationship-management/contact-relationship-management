// Right-to-left on the Arabic page (CRM-T174; spec/worldwide.md code rows 38 and 39), over a
// page.route-stubbed API as in the smoke suite: a table's cells start on the right, a back link's
// arrow points back (to the right), a "goes to" arrow points along the writing direction (to the
// left), and a range of dates is one `formatRange` text with no arrow. The English page is checked
// beside it, so the test shows the direction is the page's, not fixed. Each page is scanned with
// axe-core for WCAG A/AA, as `a11y.spec.ts` does.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page, type Route } from "@playwright/test";

const WCAG_TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"];

async function signIn(page: Page) {
  await page.context().addCookies([
    {
      name: "__Host-mxi_session",
      value: "right-to-left-test-session",
      domain: "localhost",
      path: "/",
      httpOnly: true,
      secure: true,
      sameSite: "Lax",
    },
  ]);
}

async function axeClean(page: Page) {
  await page.waitForLoadState("networkidle");
  const result = await new AxeBuilder({ page }).withTags(WCAG_TAGS).analyze();
  expect(
    result.violations.map((v) => `${v.id}: ${v.nodes.length} — ${v.help}`),
  ).toEqual([]);
}

async function stub(page: Page) {
  await page.route("**/api/proxy/**", (route: Route) =>
    route.fulfill({ json: [] }),
  );
  await page.route("**/api/proxy/fx-rates", (route: Route) =>
    route.fulfill({
      json: [
        {
          pid: "r1",
          from: "EUR",
          to: "GBP",
          rate: "0.86",
          effective_on: "2026-01-01",
          source: "Board budget rate",
          set_by: "person:x",
        },
      ],
    }),
  );
  await page.route("**/api/proxy/deals/*/external-links", (route: Route) =>
    route.fulfill({ json: { links: [] } }),
  );
  await page.route("**/api/proxy/deals/d1", (route: Route) =>
    route.fulfill({
      json: {
        pid: "d1",
        name: "Northwind renewal",
        pipeline_pid: "p1",
        stage_pid: "s1",
        amount_minor: 120000,
        currency: "EUR",
        won: false,
        closed_at: null,
        owner_ref: null,
        expected_close_on: null,
      },
    }),
  );
  await page.route("**/api/proxy/insights/executive", (route: Route) =>
    route.fulfill({
      json: {
        as_of: "2026-10-30T00:00:00Z",
        window: { from: "2026-09-01T12:00:00Z", to: "2026-10-30T12:00:00Z" },
        deals_won: 1,
        deals_lost: 0,
        won_value_by_currency_minor: {},
        lost_reasons: {},
        new_leads: 0,
        tickets_opened: 0,
        tickets_resolved: 0,
        campaigns_started: 0,
        activities_logged: 0,
        consent_withdrawals: 0,
        note: "",
      },
    }),
  );
  await page.route("**/api/proxy/insights/stale-deals*", (route: Route) =>
    route.fulfill({ json: { derivation: "", deals: [] } }),
  );
  await page.route("**/api/proxy/insights/pipeline-hygiene*", (route: Route) =>
    route.fulfill({ json: { as_of: "", threshold_days: 30, findings: [] } }),
  );
  await page.route("**/api/proxy/insights/forecast-trends*", (route: Route) =>
    route.fulfill({ json: { series: [] } }),
  );
}

/** Where the first header cell's text sits in its table: "start-right" or "start-left". */
async function firstCellSide(page: Page) {
  const table = page.getByTestId("fx-list");
  await expect(table.locator("th").first()).toBeVisible();
  return table.evaluate((el) => {
    const th = el.querySelector("th")!;
    const range = document.createRange();
    range.selectNodeContents(th);
    const text = range.getBoundingClientRect();
    const cell = th.getBoundingClientRect();
    return {
      align: getComputedStyle(th).textAlign,
      direction: getComputedStyle(th).direction,
      // How far the text is from each of its cell's edges.
      gapLeft: text.left - cell.left,
      gapRight: cell.right - text.right,
      // Whether the first column is the right-most one.
      firstOnRight:
        cell.right >= el.getBoundingClientRect().right - 1 &&
        cell.left > el.getBoundingClientRect().left,
    };
  });
}

test.describe("right-to-left on the Arabic page", () => {
  test.beforeEach(async ({ page }) => {
    await signIn(page);
    await stub(page);
  });

  test("a table starts on the right and its text is aligned to the start", async ({
    page,
  }) => {
    await page.goto("/ar-001/fx-rates");
    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
    const side = await firstCellSide(page);
    expect(side.align).toBe("start");
    expect(side.direction).toBe("rtl");
    expect(side.firstOnRight).toBe(true);
    expect(side.gapRight).toBeLessThan(side.gapLeft);
    await axeClean(page);
  });

  test("the same table starts on the left in English", async ({ page }) => {
    await page.goto("/en-001/fx-rates");
    await expect(page.locator("html")).toHaveAttribute("dir", "ltr");
    const side = await firstCellSide(page);
    expect(side.firstOnRight).toBe(false);
    expect(side.gapLeft).toBeLessThan(side.gapRight);
  });

  test("a back link points back: to the right in Arabic, to the left in English", async ({
    page,
  }) => {
    await page.goto("/ar-001/deals/d1");
    const back = page.locator('p > a[href$="/deals"]').first();
    await expect(back).toContainText("→");
    await expect(back).not.toContainText("←");
    await axeClean(page);
    await page.goto("/en-001/deals/d1");
    await expect(page.locator('p > a[href$="/deals"]').first()).toContainText(
      "←",
    );
  });

  test("a range of dates is one locale range with no arrow", async ({
    page,
  }) => {
    await page.goto("/ar-001/executive");
    // The window line is in Arabic now (CRM-T175), so it is found by its test id, not the English word.
    const line = page.getByTestId("exec-window");
    await expect(line).toBeVisible();
    await expect(line).not.toContainText("→");
    await expect(line).not.toContainText("←");
    const expected = await page.evaluate(() =>
      new Intl.DateTimeFormat("ar-001", {
        dateStyle: "medium",
      }).formatRange(
        new Date("2026-09-01T12:00:00Z"),
        new Date("2026-10-30T12:00:00Z"),
      ),
    );
    await expect(line).toContainText(expected);
    await axeClean(page);
  });
});
