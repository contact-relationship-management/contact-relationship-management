// docs/privacy-notice.md: nothing is kept in the browser until the person makes a choice, and a choice
// keeps only the item the notice names for it.
import { expect, test, type Page } from "@playwright/test";

async function kept(page: Page) {
  await page.waitForLoadState("networkidle");
  const storage = await page.evaluate(() => Object.keys(localStorage).sort());
  const cookies = (await page.context().cookies()).map((c) => c.name).sort();
  return { storage, cookies };
}

async function pick(page: Page, picker: string, option: RegExp) {
  await page.locator(`header.topbar .${picker}-picker-button`).click();
  await page
    .locator(`li.${picker}-picker-option`)
    .filter({ hasText: option })
    .first()
    .click();
}

test.describe("what the UI keeps in the browser (docs/privacy-notice.md)", () => {
  test("a first visit keeps nothing", async ({ page }) => {
    await page.goto("/signin");
    await expect(
      page.locator("header.topbar .theme-picker-button"),
    ).toBeVisible();
    expect(await kept(page)).toEqual({ storage: [], cookies: [] });
  });

  test("a page whose address names a language shows it and keeps nothing", async ({
    page,
  }) => {
    await page.goto("/de-de/signin");
    await expect(page.locator("html")).toHaveAttribute("lang", /^de/);
    expect(await kept(page)).toEqual({ storage: [], cookies: [] });
  });

  test("a text size that is picked is kept, and comes back", async ({
    page,
  }) => {
    await page.goto("/signin");
    const before = await page.locator("html").getAttribute("data-text-size");
    expect(before).toBeTruthy();
    await pick(page, "text-size", /large/i);
    const size = await page.locator("html").getAttribute("data-text-size");
    expect(size).not.toBe(before);
    expect(await kept(page)).toEqual({
      storage: ["lily-text-size"],
      cookies: [],
    });
    await page.reload();
    await expect(page.locator("html")).toHaveAttribute("data-text-size", size!);
  });

  test("a language that is picked is kept", async ({ page }) => {
    await page.goto("/signin");
    await pick(page, "locale", /Deutsch/);
    await expect(page.locator("html")).toHaveAttribute("lang", /^de/);
    expect(await kept(page)).toEqual({
      storage: ["mxi.crm.locale"],
      cookies: [],
    });
  });

  test("a theme that is picked is kept in its cookie alone", async ({
    page,
  }) => {
    await page.goto("/signin");
    await pick(page, "theme", /^\s*dark\s*$/i);
    expect(await kept(page)).toEqual({ storage: [], cookies: ["crm-theme"] });
  });
});
