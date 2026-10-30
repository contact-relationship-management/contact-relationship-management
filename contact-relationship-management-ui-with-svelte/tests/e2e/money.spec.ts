// Money in the currency's own minor unit and the reader's own number format (CRM-T167, CRM-T168),
// over a page.route-stubbed API as in the smoke suite: a yen deal shows and edits as yen (no
// hundredths), and a German reader types a decimal comma. Each page is also scanned with axe-core
// for WCAG A/AA, as `a11y.spec.ts` does.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page, type Route } from "@playwright/test";

const WCAG_TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"];

async function signIn(page: Page) {
  await page.context().addCookies([
    {
      name: "__Host-mxi_session",
      value: "money-test-session",
      domain: "localhost",
      path: "/",
      httpOnly: true,
      secure: true,
      sameSite: "Lax",
    },
  ]);
}

const deal = (pid: string, amount_minor: number, currency: string) => ({
  pid,
  name: `Deal in ${currency}`,
  pipeline_pid: "p1",
  stage_pid: "s1",
  amount_minor,
  currency,
  won: false,
  closed_at: null,
  owner_ref: null,
  expected_close_on: null,
});

/** Stub the API: every list empty, the deals given, and record each PUT body to a deal. */
async function stub(page: Page, deals: ReturnType<typeof deal>[]) {
  const puts: Record<string, unknown>[] = [];
  await page.route("**/api/proxy/**", (route: Route) =>
    route.fulfill({ json: [] }),
  );
  await page.route("**/api/proxy/deals/*/external-links", (route: Route) =>
    route.fulfill({ json: { links: [] } }),
  );
  await page.route("**/api/proxy/deals", (route: Route) =>
    route.fulfill({ json: deals }),
  );
  await page.route("**/api/proxy/deals/*", async (route: Route) => {
    const request = route.request();
    if (request.method() !== "PUT") return route.fulfill({ json: [] });
    const body = request.postDataJSON() as Record<string, unknown>;
    puts.push(body);
    const pid = new URL(request.url()).pathname.split("/").pop()!;
    const current = deals.find((d) => d.pid === pid)!;
    return route.fulfill({ json: { ...current, ...body } });
  });
  return puts;
}

async function axeClean(page: Page) {
  await page.waitForLoadState("networkidle");
  const result = await new AxeBuilder({ page }).withTags(WCAG_TAGS).analyze();
  expect(
    result.violations.map((v) => `${v.id}: ${v.nodes.length} — ${v.help}`),
  ).toEqual([]);
}

test.describe("money in its own minor unit and the reader's format", () => {
  test.beforeEach(async ({ page }) => {
    await signIn(page);
  });

  test("a yen deal shows and is edited in whole yen, not hundredths", async ({
    page,
  }) => {
    const puts = await stub(page, [deal("dy", 1500, "JPY")]);
    await page.goto("/deals/dy");
    // 1500 minor units of yen is ¥1,500 (before CRM-T167 it showed ¥15).
    await expect(page.getByRole("main")).toContainText("¥1,500");
    await expect(page.getByRole("main")).not.toContainText("¥15.00");
    await axeClean(page);
    await page.getByTestId("deal-edit").click();
    await expect(page.getByTestId("deal-amount")).toHaveValue("1500");
    await page.getByTestId("deal-amount").fill("2500");
    await page.getByTestId("deal-save").click();
    await expect(page.getByTestId("deal-saved")).toBeVisible();
    // Sent as 2500 minor units, not 250000.
    expect(puts).toEqual([{ amount_minor: 2500 }]);
    await expect(page.getByRole("main")).toContainText("¥2,500");
    await axeClean(page);
  });

  test("a German reader types a decimal comma, and a bad amount is refused in German", async ({
    page,
  }) => {
    const puts = await stub(page, [deal("de", 150_050, "EUR")]);
    await page.goto("/de-de/deals/de");
    await expect(page.getByRole("main")).toContainText("1.500,50 €");
    await page.getByTestId("deal-edit").click();
    await expect(page.getByTestId("deal-amount")).toHaveValue("1500,50");
    await page.getByTestId("deal-amount").fill("1.234,56");
    await page.getByTestId("deal-save").click();
    await expect(page.getByTestId("deal-saved")).toBeVisible();
    expect(puts).toEqual([{ amount_minor: 123_456 }]);
    await expect(page.getByRole("main")).toContainText("1.234,56 €");
    // Text that is not a number in German is refused before anything is sent.
    await page.getByTestId("deal-edit").click();
    await page.getByTestId("deal-amount").fill("zwölf");
    await page.getByTestId("deal-save").click();
    await expect(page.getByTestId("deal-error")).toContainText("Zahlenformat");
    expect(puts).toHaveLength(1);
    await axeClean(page);
  });

  test("a Kuwaiti dinar deal shows three decimal places", async ({ page }) => {
    await stub(page, [deal("dk", 1_234_567, "KWD")]);
    await page.goto("/deals/dk");
    await expect(page.getByRole("main")).toContainText("1,234.567");
    await axeClean(page);
  });
});
