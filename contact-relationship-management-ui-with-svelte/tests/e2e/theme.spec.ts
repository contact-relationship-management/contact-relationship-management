// CRM-R96: the theme follows this device unless the person picks one; the choice is kept in the
// `crm-theme` cookie, and the server sends the page in it.
import { expect, test, type Page } from "@playwright/test";

const html = (page: Page) => page.locator("html");

async function choose(page: Page, label: RegExp) {
  await page.locator("header.topbar .theme-picker-button").click();
  await page
    .locator("li.theme-picker-option")
    .filter({ hasText: label })
    .click();
}

async function themeCookie(page: Page): Promise<string | undefined> {
  const cookies = await page.context().cookies();
  return cookies.find((cookie) => cookie.name === "crm-theme")?.value;
}

test.describe("the theme (CRM-R96)", () => {
  test("follows the device's dark setting on a first visit, and keeps nothing", async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: "dark" });
    await page.goto("/signin");
    await expect(html(page)).toHaveAttribute("data-theme", "dark");
    await expect(
      page.locator('link[href="/assets/themes/dark.css"]'),
    ).not.toHaveCount(0);
    expect(await themeCookie(page)).toBeUndefined();
  });

  test("follows the device's light setting on a first visit", async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: "light" });
    await page.goto("/signin");
    await expect(html(page)).toHaveAttribute("data-theme", "light");
  });

  test("offers 'Follow this device' first, chosen until something else is", async ({
    page,
  }) => {
    await page.goto("/signin");
    await page.locator("header.topbar .theme-picker-button").click();
    const first = page.locator("li.theme-picker-option").first();
    await expect(first).toHaveText(/Follow this device/);
    await expect(first).toHaveAttribute("aria-selected", "true");
  });

  test("follows a change of the device's setting while the page is open", async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: "light" });
    await page.goto("/signin");
    await expect(html(page)).toHaveAttribute("data-theme", "light");
    await page.emulateMedia({ colorScheme: "dark" });
    await expect(html(page)).toHaveAttribute("data-theme", "dark");
  });

  test("keeps a picked theme in the cookie, and the server sends the page in it", async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: "light" });
    await page.goto("/signin");
    await choose(page, /^\s*dark\s*$/i);
    await expect(html(page)).toHaveAttribute("data-theme", "dark");
    expect(await themeCookie(page)).toBe("dark");
    const response = await page.reload();
    // The page arrives dark: the server read the cookie, before any script ran.
    expect(await response!.text()).toContain('data-theme="dark"');
    await expect(html(page)).toHaveAttribute("data-theme", "dark");
    // The device's setting no longer applies.
    await page.emulateMedia({ colorScheme: "light" });
    await expect(html(page)).toHaveAttribute("data-theme", "dark");
  });

  test("goes back to following the device when that is chosen again", async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme: "dark" });
    await page.goto("/signin");
    await choose(page, /cupcake/i);
    await expect(html(page)).toHaveAttribute("data-theme", "cupcake");
    await choose(page, /Follow this device/);
    await expect(html(page)).toHaveAttribute("data-theme", "dark");
    expect(await themeCookie(page)).toBe("system");
    await page.emulateMedia({ colorScheme: "light" });
    const response = await page.reload();
    expect(await response!.text()).toContain('data-theme=""');
    await expect(html(page)).toHaveAttribute("data-theme", "light");
  });

  test("leaves one theme stylesheet once the page has started", async ({
    page,
  }) => {
    await page.goto("/signin");
    await expect(html(page)).toHaveAttribute("data-theme", "light");
    await expect(page.locator("link[data-crm-server-theme]")).toHaveCount(0);
    await expect(
      page.locator('link[rel="stylesheet"][href^="/assets/themes/"]'),
    ).toHaveCount(1);
  });

  test("drops the theme kept in storage before the cookie", async ({
    page,
  }) => {
    await page.addInitScript(() => {
      if (!sessionStorage.getItem("seeded")) {
        localStorage.setItem("lily-theme", "dark");
        localStorage.setItem("crm-theme", "dark");
        sessionStorage.setItem("seeded", "1");
      }
    });
    await page.emulateMedia({ colorScheme: "light" });
    await page.goto("/signin");
    await expect(html(page)).toHaveAttribute("data-theme", "light");
    await expect
      .poll(() =>
        page.evaluate(() => [
          localStorage.getItem("lily-theme"),
          localStorage.getItem("crm-theme"),
        ]),
      )
      .toEqual([null, null]);
  });
});
