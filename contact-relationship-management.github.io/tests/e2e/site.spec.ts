// The built site: every page is there, axe finds no WCAG 2.2 A or AA violation under the light and the dark system
// preference, the locale switcher opens the same brief in another locale (slugs differ by script, so by peer id), the
// Arabic pages are right to left and mirror without overflowing, the site's own words follow the page's language,
// and the content security policy blocks nothing the site itself does.
import { test, expect, type Page } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { readdirSync } from "node:fs";
import { BRIEFS, BRIEF_LOCALES, COMPARE_PAGES, DOC_PAGES, ENDONYMS, LOCALE_HOMES, PRODUCTION_LOCALES, peerIn, routesOf } from "../../src/lib/site.ts";
import { TABLES } from "../../src/lib/i18n.ts";

// The themes the site serves. The picker's own list is in a .svelte module, which Playwright cannot import; the test
// of the picker holds its option count to this number.
const THEMES = readdirSync("static/assets/themes")
  .filter((f) => f.endsWith(".css"))
  .map((f) => f.replace(/\.css$/, ""));

const docRoutes = ["/", ...DOC_PAGES.map((d) => `/${d.slug}/`), ...COMPARE_PAGES.map((c) => `/compare/${c}/`)];
const indexRoutes = BRIEF_LOCALES.map((l) => `/${l}/for/`);
const briefRoute = (b: { locale: string; slug: string }) => `/${b.locale}/for/${b.slug}/`;
const briefRoutes = BRIEFS.map(briefRoute);
// One brief per locale, a different role each time.
const sampleBriefs = BRIEF_LOCALES.map((l, i) => BRIEFS.filter((b) => b.locale === l)[i % 11]!);
const homeRoutes = LOCALE_HOMES.map((l) => `/${l}/`);
const productionRoutes = PRODUCTION_LOCALES.map((l) => `/${l}/path-to-production/`);
const audited = [...docRoutes, ...homeRoutes, ...productionRoutes, "/search/", ...indexRoutes, ...sampleBriefs.map(briefRoute)];

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
  const all = [...docRoutes, ...indexRoutes, ...briefRoutes, ...productionRoutes];
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
for (const route of ["/ar-001/", "/ar-001/for/", briefRoute(ceo("ar-001"))])
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
  await expect(page.getByRole("navigation", { name: "Site" })).toContainText("Tutorial");
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

test("a document keeps its address until the reader picks a locale, which opens that locale's briefs", async ({ page }) => {
  await open(page, "/install/", "light");
  await expect(page).toHaveURL(/\/install\/$/);
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await page.getByRole("button", { name: TABLES.en!.language }).click();
  await page.locator(".locale-picker-option").filter({ hasText: /Cymraeg|Welsh/ }).first().click();
  await expect(page).toHaveURL(/\/cy-[^/]+\/$/);
  await expect(page.locator("h1")).toHaveText(TABLES.cy!.brand);
});

test("the home page in every language has its hero, its wordmark and the translations", async ({ page }) => {
  const translations = ["ar-001", "cy-001", "en-001", "hi-001", "zh-001"];
  for (const [route, lang] of [
    ["/", "en"],
    ["/en-us/", "en"],
    ["/cy-gb/", "cy"],
    ["/cy-001/", "cy"],
    ["/zh-cn/", "zh"],
    ["/zh-001/", "zh"],
    ["/ar-001/", "ar"],
    ["/hi-in/", "hi"],
    ["/hi-001/", "hi"],
  ] as const) {
    const words = TABLES[lang]!;
    await page.goto(route);
    await expect(page.locator(".landing-hero h1"), route).toHaveText(words.brand);
    await expect(page.locator(".landing-hero .doc p"), route).toHaveCount(2);
    await expect(page.locator(".brand"), route).toHaveText(words.brand);
    await expect(page.getByRole("link", { name: words["cta.tutorial"] }), route).toHaveAttribute("href", /\/tutorial\/$/);
    await expect(page.locator(".landing h2"), route).toHaveCount(6);
    await expect(page.locator(".landing ul li"), route).toHaveCount(12);
    await expect(page.locator(".landing ol li"), route).toHaveCount(4);
    await expect(page.locator(".landing table tbody tr"), route).toHaveCount(4);
    // Every link in the page's own language stays where it should: the briefs of this locale.
    await expect(page.locator('.landing a[href$="/for/"]'), route).toHaveCount(1);
    const line = page.locator(".translations");
    await expect(line, route).toContainText(words.translations);
    await expect(line, route).toContainText("العربية, Cymraeg, English, हिन्दी, 普通话");
    for (const code of translations) {
      const link = line.getByRole("link", { name: ENDONYMS[code.split("-")[0]!]!, exact: true });
      await expect(link, `${route} ${code}`).toHaveAttribute("href", new RegExp(`/${code}/$`));
    }
  }
});

