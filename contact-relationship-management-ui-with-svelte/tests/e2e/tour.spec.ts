// The guided tour: sign in as example@example.com and every screen shows
// fictitious data from the BFF's demo layer (no service, no stubs).

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

const SCREENS = [
  "/",
  "/contacts",
  "/contacts/con-1",
  "/accounts",
  "/leads",
  "/leads/board",
  "/deals",
  "/campaigns",
  "/tickets",
  "/tickets/board",
  "/articles",
  "/followups",
  "/executive",
  "/dpo",
  "/break-glass",
  "/break-glass/review",
  "/engagement",
  "/partners",
];

async function startTour(page: Page) {
  await page.goto("/signin");
  await expect(page.getByTestId("demo-hint")).toContainText("example@example.com");
  await page.getByLabel("Email").fill("example@example.com");
  await page.getByRole("button", { name: "Send magic link" }).click();
  await expect(page.getByTestId("demo-banner")).toBeVisible();
  await expect(
    page.getByRole("heading", { level: 1, name: "My Home Page" }),
  ).toBeVisible();
  const header = page.getByRole("banner");
  await expect(header.getByText("Sign out", { exact: true })).toBeVisible();
  await expect(header.getByText("Sign in", { exact: true })).toHaveCount(0);
}

test("a visitor can take the tour from the tour page", async ({ page }) => {
  await page.goto("/tour");
  await page.getByTestId("tour-signin").getByRole("link", { name: "Sign in" }).click();
  await expect(page).toHaveURL(/\/signin$/);
});

test("signing in as example@example.com shows fictitious data on every screen", async ({
  page,
}) => {
  const problems: string[] = [];
  page.on("pageerror", (e) => problems.push(`pageerror ${e.message}`));
  page.on("response", (r) => {
    if (r.url().includes("/api/proxy/") && r.status() >= 400) {
      problems.push(`${r.status()} ${r.url()}`);
    }
  });
  await startTour(page);

  for (const path of SCREENS) {
    await page.goto(path);
    await page.waitForLoadState("networkidle");
    await expect(page.getByTestId("demo-banner"), path).toBeVisible();
    await expect(page.getByTestId("error"), path).toHaveCount(0);
    await expect(page.locator("main h1").first(), path).toBeVisible();
  }
  expect(problems).toEqual([]);

  // Real-looking content, not empty states.
  await page.goto("/contacts");
  await expect(page.getByText("Imogen Hartley")).toBeVisible();
  await page.goto("/deals");
  await expect(page.getByText("Smart-meter analytics rollout")).toBeVisible();
});

test("another address is not the tour", async ({ page }) => {
  await page.goto("/signin");
  await page.getByLabel("Email").fill("someone@example.org");
  await page.getByRole("button", { name: "Send magic link" }).click();
  // The real flow asks the (absent) authentication service and says it failed.
  await expect(page.getByRole("alert")).toBeVisible();
  await expect(page.getByTestId("demo-banner")).toHaveCount(0);
});

test("signing out leaves the tour", async ({ page }) => {
  await startTour(page);
  await page.getByRole("button", { name: "Sign out" }).click();
  // The sign-out POST redirects home; wait for it before leaving the page.
  await expect(
    page.getByRole("banner").getByRole("link", { name: "Sign in" }),
  ).toBeVisible();
  await expect(page.getByTestId("demo-banner")).toHaveCount(0);
  await page.goto("/contacts");
  await expect(page).toHaveURL(/\/signin$/);
});

// Data-filled screens were never scanned before (the gate in a11y.spec.ts uses
// empty lists), and they have findings, mostly in the third-party grid and
// filter components and in colour contrast. This scan does not fail the build;
// it reports each finding as a test annotation so the list is visible, and the
// findings are tracked under the manual accessibility audit (CRM-T48).
test.describe("tour screens: WCAG A/AA findings with data (informational)", () => {
  for (const path of SCREENS) {
    test(path, async ({ page }, info) => {
      await startTour(page);
      await page.goto(path);
      await page.waitForLoadState("networkidle");
      const { violations } = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
        .analyze();
      for (const v of violations) {
        info.annotations.push({
          type: "a11y-finding",
          description: `${path}: ${v.id} (${v.impact}): ${v.nodes.length} node(s)`,
        });
      }
    });
  }
});
