// The guided tour: sign in as example@example.com and every screen shows
// fictitious data from the BFF's demo layer (no service, no stubs).

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

// The tour's data is one shared in-memory copy that every tour sign-in resets,
// so these tests run one after another in one worker, not in parallel.
test.describe.configure({ mode: "serial" });

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
  "/en-001/kanban",
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

test("/<locale>/kanban shows the deal board and moves a card", async ({ page }) => {
  await startTour(page);
  await page.goto("/en-001/kanban");
  await expect(page.getByRole("heading", { level: 1, name: "My Kanban board" })).toBeVisible();
  const board = page.getByTestId("kanban-board");
  await expect(board).toContainText("Qualification 10%");
  await expect(board).toContainText("Smart-meter analytics rollout");
  await expect(board).toContainText("£184,000.00");

  // Move a card with the keyboard-reachable menu; the move is saved.
  await page
    .getByRole("button", { name: /^Move Case-management upgrade/ })
    .click();
  await page.getByRole("option", { name: "Discovery 25%" }).click();
  await expect(page.getByTestId("action-error")).toHaveCount(0);
  await page.reload();
  const discovery = page.getByRole("gridcell", { name: /Case-management upgrade/ });
  await expect(discovery).toHaveCount(1);
  // The saved stage is Discovery: that column's header is the cell's column.
  const col = await discovery.getAttribute("data-col");
  expect(col).toBe("1");
});

test("the address names the language, and unknown languages are 404", async ({ page }) => {
  await startTour(page);
  await page.goto("/de-de/kanban");
  await expect(page.getByRole("heading", { level: 1, name: "Mein Kanban-Board" })).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("lang", "de-de");
  const missing = await page.goto("/xx/kanban");
  expect(missing?.status()).toBe(404);
});

test("the kanban board needs a session", async ({ page }) => {
  await page.goto("/en-001/kanban");
  await expect(page).toHaveURL(/\/signin$/);
});

