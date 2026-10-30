// The built site: every page is there, axe finds no WCAG 2.2 A or AA violation under the light and the dark system
// preference, the locale switcher opens the same brief in another locale (slugs differ by script, so by peer id), the
// Arabic pages are right to left and mirror without overflowing, the site's own words follow the page's language,
// and the content security policy blocks nothing the site itself does.
import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { BRIEFS, BRIEF_LOCALES, DOC_PAGES, peerIn } from "../../src/lib/site.ts";
import { TABLES } from "../../src/lib/i18n.ts";

const docRoutes = DOC_PAGES.map((d) => (d.slug ? `/${d.slug}/` : "/"));
const indexRoutes = BRIEF_LOCALES.map((l) => `/${l}/for/`);
const briefRoute = (b: { locale: string; slug: string }) => `/${b.locale}/for/${b.slug}/`;
const briefRoutes = BRIEFS.map(briefRoute);
// One brief per locale, a different role each time.
const sampleBriefs = BRIEF_LOCALES.map((l, i) => BRIEFS.filter((b) => b.locale === l)[i % 11]!);
const audited = [...docRoutes, ...indexRoutes, ...sampleBriefs.map(briefRoute)];

const ceo = (locale: string) => peerIn(locale, BRIEFS.find((b) => b.slug === "chief-executive-officer")!.peer)!;

/** Load a page under a colour scheme and wait until ThemePicker has applied the matching Lily theme. */
async function open(page: Page, route: string, scheme: "light" | "dark") {
  await page.emulateMedia({ colorScheme: scheme });
  await page.goto(route, { waitUntil: "networkidle" });
  await page.waitForFunction(
    (t) =>
      document.documentElement.dataset.theme === t &&
      [...document.styleSheets].some((s) => (s.href ?? "").endsWith(`/assets/themes/${t}.css`)),
    scheme,
  );
}

async function axe(page: Page): Promise<string[]> {
  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
    .analyze();
  return results.violations.map(
    (v) => `${v.id} (${v.impact}): ${v.nodes.slice(0, 3).map((n) => n.target.join(" ")).join("; ")}`,
  );
}

test("every page is built and answers 200", async ({ request }) => {
  const all = [...docRoutes, ...indexRoutes, ...briefRoutes];
  expect(BRIEF_LOCALES.length).toBe(11);
  expect(BRIEFS.length).toBe(11 * 11);
  for (const route of all) {
    const res = await request.get(encodeURI(route));
    expect(res.status(), route).toBe(200);
  }
});

for (const route of audited)
  for (const scheme of ["light", "dark"] as const)
    test(`${route} passes axe (${scheme})`, async ({ page }) => {
      const csp: string[] = [];
      page.on("console", (m) => {
        if (/Content Security Policy/i.test(m.text())) csp.push(m.text());
      });
      await open(page, route, scheme);
      await expect(page.locator("h1").first()).toBeVisible();
      expect(await axe(page)).toEqual([]);
      expect(csp).toEqual([]);
    });

test("a brief's html carries its locale; a document is English", async ({ page }) => {
  await page.goto(briefRoute(ceo("cy-gb")));
  await expect(page.locator("html")).toHaveAttribute("lang", "cy-GB");
  await expect(page.locator("html")).toHaveAttribute("dir", "ltr");
  await page.goto("/install/");
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.locator("html")).toHaveAttribute("dir", "ltr");
});

