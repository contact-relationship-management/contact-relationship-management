// CRM-R96: the UI uses the browser's light/dark setting, unless the person picks a theme.
import { expect, test } from "@playwright/test";

const theme = (page: import("@playwright/test").Page) =>
  page.locator("html").getAttribute("data-theme");

test.describe("the theme (CRM-R96)", () => {
  test("follows the browser's dark setting on a first visit", async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: "dark" });
    await page.goto("/signin");
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
    await expect(
      page.locator('link[href="/assets/themes/dark.css"]'),
    ).toHaveCount(1);
  });

  test("follows the browser's light setting on a first visit", async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: "light" });
    await page.goto("/signin");
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  });

  test("follows a change of the setting while the page is open, and on the next visit", async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: "light" });
    await page.goto("/signin");
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
    await page.emulateMedia({ colorScheme: "dark" });
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
    // A theme the browser chose is not remembered as a pick.
    await page.emulateMedia({ colorScheme: "light" });
    await page.reload();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
    expect(
      await page.evaluate(() => localStorage.getItem("crm-theme")),
    ).toBeNull();
  });

  test("keeps a theme the person picks, whatever the browser's setting", async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: "dark" });
    await page.goto("/signin");
    await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
    await page.locator("header.topbar .theme-picker-button").click();
    await page
      .locator("li.theme-picker-option")
      .filter({ hasText: /^\s*cupcake\s*$/i })
      .click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "cupcake");
    await page.emulateMedia({ colorScheme: "light" });
    await page.reload();
    expect(await theme(page)).toBe("cupcake");
    await expect(page.locator("html")).toHaveAttribute("data-theme", "cupcake");
  });

  test("drops the old saved theme, which may have been the browser's", async ({
    page,
  }) => {
    await page.addInitScript(() => {
      if (!sessionStorage.getItem("seeded")) {
        localStorage.setItem("lily-theme", "dark");
        sessionStorage.setItem("seeded", "1");
      }
    });
    await page.emulateMedia({ colorScheme: "light" });
    await page.goto("/signin");
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
    expect(
      await page.evaluate(() => localStorage.getItem("lily-theme")),
    ).toBeNull();
  });
});