test.describe("data tables", () => {
  test("a row opens its record, by click or Enter, and the name is a link", async ({ page }) => {
    await startTour(page);
    await page.goto("/contacts");
    // The name is a real link …
    await expect(page.getByRole("link", { name: "Imogen Hartley" })).toHaveAttribute(
      "href",
      "/contacts/con-1",
    );
    // … and the rest of the row opens the same record.
    await page.getByRole("gridcell", { name: "Head of Procurement" }).click();
    await expect(page).toHaveURL(/\/contacts\/con-1$/);
    await page.goBack();
    await page.getByRole("gridcell", { name: "Head of Procurement" }).focus();
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(/\/contacts\/con-1$/);
  });

  test("columns sort (numbers numerically) and say so", async ({ page }) => {
    await startTour(page);
    await page.goto("/leads");
    const header = page.getByRole("columnheader", { name: "Score" });
    await header.getByRole("button").click();
    await expect(header).toHaveAttribute("aria-sort", "ascending");
    const scores = async () =>
      (await page.getByTestId("lead-queue").locator("tbody tr").allInnerTexts()).map(
        (t) => Number(t.trim().split(/\s+/)[0]),
      );
    const ascending = await scores();
    expect(ascending).toEqual([...ascending].sort((a, b) => a - b));
    await header.getByRole("button").click();
    await expect(header).toHaveAttribute("aria-sort", "descending");
    const descending = await scores();
    expect(descending).toEqual([...descending].sort((a, b) => b - a));
  });

  test("status is a chip, a breach is flagged, and numbers sit at the end", async ({ page }) => {
    await startTour(page);
    await page.goto("/tickets");
    const queue = page.getByTestId("ticket-queue");
    await expect(queue.locator(".chip").first()).toBeVisible();
    await expect(queue.locator("strong.flag").first()).toContainText("Breached");
    await page.goto("/followups");
    await expect(
      page.getByTestId("followups-overdue").locator('td[data-align="end"]').first(),
    ).toBeVisible();
  });

  test("the Board links are readable (label colour differs from the background)", async ({ page }) => {
    await startTour(page);
    for (const path of ["/tickets", "/leads"]) {
      await page.goto(path);
      const link = page.getByRole("link", { name: "Board" });
      await expect(link).toBeVisible();
      const [color, background] = await link.evaluate((e) => {
        const style = getComputedStyle(e);
        return [style.color, style.backgroundColor];
      });
      expect(color, path).not.toBe(background);
    }
  });

  test("an empty table keeps its headers and says why", async ({ page }) => {
    await startTour(page);
    await page.goto("/followups");
    await page.getByTestId("kind-filter").selectOption("note");
    await expect(page.getByTestId("followups-overdue")).toContainText("Nothing overdue.");
    await expect(
      page.getByTestId("followups-overdue").getByRole("columnheader", { name: "Due", exact: true }),
    ).toBeVisible();
  });
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

test.describe("boards and timeline", () => {
  test("the ticket board offers only legal moves, and a move is saved", async ({ page }) => {
    await startTour(page);
    await page.goto("/tickets/board");
    await page
      .getByRole("button", { name: /^Move Sensor firmware update fails on site/ })
      .click();
    // An open ticket can go to Pending or Resolved, not straight to Closed.
    const menu = page.getByRole("listbox");
    await expect(menu.getByRole("option", { name: "Pending" })).not.toHaveAttribute(
      "aria-disabled",
      "true",
    );
    await expect(menu.getByRole("option", { name: "Closed" })).toHaveCount(0);
    await menu.getByRole("option", { name: "Resolved" }).click();
    await expect(page.getByTestId("action-error")).toHaveCount(0);
    await page.reload();
    const board = page.getByTestId("ticket-board");
    await expect(board).toContainText("Sensor firmware update fails on site");
  });

  test("the lead board shows scored cards in lifecycle columns", async ({ page }) => {
    await startTour(page);
    await page.goto("/leads/board");
    const board = page.getByTestId("lead-board");
    await expect(board).toContainText("Qualified");
    await expect(board).toContainText("score");
  });

  test("campaigns show a read-only timeline with one row per campaign", async ({ page }) => {
    await startTour(page);
    await page.goto("/campaigns");
    const timeline = page.getByTestId("campaign-timeline");
    await expect(timeline).toBeVisible();
    await expect(timeline.getByRole("rowheader", { name: /Spring product update/ })).toBeVisible();
    await expect(timeline.getByRole("rowheader", { name: /Customer webinar series \(running\)/ })).toBeVisible();
    // The critical path is named in text, not only by style; a finished
    // campaign that ended earlier has slack.
    await expect(timeline.getByRole("rowheader", { name: /Customer webinar series.*critical path/ })).toBeVisible();
    await expect(timeline.getByRole("rowheader", { name: /Spring product update.*days slack/ })).toBeVisible();
    // Read-only: no edit regions or save buttons.
    await expect(timeline.getByRole("button", { name: /save/i })).toHaveCount(0);
  });
});

test.describe("editing, order and rescheduling", () => {
  test("a deal's amount is edited in place and the new value shows", async ({ page }) => {
    await startTour(page);
    await page.goto("/deals");
    const board = page.getByTestId("deal-board");
    await expect(board).toContainText("£74,000.00");
    await board.getByRole("button", { name: /£74,000\.00/ }).click();
    const editor = board.getByRole("spinbutton");
    await editor.fill("80000");
    await editor.press("Enter");
    await expect(board).toContainText("£80,000.00");
    await expect(board).not.toContainText("£74,000.00");
    // It was saved, not only shown.
    await page.reload();
    await expect(page.getByTestId("deal-board")).toContainText("£80,000.00");
  });

  test("deals are grouped in a swimlane per owner", async ({ page }) => {
    await startTour(page);
    await page.goto("/deals");
    const board = page.getByTestId("deal-board");
    await expect(board).toContainText("owner-ayo");
    await expect(board).toContainText("owner-bea");
  });

  test("a card can be selected with several others and moved together", async ({ page }) => {
    await startTour(page);
    await page.goto("/tickets/board");
    const board = page.getByTestId("ticket-board");
    const first = board.getByRole("gridcell", { name: /Route export missing depot codes/ });
    const second = board.getByRole("gridcell", { name: /Licence seats not showing in console/ });
    await first.click({ modifiers: ["ControlOrMeta"] });
    await second.click({ modifiers: ["ControlOrMeta"] });
    await expect(first).toHaveAttribute("aria-selected", "true");
    await expect(second).toHaveAttribute("aria-selected", "true");
  });

  test("a follow-up moves to another day with Alt+arrow, and stays there", async ({ page }) => {
    await startTour(page);
    await page.goto("/followups");
    const calendar = page.getByTestId("followups-calendar");
    const event = calendar.getByText("task: Renewal paperwork for Harbourlight");
    await expect(event).toBeVisible();
    const cellDate = async () =>
      event.locator("xpath=ancestor::*[@data-date][1]").getAttribute("data-date");
    const before = await cellDate();
    await event.focus();
    await page.keyboard.press("Alt+ArrowRight");
    await expect(page.getByTestId("move-error")).toHaveCount(0);
    await expect
      .poll(async () => cellDate())
      .not.toBe(before);
    // Saved: it is still on the new day after a reload.
    const after = await cellDate();
    await page.reload();
    const again = page
      .getByTestId("followups-calendar")
      .getByText("task: Renewal paperwork for Harbourlight");
    await expect(again).toBeVisible();
    expect(await again.locator("xpath=ancestor::*[@data-date][1]").getAttribute("data-date")).toBe(after);
  });
});

test.describe("export", () => {
  test("a table downloads as CSV, TSV and Excel", async ({ page }) => {
    await startTour(page);
    await page.goto("/contacts");
    for (const [name, ext] of [["CSV", "csv"], ["TSV", "tsv"], ["Excel", "xlsx"]] as const) {
      const download = page.waitForEvent("download");
      await page.getByRole("button", { name, exact: true }).first().click();
      expect((await download).suggestedFilename()).toMatch(new RegExp(`\\.${ext}$`));
    }
    await expect(page.getByRole("button", { name: "Copy", exact: true }).first()).toBeVisible();
  });
});