test("the locale picker names each locale in its own language", async ({ page }) => {
  await open(page, "/", "light");
  await page.locator(".locale-picker-button").click();
  const options = page.locator(".locale-picker-option");
  await expect(options).toHaveCount(BRIEF_LOCALES.length);
  for (const label of ["English (Great Britain)", "Cymraeg (Prydain Fawr)", "普通话 (中国)"])
    await expect(options.filter({ hasText: label })).toHaveCount(1);
});

test("the sitemaps list every page of the site and of a locale", async ({ request }) => {
  const locs = (xml: string) => [...xml.matchAll(/<loc>(.+?)<\/loc>/g)].map((m) => decodeURI(new URL(m[1]!).pathname));
  const all = await request.get("/sitemap.xml");
  expect(all.status()).toBe(200);
  expect(locs(await all.text()).sort()).toEqual(
    [...docRoutes, ...BRIEF_LOCALES.flatMap(routesOf)].sort(),
  );
  for (const locale of ["en-001", "cy-gb", "zh-cn"]) {
    const one = await request.get(`/${locale}/sitemap.xml`);
    expect(one.status(), locale).toBe(200);
    expect(locs(await one.text()), locale).toEqual(routesOf(locale));
  }
});

// The rules' checks: hovering a picker button changes its colour, and its dropdown opens under it without moving the
// page. Fresh state for each, so one picker's popup cannot sit over the next.
for (const picker of ["link", "search", "theme", "locale", "time-zone", "text-size", "share"])
  for (const route of ["/", "/ar-001/for/"])
    test(`the ${picker} picker on ${route} answers a hover and opens under its button`, async ({ page }) => {
      await open(page, route, "light");
      const button = page.locator(`.${picker}-picker-button`);
      await page.mouse.move(0, 0);
      const look = () =>
        button.evaluate((e) => {
          const c = getComputedStyle(e);
          return [c.backgroundColor, c.color, c.borderTopColor].join(" ");
        });
      const before = await look();
      await button.hover();
      await expect.poll(look, { timeout: 2000 }).not.toBe(before);

      const main = await page.locator("main").boundingBox();
      const width = await page.evaluate(() => document.documentElement.scrollWidth);
      await button.click();
      const popup = page.locator(`.${picker}-picker-${picker === "search" ? "panel" : "list"}`);
      await expect(popup).toBeVisible();
      const b = (await button.boundingBox())!;
      const p = (await popup.boundingBox())!;
      expect(p.y, "under the button").toBeGreaterThanOrEqual(b.y + b.height - 1);
      expect(p.x, "inside the viewport").toBeGreaterThanOrEqual(0);
      expect(p.x + p.width, "inside the viewport").toBeLessThanOrEqual(page.viewportSize()!.width);
      expect(await page.locator("main").boundingBox(), "the page did not move").toEqual(main);
      expect(await page.evaluate(() => document.documentElement.scrollWidth), "no sideways growth").toBe(width);
    });

test("the path to production shows each readiness estimate as a disabled Lily slider with its percentage", async ({ page }) => {
  expect(PRODUCTION_LOCALES).toContain("en-us");
  await page.goto("/en-us/path-to-production/");
  await expect(page.locator("h1")).toHaveText("Path to production");
  const items = page.locator('[data-testid="readiness"] > li');
  const count = await items.count();
  expect(count).toBeGreaterThan(5);
  for (let i = 0; i < count; i++) {
    const item = items.nth(i);
    const slider = item.getByRole("slider");
    await expect(slider).toBeDisabled();
    const name = (await item.locator("strong").first().textContent())?.trim() ?? "";
    await expect(slider).toHaveAccessibleName(`${name}: estimated readiness`);
    const value = Number(await slider.inputValue());
    expect(value, name).toBeGreaterThanOrEqual(0);
    expect(value, name).toBeLessThanOrEqual(100);
    await expect(slider).toHaveAttribute("aria-valuetext", `${value}%`);
    await expect(item.locator("output")).toHaveText(`${value}%`);
  }
  // Every locale's page has the same estimates, in the same order, labelled in its own language.
  const percents = async () =>
    page.locator('[data-testid="readiness"] input[type="range"]').evaluateAll((els) => els.map((e) => (e as HTMLInputElement).value));
  const english = await percents();
  expect(PRODUCTION_LOCALES.length).toBe(11);
  for (const locale of PRODUCTION_LOCALES) {
    await page.goto(`/${locale}/path-to-production/`);
    expect(await percents(), locale).toEqual(english);
    await expect(page.locator('[data-testid="readiness"] input[type="range"]:disabled'), locale).toHaveCount(english.length);
  }
  for (const locale of LOCALE_HOMES) {
    await page.goto(`/${locale}/`);
    await expect(page.locator('main a[href*="/path-to-production/"]'), `${locale} home links the page`).not.toHaveCount(0);
  }
  // The README's banner links here.
  await page.goto("/overview/");
  await expect(page.locator('main a[href$="/en-us/path-to-production/"]').first()).toBeVisible();
});

