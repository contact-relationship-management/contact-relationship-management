// Dates, numbers and the week in the page's locale and the reader's region (CRM-T172), over a
// page.route-stubbed API as in the smoke suite: a German page shows a rate and a date in German and
// takes a decimal comma; a reader in Egypt on the Arabic page gets a Saturday-first week with a
// Friday–Saturday weekend; a reader in Germany gets Monday and Saturday–Sunday. Each page is also
// scanned with axe-core for WCAG A/AA, as `a11y.spec.ts` does.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page, type Route } from "@playwright/test";

const WCAG_TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"];

async function signIn(page: Page) {
  await page.context().addCookies([
    {
      name: "__Host-mxi_session",
      value: "locale-format-test-session",
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

const rate = {
  pid: "r1",
  from: "EUR",
  to: "GBP",
  rate: "0.8600",
  effective_on: "2026-01-01",
  source: "Board budget rate",
  set_by: "person:x",
};

async function stubFollowups(page: Page) {
  await page.route("**/api/proxy/**", (route: Route) =>
    route.fulfill({ json: [] }),
  );
  await page.route("**/api/proxy/insights/followups*", (route: Route) =>
    route.fulfill({
      json: {
        as_of: "2026-10-30T00:00:00Z",
        note: "",
        overdue: [],
        upcoming_30d: [],
        open_by_recorder: {},
      },
    }),
  );
}

/** The calendar's weekday column headers, in order, and the ones marked as the weekend. */
async function week(page: Page) {
  const calendar = page.getByTestId("followups-calendar");
  await expect(calendar.locator("thead th abbr").first()).toBeVisible();
  const titles = (sel: string) =>
    calendar
      .locator(sel)
      .evaluateAll((els) => els.map((el) => el.getAttribute("title") ?? ""));
  return {
    days: await titles("thead th abbr"),
    weekend: await titles("thead th[data-weekend] abbr"),
  };
}

const weekday = (locale: string, sundayBased: number) =>
  new Intl.DateTimeFormat(locale, { weekday: "long", timeZone: "UTC" }).format(
    new Date(Date.UTC(2024, 0, 7 + sundayBased)),
  );

test.describe("dates, numbers and the week in the reader's locale", () => {
  test.beforeEach(async ({ page }) => {
    await signIn(page);
  });

  test("a German page shows a rate and a date in German and takes a decimal comma", async ({
    page,
  }) => {
    const puts: Record<string, unknown>[] = [];
    await page.route("**/api/proxy/**", (route: Route) =>
      route.fulfill({ json: [] }),
    );
    await page.route("**/api/proxy/fx-rates", async (route: Route) => {
      const request = route.request();
      if (request.method() === "PUT") {
        const body = request.postDataJSON() as Record<string, unknown>;
        puts.push(body);
        return route.fulfill({ json: { ...rate, ...body, pid: "r2" } });
      }
      return route.fulfill({ json: [rate] });
    });
    await page.goto("/de-de/fx-rates");
    const row = page.getByTestId("fx-row").first();
    await expect(row).toContainText("0,86");
    await expect(row).toContainText("01.01.2026");
    await expect(row).not.toContainText("2026-01-01");

    await page.getByTestId("fx-from").fill("EUR");
    await page.getByTestId("fx-to").fill("GBP");
    await page.getByTestId("fx-rate").fill("0,87");
    await page.getByTestId("fx-source").fill("Budget");
    await page.getByTestId("fx-set").click();
    await expect.poll(() => puts.length).toBe(1);
    expect(puts[0]!.rate).toBe("0.87");
    // The day it starts is today in the reader's own time zone.
    expect(puts[0]!.effective_on).toMatch(/^\d{4}-\d{2}-\d{2}$/);

    // Two decimal marks are not a number: refused, in German, before anything is sent.
    // A saved rate clears the box; wait for that before typing the next one.
    await expect(page.getByTestId("fx-rate")).toHaveValue("");
    await page.getByTestId("fx-rate").fill("1,2,3");
    await page.getByTestId("fx-set").click();
    await expect(page.getByTestId("fx-error")).toContainText(
      "Zahlenformat dieser Seite",
    );
    expect(puts.length).toBe(1);
    await axeClean(page);
  });

  test.describe("a reader in Egypt", () => {
    test.use({ locale: "ar-EG" });

    test("gets a Saturday-first week with a Friday–Saturday weekend on the Arabic page", async ({
      page,
    }) => {
      await stubFollowups(page);
      await page.goto("/ar-001/followups");
      const { days, weekend } = await week(page);
      expect(days[0]).toBe(weekday("ar-001", 6));
      // In column order: the week starts on Saturday, so Saturday comes first and Friday last.
      expect(weekend).toEqual([weekday("ar-001", 6), weekday("ar-001", 5)]);
      await axeClean(page);
    });
  });

  test.describe("a reader in Germany", () => {
    test.use({ locale: "de-DE" });

    test("gets a Monday-first week with a Saturday–Sunday weekend", async ({
      page,
    }) => {
      await stubFollowups(page);
      await page.goto("/de-de/followups");
      const { days, weekend } = await week(page);
      expect(days[0]).toBe(weekday("de-de", 1));
      expect(weekend).toEqual([weekday("de-de", 6), weekday("de-de", 0)]);
      await axeClean(page);
    });
  });
});