// One test per page and scheme: ThemePicker stores the theme it applied, so a second visit in the same browser
// context would keep the first scheme's theme.
for (const route of ["/ar-001/for/", briefRoute(ceo("ar-001"))])
  for (const scheme of ["light", "dark"] as const)
    test(`${route} is right to left, mirrored, its own words Arabic, and passes axe (${scheme})`, async ({ page }) => {
      await open(page, route, scheme);
      await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
      await expect(page.locator("html")).toHaveAttribute("lang", "ar-001");
      await expect(page.locator("header")).toHaveAttribute("dir", "rtl");
      await expect(page.getByRole("navigation")).toContainText(TABLES.ar!["nav.briefs"]);
      // The brand sits at the inline start: on the right.
      const brand = await page.locator(".brand").boundingBox();
      const width = page.viewportSize()?.width ?? 0;
      expect(brand && brand.x + brand.width, route).toBeGreaterThan(width / 2);
      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
      );
      expect(overflow, route).toBeLessThanOrEqual(0);
      expect(await axe(page), `${route} ${scheme}`).toEqual([]);
    });

test("the site's own words follow the brief's language, by language", async ({ page }) => {
  for (const [locale, lang] of [
    ["cy-gb", "cy"],
    ["cy-001", "cy"],
    ["hi-in", "hi"],
    ["zh-001", "zh"],
    ["en-us", "en"],
  ] as const) {
    await page.goto(`/${locale}/for/`);
    await expect(page.locator("h1")).toHaveText(TABLES[lang]!["nav.briefs"]);
    await expect(page.locator(".skip")).toHaveText(TABLES[lang]!.skip);
  }
  await page.goto("/spec/");
  await expect(page.getByRole("navigation", { name: "Site" })).toContainText("Specification");
});

test("the locale switcher opens the same brief in the chosen locale, across scripts", async ({ page }) => {
  await open(page, briefRoute(ceo("en-gb")), "light");
  await page.getByRole("button", { name: TABLES.en!.briefLanguage }).click();
  await page.locator(".locale-picker-option").filter({ hasText: /العربية|Arabic/ }).first().click();
  await expect(page).toHaveURL(new RegExp(`/ar-001/for/${encodeURI(ceo("ar-001").slug)}/$`));
  await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
  await page.getByRole("button", { name: TABLES.ar!.briefLanguage }).click();
  await page.locator(".locale-picker-option").filter({ hasText: /Cymraeg|Welsh/ }).first().click();
  await expect(page).toHaveURL(/\/cy-(gb|001)\/for\/[^/]+\/$/);
  await expect(page.locator("html")).toHaveAttribute("dir", "ltr");
  await expect(page.locator("h1").first()).toContainText(/Prif Weithredwr|Prif Swyddog Gweithredol/);
});

test("the locale switcher is only on locale pages", async ({ page }) => {
  await open(page, "/install/", "light");
  await expect(page.getByRole("button", { name: TABLES.en!.briefLanguage })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Theme" })).toBeVisible();
});

test("links between the site's own documents stay on the site; others go to the repository", async ({ page }) => {
  await page.goto("/");
  const install = page.locator("main a[href$='install/']").first();
  await expect(install).toHaveCount(1);
  const regulatory = page.locator("main a", { hasText: "spec/regulatory.md" }).first();
  await expect(regulatory).toHaveAttribute(
    "href",
    "https://github.com/contact-relationship-management/contact-relationship-management/blob/main/spec/regulatory.md",
  );
});

test("at phone width no page scrolls sideways", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  for (const route of [...docRoutes, "/en-gb/for/", briefRoute(ceo("hi-in")), briefRoute(ceo("ar-001"))]) {
    await page.goto(encodeURI(route));
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
    );
    expect(overflow, route).toBeLessThanOrEqual(0);
  }
});

test("no element carries a style attribute (the CSP allows only SvelteKit's announcer)", async ({ page }) => {
  for (const route of ["/", "/spec/", briefRoute(ceo("cy-001")), briefRoute(ceo("ar-001"))]) {
    await open(page, encodeURI(route), "dark");
    const styled = await page.evaluate(() =>
      [...document.querySelectorAll("[style]")]
        .filter((e) => e.id !== "svelte-announcer")
        .map((e) => e.outerHTML.slice(0, 120)),
    );
    expect(styled, route).toEqual([]);
  }
});