test("the comparison index links every product page, and each is fair and sourced", async ({ page }) => {
  await page.goto("/compare/");
  for (const product of COMPARE_PAGES)
    await expect(page.locator(`main a[href$="/compare/${product}/"]`), product).toHaveCount(1);
  for (const product of COMPARE_PAGES) {
    await page.goto(`/compare/${product}/`);
    await expect(page.locator("main h2").filter({ hasText: /is stronger$/ }), product).toHaveCount(1);
    await expect(page.locator("main h2").filter({ hasText: "Sources" }), product).toHaveCount(1);
    await expect(page.locator("main").getByText("not ready for production"), product).toBeVisible();
  }
});

test("the header carries the full picker bar, on a document and on a brief", async ({ page }) => {
  for (const [route, words] of [
    ["/spec/", TABLES.en!],
    [briefRoute(ceo("cy-gb")), TABLES.cy!],
  ] as const) {
    await open(page, encodeURI(route), "light");
    const bar = page.locator(".picker-bar");
    for (const name of [words.pages, words.search, words.theme, words.timeZone, words.textSize, words.share])
      await expect(bar.getByRole("button", { name, exact: true }), `${route} ${name}`).toBeVisible();
    await expect(bar.locator(".locale-picker-button")).toBeVisible();
  }
});

test("the theme picker offers every Lily theme and applies one", async ({ page }) => {
  await open(page, "/", "light");
  await page.getByRole("button", { name: "Theme", exact: true }).click();
  await expect(page.locator(".theme-picker-option")).toHaveCount(THEMES.length);
  await page.locator(".theme-picker-option").filter({ hasText: /^Nord$/ }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "nord");
});

test("the link picker opens the site's pages and the repository in a new tab", async ({ page }) => {
  await open(page, "/", "light");
  await page.getByRole("button", { name: "Pages", exact: true }).click();
  await expect(page.locator(".link-picker-link").filter({ hasText: "Source repository" })).toHaveAttribute(
    "target",
    "_blank",
  );
  await page.locator(".link-picker-link").filter({ hasText: "Install" }).click();
  await expect(page).toHaveURL(/\/install\/$/);
});

test("the search button finds pages in the browser, in any script", async ({ page }) => {
  const csp: string[] = [];
  page.on("console", (m) => {
    if (/Content Security Policy/i.test(m.text())) csp.push(m.text());
  });
  await open(page, "/", "light");
  const find = async (words: string) => {
    await page.getByRole("button", { name: "Search", exact: true }).first().click();
    await page.getByRole("searchbox", { name: "Search terms" }).fill(words);
    await page.keyboard.press("Enter");
  };
  await find("family bridges");
  await expect(page).toHaveURL(/\/search\/\?family%20bridges$/);
  await expect(page.locator("h1")).toContainText("family bridges");
  await expect(page.locator(".results a").first()).toHaveAttribute("href", /family-bridges/);

  await find("zzzz-no-such-word");
  await expect(page.getByRole("status")).toHaveText(TABLES.en!["search.none"]);

  // A native-script word, and the same page reached by its address.
  const title = "الرئيس التنفيذي";
  await page.goto(`/search/?${encodeURIComponent(title)}`);
  const first = page.locator(".results li").first();
  await expect(first).toHaveAttribute("dir", "rtl");
  await expect(first.locator("a")).toHaveAttribute("href", /ar-001/);
  expect(csp).toEqual([]);
});

test("the share picker offers its destinations and copies the link", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await open(page, "/install/", "light");
  await page.getByRole("button", { name: "Share", exact: true }).click();
  await expect(page.locator(".share-picker-target")).toHaveCount(5);
  await page.locator(".share-picker-copy").click();
  await expect(page.locator(".share-picker-status")).toHaveText(TABLES.en!["share.copied"]);
  expect(await page.evaluate(() => navigator.clipboard.readText())).toMatch(/\/install\/$/);
});

test("the time-zone picker lists zones and records the choice on <html>", async ({ page }) => {
  await open(page, "/", "light");
  await page.getByRole("button", { name: "Time zone", exact: true }).click();
  await page.locator(".time-zone-picker-option").filter({ hasText: /Japan/ }).first().click();
  await expect(page.locator("html")).toHaveAttribute("data-time-zone", "Asia/Tokyo");
});

// Every theme the picker offers, over the header and the running text of one document.
for (const theme of THEMES)
  test(`the ${theme} theme passes axe`, async ({ page }) => {
    await page.addInitScript((t) => localStorage.setItem("lily-theme", t), theme);
    await page.goto("/install/", { waitUntil: "networkidle" });
    await page.waitForFunction(
      (t) =>
        document.documentElement.dataset.theme === t &&
        [...document.styleSheets].some((s) => (s.href ?? "").endsWith(`/assets/themes/${t}.css`)),
      theme,
    );
    expect(await axe(page), theme).toEqual([]);
  });

test("links between the site's own documents stay on the site; others go to the repository", async ({ page }) => {
  await page.goto("/overview/");
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
  for (const route of [...docRoutes, "/en-gb/for/", "/ar-001/", "/hi-in/", briefRoute(ceo("hi-in")), briefRoute(ceo("ar-001"))]) {
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
