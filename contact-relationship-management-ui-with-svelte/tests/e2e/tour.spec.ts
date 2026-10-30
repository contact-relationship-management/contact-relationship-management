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
  "/help",
  "/followups",
  "/notifications",
  "/leads/import",
  "/executive",
  "/dpo",
  "/break-glass",
  "/break-glass/review",
  "/engagement",
  "/partners",
  "/en-001/kanban",
  "/search?q=a",
  "/de-de/contacts",
];

async function startTour(page: Page) {
  await page.goto("/signin");
  await expect(page.getByTestId("demo-hint")).toContainText(
    "example@example.com",
  );
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
  await page
    .getByTestId("tour-signin")
    .getByRole("link", { name: "Sign in" })
    .click();
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
  await expect(
    page.getByText("Smart-meter analytics rollout").first(),
  ).toBeVisible();
});

test("/<locale>/kanban shows the deal board and moves a card", async ({
  page,
}) => {
  await startTour(page);
  await page.goto("/en-001/kanban");
  await expect(
    page.getByRole("heading", { level: 1, name: "My Kanban board" }),
  ).toBeVisible();
  const board = page.getByTestId("kanban-board");
  await expect(board).toContainText("Qualification 10%");
  await expect(board).toContainText("Smart-meter analytics rollout");
  await expect(board).toContainText("£184,000.00");
  // Each deal in its own currency's unit: whole yen, and dinars to three places.
  await expect(board).toContainText("¥14,500,000");
  await expect(board).toContainText("KWD 9,600.500");

  // Move a card with the keyboard-reachable menu; the move is saved.
  await page
    .getByRole("button", { name: /^Move Case-management upgrade/ })
    .click();
  await page.getByRole("option", { name: "Discovery 25%" }).click();
  await expect(page.getByTestId("action-error")).toHaveCount(0);
  await page.reload();
  const discovery = page.getByRole("gridcell", {
    name: /Case-management upgrade/,
  });
  await expect(discovery).toHaveCount(1);
  // The saved stage is Discovery: that column's header is the cell's column.
  const col = await discovery.getAttribute("data-col");
  expect(col).toBe("1");
});

test("the address names the language, and unknown languages are 404", async ({
  page,
}) => {
  await startTour(page);
  await page.goto("/de-de/kanban");
  await expect(
    page.getByRole("heading", { level: 1, name: "Mein Kanban-Board" }),
  ).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("lang", "de-de");
  const missing = await page.goto("/xx/kanban");
  expect(missing?.status()).toBe(404);
});

test("search finds part of a name written without spaces, and one ideograph is enough", async ({
  page,
}) => {
  await startTour(page);
  // 芳 is the given name in 王芳; the service searches such scripts by character pairs (CRM-T170).
  await page.goto("/search?q=" + encodeURIComponent("芳"));
  await expect(page.getByTestId("search-nav.contacts")).toContainText("王芳");
  await page.goto("/search?q=" + encodeURIComponent("वेंकट"));
  await expect(page.getByTestId("search-nav.contacts")).toContainText(
    "प्रिया वेंकटरमन",
  );
});

test("the kanban board needs a session", async ({ page }) => {
  await page.goto("/en-001/kanban");
  await expect(page).toHaveURL(/\/signin$/);
});

test.describe("data tables", () => {
  test("a row opens its record, by click or Enter, and the name is a link", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/contacts");
    // The name is a real link …
    await expect(
      page.getByRole("link", { name: "Imogen Hartley" }),
    ).toHaveAttribute("href", "/contacts/con-1");
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
      (
        await page.getByTestId("lead-queue").locator("tbody tr").allInnerTexts()
      ).map((t) => Number(t.trim().split(/\s+/)[0]));
    const ascending = await scores();
    expect(ascending).toEqual([...ascending].sort((a, b) => a - b));
    await header.getByRole("button").click();
    await expect(header).toHaveAttribute("aria-sort", "descending");
    const descending = await scores();
    expect(descending).toEqual([...descending].sort((a, b) => b - a));
  });

  test("status is a chip, a breach is flagged, and numbers sit at the end", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/tickets");
    const queue = page.getByTestId("ticket-queue");
    await expect(queue.locator(".chip").first()).toBeVisible();
    await expect(queue.locator("strong.flag").first()).toContainText(
      "Breached",
    );
    await page.goto("/followups");
    await expect(
      page
        .getByTestId("followups-overdue")
        .locator('td[data-align="end"]')
        .first(),
    ).toBeVisible();
  });

  test("a ticket waiting for the customer shows that its SLA clock is stopped", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/tickets");
    const queue = page.getByTestId("ticket-queue");
    const row = queue.locator("tbody tr", { hasText: "Licence seats" });
    await expect(row).not.toContainText("SLA clock stopped");
    await row.click();
    await page.getByRole("button", { name: "→ pending" }).click();
    await expect(row).toContainText("SLA clock stopped");
    // Leaving `pending` starts it again.
    await row.click();
    await page.getByRole("button", { name: "→ open" }).click();
    await expect(row).not.toContainText("SLA clock stopped");
    // A customer reply does the same, in one step.
    await row.click();
    await page.getByRole("button", { name: "→ pending" }).click();
    await expect(row).toContainText("SLA clock stopped");
    await row.click();
    await page.getByTestId("ticket-replied").click();
    await expect(row).not.toContainText("SLA clock stopped");
    await row.click();
    await expect(page.getByTestId("ticket-replied")).toHaveCount(0);
    // The reference a mailed reply carries in its subject is shown for the selected ticket.
    await expect(page.getByTestId("ticket-reference")).toContainText("[T-");
  });

  test("a ticket left waiting too long is marked, and the customer's reply clears it", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/tickets");
    const row = page
      .getByTestId("ticket-queue")
      .locator("tbody tr", { hasText: "Clinical feed delayed overnight" });
    await expect(row).toContainText("Waiting too long");
    await row.click();
    await page.getByTestId("ticket-replied").click();
    await expect(row).not.toContainText("Waiting too long");
  });

  test("the inbox lists notifications, marks one read, then all", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/notifications");
    const list = page.getByTestId("notifications");
    await expect(list.locator("li")).toHaveCount(2);
    await expect(list).toContainText("Task reminder");
    await expect(list).toContainText("SLA breach");
    await list.getByRole("button", { name: "Mark as read" }).first().click();
    await expect(
      list.getByRole("button", { name: "Mark as read" }),
    ).toHaveCount(1);
    await page.getByTestId("mark-all").click();
    await expect(page.getByTestId("mark-all")).toBeDisabled();
    await page.getByTestId("unread-only").check();
    await expect(page.getByTestId("notifications-empty")).toBeVisible();
  });

  test("a CSV is mapped, reported row by row, imported and undone", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/leads/import");
    await page.getByTestId("file").setInputFiles({
      name: "leads.csv",
      mimeType: "text/csv",
      buffer: Buffer.from("Name,Email\nNew Person,n@x.test\n,nobody@x.test\n"),
    });
    await expect(page.getByTestId("map-display_name")).toHaveValue("Name");
    await page.getByTestId("check").click();
    await expect(page.getByTestId("summary")).toContainText("1");
    await expect(page.getByTestId("import-report")).toContainText(
      "has no name",
    );
    await page.getByTestId("source").fill("Trade show");
    await page.getByTestId("commit").click();
    await expect(page.getByTestId("done")).toContainText("1");
    await page.getByTestId("undo").click();
    await expect(page.getByTestId("undone")).toBeVisible();
  });

  test("a saved view narrows a list, can be saved, and a shared one is offered", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/tickets");
    const rows = page.getByTestId("ticket-queue").locator("tbody tr");
    await expect(rows.first()).toBeVisible();
    const all = await rows.count();
    await page.getByTestId("view-select").selectOption("view-1");
    await expect.poll(async () => rows.count()).toBeLessThan(all);
    await page.getByTestId("view-select").selectOption("");
    await expect.poll(async () => rows.count()).toBe(all);

    await page.getByTestId("view-new").click();
    await page.getByTestId("view-name").fill("Closed ones");
    await page.getByTestId("view-editor").getByLabel("closed").check();
    await page.getByTestId("view-create").click();
    await expect(page.getByTestId("view-select")).toContainText("Closed ones");
    await expect(page.getByTestId("view-delete")).toBeVisible();
    await page.getByTestId("view-delete").click();
    await expect(page.getByTestId("view-select")).not.toContainText(
      "Closed ones",
    );
  });

  test("the Board links are readable (label colour differs from the background)", async ({
    page,
  }) => {
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
    await expect(page.getByTestId("followups-overdue")).toContainText(
      "Nothing overdue.",
    );
    await expect(
      page
        .getByTestId("followups-overdue")
        .getByRole("columnheader", { name: "Due", exact: true }),
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
  test("the ticket board offers only legal moves, and a move is saved", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/tickets/board");
    await page
      .getByRole("button", {
        name: /^Move Sensor firmware update fails on site/,
      })
      .click();
    // An open ticket can go to Pending or Resolved, not straight to Closed.
    const menu = page.getByRole("listbox");
    await expect(
      menu.getByRole("option", { name: "Pending" }),
    ).not.toHaveAttribute("aria-disabled", "true");
    await expect(menu.getByRole("option", { name: "Closed" })).toHaveCount(0);
    await menu.getByRole("option", { name: "Resolved" }).click();
    await expect(page.getByTestId("action-error")).toHaveCount(0);
    await page.reload();
    const board = page.getByTestId("ticket-board");
    await expect(board).toContainText("Sensor firmware update fails on site");
  });

  test("the lead board shows scored cards in lifecycle columns", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/leads/board");
    const board = page.getByTestId("lead-board");
    await expect(board).toContainText("Qualified");
    await expect(board).toContainText("score");
  });

  test("campaigns show a read-only timeline with one row per campaign", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/campaigns");
    const timeline = page.getByTestId("campaign-timeline");
    await expect(timeline).toBeVisible();
    await expect(
      timeline.getByRole("rowheader", { name: /Spring product update/ }),
    ).toBeVisible();
    await expect(
      timeline.getByRole("rowheader", {
        name: /Customer webinar series \(running\)/,
      }),
    ).toBeVisible();
    // The critical path is named in text, not only by style; a finished
    // campaign that ended earlier has slack.
    await expect(
      timeline.getByRole("rowheader", {
        name: /Customer webinar series.*critical path/,
      }),
    ).toBeVisible();
    await expect(
      timeline.getByRole("rowheader", {
        name: /Spring product update.*days slack/,
      }),
    ).toBeVisible();
    // Read-only: no edit regions or save buttons.
    await expect(timeline.getByRole("button", { name: /save/i })).toHaveCount(
      0,
    );
  });
});

test.describe("editing, order and rescheduling", () => {
  test("a deal's amount is edited in place and the new value shows", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/deals");
    const board = page.getByTestId("deal-board");
    await expect(board).toContainText("£74,000.00");
    await board.getByRole("button", { name: /£74,000\.00/ }).click();
    const editor = board.getByRole("textbox", { name: "Amount" });
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

  test("a card can be selected with several others and moved together", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/tickets/board");
    const board = page.getByTestId("ticket-board");
    const first = board.getByRole("gridcell", {
      name: /Route export missing depot codes/,
    });
    const second = board.getByRole("gridcell", {
      name: /Licence seats not showing in console/,
    });
    await first.click({ modifiers: ["ControlOrMeta"] });
    await second.click({ modifiers: ["ControlOrMeta"] });
    await expect(first).toHaveAttribute("aria-selected", "true");
    await expect(second).toHaveAttribute("aria-selected", "true");
  });

  test("a follow-up moves to another day with Alt+arrow, and stays there", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/followups");
    const calendar = page.getByTestId("followups-calendar");
    const event = calendar.getByText(
      "task: Renewal paperwork for Harbourlight",
    );
    await expect(event).toBeVisible();
    const cellDate = async () =>
      event
        .locator("xpath=ancestor::*[@data-date][1]")
        .getAttribute("data-date");
    const before = await cellDate();
    await event.focus();
    await page.keyboard.press("Alt+ArrowRight");
    await expect(page.getByTestId("move-error")).toHaveCount(0);
    await expect.poll(async () => cellDate()).not.toBe(before);
    // Saved: it is still on the new day after a reload.
    const after = await cellDate();
    await page.reload();
    const again = page
      .getByTestId("followups-calendar")
      .getByText("task: Renewal paperwork for Harbourlight");
    await expect(again).toBeVisible();
    expect(
      await again
        .locator("xpath=ancestor::*[@data-date][1]")
        .getAttribute("data-date"),
    ).toBe(after);
  });
});

test.describe("export, undo and search", () => {
  test("a table downloads as CSV, TSV and Excel", async ({ page }) => {
    await startTour(page);
    await page.goto("/contacts");
    for (const [name, ext] of [
      ["CSV", "csv"],
      ["TSV", "tsv"],
      ["Excel", "xlsx"],
    ] as const) {
      const download = page.waitForEvent("download");
      await page.getByRole("button", { name, exact: true }).first().click();
      expect((await download).suggestedFilename()).toMatch(
        new RegExp(`\\.${ext}$`),
      );
    }
    await expect(
      page.getByRole("button", { name: "Copy", exact: true }).first(),
    ).toBeVisible();
  });

  test("an edit on the deal board is undone with Ctrl+Z", async ({ page }) => {
    await startTour(page);
    await page.goto("/deals");
    const board = page.getByTestId("deal-board");
    await board.getByRole("button", { name: /£74,000\.00/ }).click();
    const editor = board.getByRole("textbox", { name: "Amount" });
    await editor.fill("81000");
    await editor.press("Enter");
    await expect(board).toContainText("£81,000.00");
    await board.getByRole("gridcell").first().focus();
    await page.keyboard.press("Control+z");
    await expect(board).toContainText("£74,000.00");
    await expect(board).not.toContainText("£81,000.00");
  });

  test("the follow-up calendar is searchable and marks weekends", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/followups");
    const calendar = page.getByTestId("followups-calendar");
    await expect(calendar.locator("[data-weekend]").first()).toBeVisible();
    const search = calendar.getByRole("searchbox");
    await search.fill("zzzz-no-such-follow-up");
    await expect(calendar.locator(".calendar-event")).toHaveCount(0);
  });

  test("the campaign timeline downloads as CSV, TSV and Excel", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/campaigns");
    const timeline = page.getByTestId("campaign-timeline");
    for (const [name, ext] of [
      ["CSV", "csv"],
      ["TSV", "tsv"],
      ["Excel", "xlsx"],
    ] as const) {
      const download = page.waitForEvent("download");
      await timeline.getByRole("button", { name, exact: true }).click();
      const file = await download;
      expect(file.suggestedFilename()).toBe(`campaign-timeline.${ext}`);
    }
  });

  test("any page opens under a language in the address, and keeps it", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/de-de/contacts");
    await expect(page.getByRole("heading", { level: 1 })).toHaveText(
      "Kontakte",
    );
    await expect(page).toHaveURL(/\/de-de\/contacts$/);
    // Links keep the language; the language picker rewrites the address.
    await page.locator("header.topbar .link-picker-button").click();
    await page.getByRole("link", { name: "Konten" }).click();
    await expect(page).toHaveURL(/\/de-de\/accounts$/);
    await page.goto("/fr-001");
    await expect(page).toHaveURL(/\/fr-001$/);
    await expect(page.getByTestId("demo-banner")).toBeVisible();
  });

  test("an unauthenticated visit to a prefixed private page goes to sign-in", async ({
    page,
  }) => {
    await page.goto("/de-de/contacts");
    await expect(page).toHaveURL(/\/signin$/);
  });

  test("a ticket's title is edited in place on the board and saved", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/tickets/board");
    const board = page.getByTestId("ticket-board");
    const first = board.locator(".kanban-board-card-title").first();
    const before = (await first.innerText()).split("\n")[0]!.trim();
    await first.dblclick();
    const box = board.getByRole("textbox").first();
    await box.fill("Renamed in place");
    await box.press("Enter");
    await expect(board).toContainText("Renamed in place");
    await page.reload();
    await expect(page.getByTestId("ticket-board")).toContainText(
      "Renamed in place",
    );
    await expect(page.getByTestId("ticket-board")).not.toContainText(before);
  });

  test("a contact page offers likely duplicates, in the chosen language, and they link through", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/contacts/con-1");
    const list = page.getByTestId("duplicates");
    await expect(list).toContainText("Imogen Hartly");
    await expect(list).toContainText("Likely the same person");
    // Rounded down: a likely match is never shown as certain.
    await expect(page.getByTestId("duplicate-probability")).toHaveText("99%");
    await expect(list).toContainText(
      "Agrees: Given name, Family name, Employer.",
    );

    await list.getByRole("link", { name: "Imogen Hartly" }).click();
    await expect(
      page.getByRole("heading", { level: 1, name: "Imogen Hartly" }),
    ).toBeVisible();
    await expect(page.getByTestId("duplicates")).toContainText(
      "Imogen Hartley",
    );

    // A contact with none says so.
    await page.goto("/contacts/con-5");
    await expect(page.getByTestId("duplicates-none")).toHaveText(
      "No likely duplicates.",
    );

    // The labels follow the language.
    await page.goto("/de-de/contacts/con-1");
    await expect(
      page.getByRole("heading", { level: 2, name: "Mögliche Duplikate" }),
    ).toBeVisible();
    await expect(page.getByTestId("duplicates")).toContainText(
      "Wahrscheinlich dieselbe Person",
    );
  });
  test("a duplicate can be merged into this contact, after a preview, with a reason", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/contacts/con-1");
    await page.getByTestId("merge").click();
    const panel = page.getByTestId("merge-panel");
    await expect(panel).toContainText("Imogen Hartly");
    // The preview names what moves and what consent becomes; nothing has changed.
    await expect(page.getByTestId("merge-moves")).toContainText("Will move");
    await expect(page.getByTestId("merge-consent")).toContainText(
      "Marketing consent afterwards",
    );
    // A reason is required.
    await expect(page.getByTestId("merge-confirm")).toBeDisabled();
    await page.getByTestId("merge-cancel").click();
    await expect(panel).toHaveCount(0);
    await expect(page.getByTestId("duplicates")).toContainText("Imogen Hartly");

    await page.getByTestId("merge").click();
    await page.getByTestId("merge-reason").fill("Same person, two spellings");
    await page.getByTestId("merge-confirm").click();
    await expect(page.getByTestId("merge-done")).toContainText("Merged.");
    await expect(page.getByTestId("duplicates-none")).toBeVisible();

    // The merged contact is gone from the list.
    await page.goto("/contacts");
    await expect(page.getByText("Imogen Hartly")).toHaveCount(0);
  });

  test("the merge panel follows the language", async ({ page }) => {
    await startTour(page);
    await page.goto("/de-de/contacts/con-1");
    await page.getByTestId("merge").click();
    await expect(page.getByTestId("merge-moves")).toContainText(
      "Wird verschoben",
    );
    await expect(page.getByTestId("merge-confirm")).toHaveText(
      "Zusammenführen",
    );
  });

  test("a merge can be undone from the survivor's page, after a preview, with a reason", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/contacts/con-1");
    await expect(page.getByTestId("merges")).toHaveCount(0);
    await page.getByTestId("merge").click();
    await page.getByTestId("merge-reason").fill("Same person");
    await page.getByTestId("merge-confirm").click();
    await expect(page.getByTestId("merge-done")).toBeVisible();

    // The merged contact is listed, with an Undo.
    await expect(page.getByTestId("merged-entry")).toContainText(
      "Imogen Hartly",
    );
    await page.getByTestId("unmerge").click();
    await expect(page.getByTestId("unmerge-restores")).toContainText(
      "Will go back",
    );
    await expect(page.getByTestId("unmerge-consent")).toContainText(
      "Marketing consent for the returning contact",
    );
    await expect(page.getByTestId("unmerge-confirm")).toBeDisabled();
    await page.getByTestId("unmerge-reason").fill("Wrong person");
    await page.getByTestId("unmerge-confirm").click();
    await expect(page.getByTestId("unmerge-done")).toContainText(
      "Merge undone.",
    );
    await expect(page.getByTestId("merged-entry")).toHaveCount(0);

    // The contact is back in the list and is again a likely duplicate.
    await page.goto("/contacts");
    await expect(page.getByText("Imogen Hartly")).toBeVisible();
  });

  test("the undo panel follows the language", async ({ page }) => {
    await startTour(page);
    await page.goto("/de-de/contacts/con-1");
    await page.getByTestId("merge").click();
    await page.getByTestId("merge-reason").fill("Gleiche Person");
    await page.getByTestId("merge-confirm").click();
    await page.getByTestId("unmerge").click();
    await expect(page.getByTestId("unmerge-restores")).toContainText(
      "Geht zurück",
    );
  });

  test("a contact lists its links to other systems, and one can be added and removed", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/contacts/con-1");
    const open = page.getByTestId("link-open");
    await expect(open).toHaveCount(1);
    await expect(open.first()).toHaveAttribute(
      "href",
      "https://app.hubspot.com/contacts/1234/record/0-1/5001",
    );
    // Opens in a new tab without handing the other site this page.
    await expect(open.first()).toHaveAttribute("target", "_blank");
    await expect(open.first()).toHaveAttribute("rel", /noopener/);

    await page.getByTestId("link-system").selectOption("attio");
    await page
      .getByTestId("link-url")
      .fill("https://app.attio.com/acme/people/42");
    await page.getByTestId("link-label").fill("Attio person");
    await page.getByTestId("link-add").click();
    await expect(page.getByTestId("link")).toHaveCount(2);
    await expect(page.getByTestId("links")).toContainText("Attio person");

    await page.getByTestId("link-remove").first().click();
    await expect(page.getByTestId("link")).toHaveCount(1);
    await expect(page.getByTestId("links")).toContainText("Attio person");
  });

  for (const [list, name, path] of [
    ["/accounts", "Brightwater Energy", /\/accounts\/acc-1$/],
    ["/leads", "Hollis Greenway", /\/leads\/lead-1$/],
  ] as const) {
    test(`a link is added and removed on a record opened from ${list}`, async ({
      page,
    }) => {
      await startTour(page);
      await page.goto(list);
      await page.getByRole("link", { name }).click();
      await expect(page).toHaveURL(path);
      await expect(page.getByTestId("record-title")).toHaveText(name);
      await expect(page.getByTestId("links-none")).toBeVisible();
      await page.getByTestId("link-system").selectOption("salesforce");
      await page
        .getByTestId("link-url")
        .fill("https://acme.lightning.force.com/lightning/r/Record/001/view");
      await page.getByTestId("link-add").click();
      await expect(page.getByTestId("link")).toHaveCount(1);
      await page.getByTestId("link-remove").click();
      await expect(page.getByTestId("links-none")).toBeVisible();
    });
  }

  test("an account page lists its contacts' deals, each a link to the deal", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/accounts/acc-1");
    await page
      .getByRole("link", { name: "Smart-meter analytics rollout" })
      .click();
    await expect(page).toHaveURL(/\/deals\/deal-1$/);
    await expect(page.getByTestId("record-title")).toHaveText(
      "Smart-meter analytics rollout",
    );
  });

  test("a deal opened from the deals page carries links too", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/deals");
    await page.getByTestId("deal-list").getByRole("link").first().click();
    await expect(page).toHaveURL(/\/deals\/[^/]+$/);
    await page.getByTestId("link-system").selectOption("hubspot");
    await page
      .getByTestId("link-url")
      .fill("https://app.hubspot.com/contacts/1234/deal/77");
    await page.getByTestId("link-add").click();
    await expect(page.getByTestId("link")).toHaveCount(1);
  });

  test("a deal is edited on its own page", async ({ page }) => {
    await startTour(page);
    await page.goto("/deals");
    await page.getByTestId("deal-list").getByRole("link").first().click();
    await page.getByTestId("deal-edit").click();
    await page.getByTestId("deal-name").fill("Renamed on the deal page");
    await page.getByTestId("deal-amount").fill("1234.50");
    await page.getByTestId("deal-save").click();
    await expect(page.getByTestId("deal-saved")).toBeVisible();
    await expect(page.getByTestId("record-title")).toHaveText(
      "Renamed on the deal page",
    );
    await expect(page.getByRole("main")).toContainText("1,234.50");
    await expect(page.getByTestId("deal-form")).toHaveCount(0);
  });

  test("a closed deal says it cannot be edited", async ({ page }) => {
    await startTour(page);
    await page.goto("/deals/deal-9");
    await expect(page.getByTestId("deal-locked")).toBeVisible();
    await expect(page.getByTestId("deal-edit")).toHaveCount(0);
  });

  test("a closed deal is reopened with a reason and then edited", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/deals/deal-9");
    await page.getByTestId("deal-reopen").click();
    await expect(page.getByTestId("deal-reopen-confirm")).toBeDisabled();
    await page.getByTestId("deal-reopen-reason").fill("Customer came back");
    await page.getByTestId("deal-reopen-confirm").click();
    await expect(page.getByTestId("deal-reopened")).toBeVisible();
    await expect(page.getByTestId("deal-locked")).toHaveCount(0);
    await page.getByTestId("deal-edit").click();
    await page.getByTestId("deal-name").fill("Reopened and renamed");
    await page.getByTestId("deal-save").click();
    await expect(page.getByTestId("record-title")).toHaveText(
      "Reopened and renamed",
    );
  });

  test("a link on the wrong domain, or a repeat, is refused with the reason", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/contacts/con-1");
    await page.getByTestId("link-system").selectOption("hubspot");
    await page
      .getByTestId("link-url")
      .fill("https://hubspot.com.evil.example/x");
    await page.getByTestId("link-add").click();
    await expect(page.getByTestId("link-error")).toContainText(
      "That was not accepted",
    );
    await page
      .getByTestId("link-url")
      .fill("https://app.hubspot.com/contacts/1234/record/0-1/5001");
    await page.getByTestId("link-add").click();
    await expect(page.getByTestId("link-error")).toContainText(
      "conflicts with something",
    );
    await expect(page.getByTestId("link")).toHaveCount(1);
  });

  test("the links panel follows the language", async ({ page }) => {
    await startTour(page);
    await page.goto("/de-de/contacts/con-1");
    await expect(page.getByTestId("link-add")).toHaveText("Link hinzufügen");
  });

  test("a template previews for a made-up contact, shows fallbacks, and needs the unsubscribe link", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/templates");
    await expect(page.getByTestId("template-list")).toContainText(
      "Autumn news",
    );
    await page.getByTestId("template-new").click();
    await page.getByTestId("template-name").fill("Welcome");
    await page.getByTestId("template-subject").fill("Hello {{given_name}}");
    await page.getByTestId("template-body").fill("Hi {{account_name}}");
    // No unsubscribe link yet: the preview still renders, and says what is missing.
    await expect(page.getByTestId("preview-subject")).toHaveText("Hello Alex");
    await expect(page.getByTestId("preview-text")).toHaveText("Hi Example Ltd");
    await expect(page.getByTestId("preview-problems")).toContainText(
      "unsubscribe_url",
    );
    await page.getByTestId("insert-unsubscribe_url").click();
    await expect(page.getByTestId("preview-problems")).toHaveCount(0);
    // A value that is missing shows the fallback, and the template's own wins.
    await page.getByTestId("blank-account_name").check();
    await expect(page.getByTestId("preview-text")).toContainText(
      "Hi your organisation",
    );
    await page.getByTestId("fallback-account_name").fill("your team");
    await expect(page.getByTestId("preview-text")).toContainText(
      "Hi your team",
    );
    await expect(page.getByTestId("preview-fell-back")).toContainText(
      "account_name",
    );
    await page.getByTestId("template-save").click();
    await expect(page.getByTestId("template-list")).toContainText("Welcome");
  });

  test("a template's fallbacks are in its language, and a language without them needs its own", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/templates");
    await page.getByTestId("template-new").click();
    // A new template is in the page's language.
    await expect(page.getByTestId("template-language")).toHaveValue("en");
    await page.getByTestId("template-subject").fill("{{account_name}}");
    await page
      .getByTestId("template-body")
      .fill("{{given_name}} {{unsubscribe_url}}");
    await page.getByTestId("blank-account_name").check();
    await expect(page.getByTestId("preview-subject")).toHaveText(
      "your organisation",
    );
    await page.getByTestId("template-language").fill("de");
    await expect(page.getByTestId("preview-subject")).toHaveText(
      "Ihre Organisation",
    );
    await expect(page.getByTestId("fallback-account_name")).toHaveAttribute(
      "placeholder",
      "Ihre Organisation",
    );
    // Japanese has no built-in fallbacks: the page says so, and saving needs the template's own.
    await page.getByTestId("template-language").fill("ja");
    await expect(page.getByTestId("template-no-built-in")).toBeVisible();
    await expect(page.getByTestId("preview-problems")).toContainText(
      "fallbacks/account_name",
    );
    await page.getByTestId("fallback-account_name").fill("御社");
    await expect(page.getByTestId("preview-subject")).toHaveText("御社");
    await expect(page.getByTestId("preview-problems")).toHaveCount(0);
  });

  test("a campaign is made from a template and listed as a draft", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/campaigns");
    await page.getByTestId("campaign-new").click();
    await page.getByTestId("campaign-name").fill("Autumn push");
    await page.getByTestId("campaign-currency").fill("EUR");
    await page.getByTestId("campaign-segment").selectOption("seg-1");
    await page.getByTestId("campaign-template").selectOption("template-1");
    // A template replaces the message fields.
    await expect(page.getByTestId("campaign-body")).toHaveCount(0);
    await page.getByTestId("campaign-create").click();
    await expect(page.getByTestId("campaigns")).toContainText("Autumn push");
    await expect(page.getByTestId("campaign-form")).toHaveCount(0);
  });

  test("a contact's names are edited, and an emptied box clears it", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/contacts/con-1");
    await expect(page.getByTestId("contact-names")).toContainText("—");
    await page.getByTestId("contact-edit").click();
    const form = page.getByTestId("contact-form");
    await form.getByRole("textbox").nth(1).fill("Imo");
    await form.getByRole("textbox").nth(2).fill("Hartley");
    await page.getByTestId("contact-save").click();
    await expect(page.getByTestId("contact-saved")).toBeVisible();
    await expect(page.getByTestId("contact-names")).toContainText("Imo");
    await expect(page.getByTestId("contact-names")).toContainText("Hartley");
    await page.getByTestId("contact-edit").click();
    await form.getByRole("textbox").nth(1).fill("");
    await page.getByTestId("contact-save").click();
    await expect(page.getByTestId("contact-names")).not.toContainText("Imo");
  });

  test("an account's tier and industry are edited", async ({ page }) => {
    await startTour(page);
    await page.goto("/accounts/acc-1");
    await page.getByTestId("account-edit").click();
    const form = page.getByTestId("account-form");
    await form.getByRole("combobox").selectOption("former");
    await form.getByRole("textbox").nth(1).fill("");
    await page.getByTestId("account-save").click();
    await expect(page.getByTestId("account-saved")).toBeVisible();
    await expect(page.locator(".chip").first()).toContainText("former");
  });

  test("a workflow is read, tried with a dry run, and made", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/workflows");
    await page.getByRole("button", { name: "Greet new emailers" }).click();
    await expect(page.getByTestId("workflow-runs")).toContainText(
      "channel is email",
    );
    await page.getByTestId("workflow-dry").click();
    await expect(page.getByTestId("workflow-would-fire")).not.toHaveText("0");
    await expect(page.getByTestId("workflow-dry-result")).toContainText(
      "would_skip",
    );

    await page.getByTestId("workflow-new").click();
    await page.getByTestId("workflow-name").fill("Notify on update");
    await page.getByTestId("workflow-trigger").selectOption("contact_updated");
    await page.getByTestId("workflow-action-type").selectOption("send_webhook");
    await page.getByTestId("workflow-add-action").click();
    await page
      .getByTestId("workflow-actions")
      .getByRole("textbox")
      .fill("changed");
    await page.getByTestId("workflow-save").click();
    await expect(page.getByTestId("workflow-list")).toContainText(
      "Notify on update",
    );
  });

  test("a custom field is declared and its value set on a contact", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/custom-fields");
    await page.getByTestId("cf-key").fill("segment_note");
    await page.getByTestId("cf-label").fill("Segment note");
    await page.getByTestId("cf-add").click();
    await expect(page.getByTestId("cf-list")).toContainText(
      "contact.segment_note",
    );

    await page.goto("/contacts/con-1");
    await expect(page.getByTestId("cf-hobby")).toHaveValue("sailing");
    await page.getByTestId("cf-segment_note").fill("Renewal risk");
    await page.getByTestId("cf-save").click();
    await expect(page.getByTestId("cf-saved")).toBeVisible();
    await expect(page.getByTestId("cf-segment_note")).toHaveValue(
      "Renewal risk",
    );
  });

  test("a file is attached to a contact, listed and removed", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/contacts/con-1");
    await expect(page.getByTestId("att-list")).toContainText("proposal.pdf");
    await page.getByTestId("att-file").setInputFiles({
      name: "notes.txt",
      mimeType: "text/plain",
      buffer: Buffer.from("a note"),
    });
    await expect(page.getByTestId("att-list")).toContainText("notes.txt");
    page.once("dialog", (dialog) => void dialog.accept());
    await page
      .getByTestId("att-list")
      .locator("li", { hasText: "notes.txt" })
      .getByRole("button")
      .click();
    await expect(page.getByTestId("att-list")).not.toContainText("notes.txt");
    await page.getByTestId("att-file").setInputFiles({
      name: "run.exe",
      mimeType: "application/octet-stream",
      buffer: Buffer.from("MZ"),
    });
    await expect(page.getByTestId("att-error")).toBeVisible();
  });

  test("contacts are imported from a CSV, with the address kept out of the report", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/contacts");
    await page.getByTestId("import-link").click();
    await page.getByTestId("file").setInputFiles({
      name: "people.csv",
      mimeType: "text/csv",
      buffer: Buffer.from(
        "Name,Email\nAda Lovelace,ada@x.test\n,nobody@x.test\n",
      ),
    });
    await expect(page.getByTestId("map-email")).toHaveValue("Email");
    await page.getByTestId("check").click();
    await expect(page.getByTestId("summary")).toContainText("1");
    await expect(page.getByTestId("import-report")).toContainText(
      "has no name",
    );
    await page.getByTestId("source").fill("Trade show");
    await page.getByTestId("commit").click();
    await expect(page.getByTestId("done")).toBeVisible();
    await expect(page.getByTestId("done-detail")).toBeVisible();
    await page.getByTestId("undo").click();
    await expect(page.getByTestId("undone")).toBeVisible();
  });

  test("a workflow assigns new leads to a pool of workers", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/workflows");
    await page.getByTestId("workflow-new").click();
    await page.getByTestId("workflow-name").fill("Give out web leads");
    await page.getByTestId("workflow-trigger").selectOption("lead_created");
    // A lead is assigned, not enrolled: only its own actions are offered.
    await expect(
      page
        .getByTestId("workflow-action-type")
        .locator("option[value=enrol_nurture]"),
    ).toHaveCount(0);
    await page.getByTestId("workflow-action-type").selectOption("assign_owner");
    await page.getByTestId("workflow-add-action").click();
    await page
      .getByTestId("workflow-actions")
      .getByRole("textbox")
      .fill(
        "person:00000000-0000-0000-0000-000000000001, person:00000000-0000-0000-0000-000000000002",
      );
    await page.getByTestId("workflow-save").click();
    await expect(page.getByTestId("workflow-list")).toContainText(
      "Give out web leads",
    );
  });

  test("every screen fits a phone: no sideways scroll at 360px", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 360, height: 740 });
    await startTour(page);
    const wide: string[] = [];
    for (const path of [
      ...SCREENS,
      "/workflows",
      "/custom-fields",
      "/templates",
      "/contacts/import",
      "/accounts/import",
      "/accounts/acc-1",
      "/leads/lead-1",
      "/deals/deal-1",
      "/tickets/tkt-1",
    ]) {
      await page.goto(path);
      await page.waitForLoadState("networkidle");
      await expect(page.locator("main h1").first(), path).toBeVisible();
      // The page itself must not scroll sideways; a wide table may scroll inside its own box.
      const excess = await page.evaluate(
        () =>
          document.documentElement.scrollWidth -
          document.documentElement.clientWidth,
      );
      if (excess > 1) wide.push(`${path} (+${excess}px)`);
    }
    expect(wide).toEqual([]);
  });

  test("a contact is tagged, and the list is narrowed by the tag", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/contacts/con-2");
    await expect(page.getByTestId("tags-none")).toBeVisible();
    await page.getByTestId("tag-input").fill("Press");
    await page.getByTestId("tag-add").click();
    await expect(page.getByTestId("tag-list")).toContainText("Press");
    await page.goto("/contacts");
    await page.getByTestId("tag-filter").selectOption("Press");
    await expect(page.getByTestId("contact-table")).toContainText(
      "Rafael Okonkwo",
    );
    await expect(page.getByTestId("contact-table")).not.toContainText(
      "Imogen Hartley",
    );
    await page.getByTestId("tag-filter").selectOption("");
    await expect(page.getByTestId("contact-table")).toContainText(
      "Imogen Hartley",
    );
  });

  test("a report counts leads by source, and is saved", async ({ page }) => {
    await startTour(page);
    await page.goto("/reports");
    await expect(page.getByTestId("report-list")).toContainText(
      "Leads by source",
    );
    await page.getByTestId("report-kind").selectOption("deal");
    await page.getByTestId("report-measure").selectOption("sum_amount");
    await page.getByTestId("report-group").selectOption("state");
    await page.getByTestId("report-run").click();
    await expect(page.getByTestId("report-result")).toContainText("GBP");
    await expect(page.getByTestId("report-row").first()).toBeVisible();
    await page.getByTestId("report-name").fill("Pipeline value");
    await page.getByTestId("report-save").click();
    await expect(page.getByTestId("report-list")).toContainText(
      "Pipeline value",
    );
    // Open the saved one again: it restores the form and answers.
    await page.getByRole("button", { name: "Leads by source" }).click();
    await expect(page.getByTestId("report-kind")).toHaveValue("lead");
    await expect(page.getByTestId("report-result")).toContainText("referral");
  });

  test("the organization's calendar sets the report's fiscal quarters and zone", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/reports");
    await expect(page.getByTestId("org-zone")).toHaveValue("UTC");
    await page.getByTestId("org-zone").fill("Asia/Kolkata");
    await page.getByTestId("org-fiscal-start").selectOption("4");
    await page.getByTestId("org-save").click();
    await expect(page.getByTestId("org-error")).toHaveCount(0);
    await page.getByTestId("report-kind").selectOption("deal");
    await page
      .getByTestId("report-group")
      .selectOption("created_fiscal_quarter");
    await page.getByTestId("report-run").click();
    // October 2026 is the third quarter of a year that starts in April.
    await expect(page.getByTestId("report-result")).toContainText("2026/27-Q3");
    await expect(page.getByTestId("report-periods")).toContainText(
      "Asia/Kolkata",
    );
    await expect(page.getByTestId("report-periods")).toContainText("April");
    // A zone that is not one is refused.
    await page.getByTestId("org-zone").fill("Kolkata");
    await page.getByTestId("org-save").click();
    await expect(page.getByTestId("org-error")).toBeVisible();
  });

  test("a report leaves out a group of one and says so", async ({ page }) => {
    await startTour(page);
    await page.goto("/reports");
    await page.getByTestId("report-kind").selectOption("deal");
    await page.getByTestId("report-group").selectOption("state");
    await page.getByTestId("report-run").click();
    // Eight open and three won are shown; the one lost deal would be that deal, so it is not.
    await expect(page.getByTestId("report-row")).toHaveCount(2);
    await expect(page.getByTestId("report-result")).not.toContainText("lost");
    await expect(page.getByTestId("rpt-suppressed")).toContainText(
      "1 group with fewer than 2 records was left out",
    );
  });

  test("won revenue is shared among the campaigns that touched the buyer, per model and window", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/attribution");
    await page.getByTestId("attribution-run").click();
    // Linear over the three won deals: the campaign that touched two of them earns the most, and
    // the deal nobody touched is kept apart.
    await expect(page.getByTestId("attribution-row")).toHaveCount(2);
    const first = page.getByTestId("attribution-row").first();
    await expect(first).toContainText("Spring product update");
    await expect(first).toContainText("£114,000.00");
    await expect(page.getByTestId("attribution-row").nth(1)).toContainText(
      "£26,000.00",
    );
    await expect(page.getByTestId("attribution-unattributed")).toContainText(
      "£27,500.00",
    );
    // First touch gives each deal wholly to its earliest campaign; the other still shows that it
    // touched a deal, with nothing credited.
    await page.getByTestId("attribution-model").selectOption("first");
    await page.getByTestId("attribution-run").click();
    await expect(page.getByTestId("attribution-row")).toHaveCount(2);
    await expect(page.getByTestId("attribution-row").first()).toContainText(
      "£140,000.00",
    );
    await expect(page.getByTestId("attribution-row").nth(1)).not.toContainText(
      "£",
    );
    // A short window drops the older touches, so more of the revenue is unattributed.
    await page.getByTestId("attribution-window").fill("15");
    await page.getByTestId("attribution-run").click();
    await expect(page.getByTestId("attribution-row")).toHaveCount(1);
    await expect(page.getByTestId("attribution-row")).toContainText(
      "Customer webinar series",
    );
    await expect(page.getByTestId("attribution-unattributed")).toContainText(
      "2",
    );
  });

  test("opens and clicks a mail provider reported can be chosen as touches", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/attribution");
    // Clicks alone: only the £88,000 deal has one, so its campaign takes it all and the rest is
    // unattributed (the £52,000 deal and the £27,500 one nobody touched).
    await page.getByTestId("attribution-touch-sent").uncheck();
    await page.getByTestId("attribution-touch-clicked").check();
    await page.getByTestId("attribution-run").click();
    await expect(page.getByTestId("attribution-row")).toHaveCount(1);
    await expect(page.getByTestId("attribution-row")).toContainText(
      "£88,000.00",
    );
    await expect(page.getByTestId("attribution-unattributed")).toContainText(
      "£79,500.00",
    );
    // Opens alone: the £52,000 deal's newer campaign opened its e-mail.
    await page.getByTestId("attribution-touch-clicked").uncheck();
    await page.getByTestId("attribution-touch-opened").check();
    await page.getByTestId("attribution-run").click();
    await expect(page.getByTestId("attribution-row")).toHaveCount(1);
    await expect(page.getByTestId("attribution-row")).toContainText(
      "£52,000.00",
    );
    // That open came twenty seconds after sending: leaving out opens sooner than a minute drops it.
    await page.getByTestId("attribution-min-open").fill("60");
    await page.getByTestId("attribution-run").click();
    await expect(page.getByTestId("attribution-row")).toHaveCount(0);
    await page.getByTestId("attribution-min-open").fill("10");
    await page.getByTestId("attribution-run").click();
    await expect(page.getByTestId("attribution-row")).toHaveCount(1);
    // With no kind chosen there is nothing to ask for; the open setting waits for opens.
    await page.getByTestId("attribution-touch-opened").uncheck();
    await expect(page.getByTestId("attribution-min-open")).toBeDisabled();
    await expect(page.getByTestId("attribution-run")).toBeDisabled();
  });

  test("a click sooner after sending than the delay is left out", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/attribution");
    await page.getByTestId("attribution-touch-sent").uncheck();
    await expect(page.getByTestId("attribution-min-click")).toBeDisabled();
    await page.getByTestId("attribution-touch-clicked").check();
    await page.getByTestId("attribution-run").click();
    await expect(page.getByTestId("attribution-row")).toHaveCount(1);
    // That click came eight seconds after sending: a delay of a minute drops it, ten seconds keeps it.
    await page.getByTestId("attribution-min-click").fill("60");
    await page.getByTestId("attribution-run").click();
    await expect(page.getByTestId("attribution-row")).toHaveCount(0);
    await page.getByTestId("attribution-min-click").fill("5");
    await page.getByTestId("attribution-run").click();
    await expect(page.getByTestId("attribution-row")).toHaveCount(1);
  });

  test("time decay favours the recent touch and position splits the ends evenly", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/attribution");
    // The older campaign touched the £52,000 deal 40 days before it closed, the newer one 9 days
    // before: the newer earns almost all of it, and every pound is still accounted for.
    await page.getByTestId("attribution-model").selectOption("time_decay");
    await page.getByTestId("attribution-run").click();
    await expect(page.getByTestId("attribution-row").first()).toContainText(
      "£90,328.36",
    );
    await expect(page.getByTestId("attribution-row").nth(1)).toContainText(
      "£49,671.64",
    );
    // A longer half-life lets the older touch weigh more; the field is for time decay alone.
    await page.getByTestId("attribution-half-life").fill("90");
    await page.getByTestId("attribution-run").click();
    await expect(page.getByTestId("attribution-row").first()).not.toContainText(
      "£90,328.36",
    );
    // With two touches, position gives each end half.
    await page.getByTestId("attribution-model").selectOption("position");
    await expect(page.getByTestId("attribution-half-life")).toBeDisabled();
    // The end share is for position alone; with two touches each end still gets half.
    await page.getByTestId("attribution-end-percent").fill("30");
    await page.getByTestId("attribution-run").click();
    await expect(page.getByTestId("attribution-row").first()).toContainText(
      "£114,000.00",
    );
    await expect(page.getByTestId("attribution-row").nth(1)).toContainText(
      "£26,000.00",
    );
  });

  test("attribution is shown in one currency, with the rates it used and what it could not convert", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/attribution");
    await page.getByTestId("attribution-convert").fill("USD");
    await page.getByTestId("attribution-run").click();
    // The latest pound-to-dollar rate is 1.30.
    await expect(page.getByTestId("attribution-row").first()).toContainText(
      "$148,200.00",
    );
    await expect(page.getByTestId("attribution-row").nth(1)).toContainText(
      "$33,800.00",
    );
    await expect(page.getByTestId("attribution-unattributed")).toContainText(
      "$35,750.00",
    );
    await expect(page.getByTestId("attribution-rates")).toContainText(
      "1 GBP = 1.3 USD",
    );
    // No pound-to-yen rate: nothing is credited, and the pounds are listed as left out.
    await page.getByTestId("attribution-convert").fill("JPY");
    await page.getByTestId("attribution-run").click();
    await expect(page.getByTestId("attribution-row").first()).not.toContainText(
      "¥",
    );
    await expect(page.getByTestId("attribution-unconverted")).toContainText(
      "GBP",
    );
  });

  test("the help centre shows published articles to anyone, counts a helpful vote, and staff see the counts", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/help");
    // Five published articles; the draft is not offered.
    await expect(page.getByTestId("help-result")).toHaveCount(5);
    await expect(page.getByTestId("help-results")).not.toContainText(
      "awaiting review",
    );
    await page.getByTestId("help-query").fill("forecast");
    await page.getByTestId("help-run").click();
    await expect(page.getByTestId("help-result")).toHaveCount(1);
    await page.getByTestId("help-query").fill("zzzz");
    await page.getByTestId("help-run").click();
    await expect(page.getByTestId("help-none")).toBeVisible();

    await page.getByTestId("help-query").fill("forecast");
    await page.getByTestId("help-run").click();
    await page.getByTestId("help-result").getByRole("link").click();
    await expect(page.getByTestId("help-article")).toContainText(
      "multiplied by its stage probability",
    );
    await page.getByTestId("help-yes").click();
    await expect(page.getByTestId("help-thanks")).toBeVisible();

    // A draft is simply not there.
    await page.goto("/help/kb-5");
    await expect(page.getByTestId("help-missing")).toBeVisible();

    // Staff see the counts: 80 views and 42 yes votes to begin with, plus this visit and vote,
    // against the tour's eight tickets: 43 of 51.
    await page.goto("/articles");
    const panel = page.getByTestId("kb-deflection");
    await expect(panel).toContainText("81");
    await expect(panel).toContainText("84%");
    await expect(panel).toContainText("43 / 51");
  });

  test("the forecast can also be shown in one currency, naming the rate it used", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/deals");
    await expect(page.getByTestId("forecast-converted")).toHaveCount(0);
    await page.getByTestId("forecast-convert").fill("USD");
    await expect(page.getByTestId("forecast-converted")).toContainText("$");
    await expect(page.getByTestId("forecast-converted")).toContainText(
      "1 GBP = 1.3 USD",
    );
    // No pound-to-yen rate: the pounds are listed as left out, and the total is the yen deal's
    // weighted amount alone, in whole yen (¥14,500,000 at 50%).
    await page.getByTestId("forecast-convert").fill("JPY");
    await expect(page.getByTestId("forecast-converted")).toContainText("GBP");
    await expect(page.getByTestId("forecast-converted")).toContainText(
      "¥7,250,000",
    );
  });

  test("a deal report is shown in one currency, with the rates it used and what it could not convert", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/reports");
    await page.getByTestId("report-kind").selectOption("deal");
    await page.getByTestId("report-measure").selectOption("sum_amount");
    await page.getByTestId("report-group").selectOption("state");
    await page.getByTestId("report-convert").fill("USD");
    await page.getByTestId("report-run").click();
    // The latest pound-to-dollar rate: 731,500 of open deals at 1.30; the yen and dinar deals have
    // no rate to the dollar and are left out.
    await expect(page.getByTestId("report-result")).toContainText("950,950.00");
    await expect(page.getByTestId("report-unconverted")).toContainText("JPY");
    await expect(page.getByTestId("report-unconverted")).toContainText("KWD");
    await expect(page.getByTestId("report-rates")).toContainText(
      "1 GBP = 1.3 USD",
    );
    // An earlier date picks the earlier rate.
    const earlier = new Date(Date.now() - 100 * 86_400_000)
      .toISOString()
      .slice(0, 10);
    await page.getByTestId("report-rate-date").fill(earlier);
    await page.getByTestId("report-run").click();
    await expect(page.getByTestId("report-result")).toContainText("929,005.00");
    await expect(page.getByTestId("report-rates")).toContainText(
      "1 GBP = 1.27 USD",
    );
    // No pound-to-yen rate: only the yen deal is in yen, and the pounds are listed as left out.
    await page.getByTestId("report-convert").fill("JPY");
    await page.getByTestId("report-run").click();
    await expect(page.getByTestId("report-result")).toContainText("14,500,000");
    await expect(page.getByTestId("report-unconverted")).toContainText("GBP");
  });

  test("a deal report in euros uses rates from more than one base, each currency in its own unit", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/reports");
    await page.getByTestId("report-kind").selectOption("deal");
    await page.getByTestId("report-measure").selectOption("sum_amount");
    await page.getByTestId("report-group").selectOption("state");
    await page.getByTestId("report-convert").fill("EUR");
    await page.getByTestId("report-run").click();
    // Pounds at 1.17, dinars (three places) at 2.79, and yen (none) by the euro-to-yen rate
    // inverted: 855,855.00 + 26,785.40 + 89,230.77 = 971,871.17.
    await expect(page.getByTestId("report-result")).toContainText("971,871.17");
    await expect(page.getByTestId("report-rates")).toContainText(
      "1 KWD = 2.79 EUR",
    );
    await expect(page.getByTestId("report-rates")).toContainText("JPY");
    await expect(page.getByTestId("report-unconverted")).toHaveCount(0);
  });

  test("attribution can convert each deal's share at the rate of the day it closed", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/attribution");
    await expect(page.getByTestId("attribution-close-day")).toHaveCount(0);
    await page.getByTestId("attribution-convert").fill("USD");
    await page.getByTestId("attribution-close-day").check();
    await expect(page.getByTestId("attribution-rate-date")).toBeDisabled();
    await page.getByTestId("attribution-run").click();
    await expect(page.getByTestId("attribution-row").first()).toContainText(
      "$",
    );
    await expect(page.getByTestId("attribution-error")).toHaveCount(0);
    await expect(page.getByTestId("attribution-rates")).toBeVisible();
    // No one day stands for the answer, so none is shown beside the rates.
    await expect(page.getByTestId("attribution-rates")).not.toContainText(
      "(20",
    );
  });

  test("a deal report can convert each deal at the rate of the day it closed, for closed deals only", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/reports");
    await page.getByTestId("report-kind").selectOption("deal");
    await page.getByTestId("report-measure").selectOption("sum_amount");
    await page.getByTestId("report-group").selectOption("state");
    await page.getByTestId("report-convert").fill("USD");
    // An open deal has no close day: the choice waits for a report of won or lost deals.
    await expect(page.getByTestId("report-close-day")).toBeDisabled();
    await page.getByTestId("report-filter-field").selectOption("state");
    await page.getByTestId("report-filter-values").fill("won, open");
    await expect(page.getByTestId("report-close-day")).toBeDisabled();
    await page.getByTestId("report-filter-values").fill("won, lost");
    await page.getByTestId("report-close-day").check();
    await expect(page.getByTestId("report-rate-date")).toBeDisabled();
    await page.getByTestId("report-run").click();
    await expect(page.getByTestId("report-result")).toBeVisible();
    await expect(page.getByTestId("report-error")).toHaveCount(0);
    // No one day stands for the answer, so none is shown beside the rates.
    // (A day is written in the page's format, "26 Oct 2026", so the check is for any bracketed date.)
    await expect(page.getByTestId("report-rates")).not.toContainText(
      /\(\d{1,2} \w+ \d{4}\)/,
    );
  });

  test("quotas show where each worker stands, and a plan in use cannot be deleted", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/quotas");
    await expect(page.getByTestId("quota-standing-row")).toHaveCount(2);
    await expect(page.getByTestId("quota-row")).toHaveCount(2);
    await page.getByTestId("quota-owner").fill("owner-cy");
    await page.getByTestId("quota-currency").fill("GBP");
    await page.getByTestId("quota-amount").fill("1000.00");
    await page.getByTestId("quota-set").click();
    await expect(page.getByTestId("quota-row")).toHaveCount(3);
    // A plan that quotas are paid under is refused.
    await page.getByTestId("plan-row").getByRole("button").click();
    await expect(page.getByTestId("quota-error")).toBeVisible();
  });

  test("a selected ticket is offered the articles that share its words", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/tickets");
    const queue = page.getByTestId("ticket-queue");
    await queue.locator("tbody tr", { hasText: "Licence seats" }).click();
    await expect(page.getByTestId("ticket-suggestions")).toContainText(
      "Licence seats missing from the console",
    );
    // A ticket sharing no words with any article is offered none.
    await queue.locator("tbody tr", { hasText: "Route export" }).click();
    await expect(page.getByTestId("ticket-suggestions")).toHaveCount(0);
  });

  test("a won deal can be split between workers, and the standing follows", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/quotas");
    const ayo = page
      .getByTestId("quota-standing-row")
      .filter({ hasText: "owner-ayo" });
    const bea = page
      .getByTestId("quota-standing-row")
      .filter({ hasText: "owner-bea" });
    await expect(bea).toContainText("140,000.00");
    await page
      .getByTestId("split-deal")
      .selectOption({ label: "Fleet telematics renewal" });
    await page.getByTestId("split-shares").fill("owner-ayo:50\nowner-bea:50");
    await page.getByTestId("split-set").click();
    // £88,000 shared evenly: Bea keeps 52,000 + 44,000, Ayo gains 44,000 on 27,500.
    await expect(bea).toContainText("96,000.00");
    await expect(ayo).toContainText("71,500.00");
    // Shares that do not add up to 100 are refused.
    await page.getByTestId("split-shares").fill("owner-ayo:60\nowner-bea:50");
    await page.getByTestId("split-set").click();
    await expect(page.getByTestId("quota-error")).toBeVisible();
    // Removing the split credits the owner again.
    await page.getByTestId("split-remove").click();
    await expect(bea).toContainText("140,000.00");
  });

  test("a won deal offers its delivery idea to the project portfolio, and teams can be imported", async ({
    page,
  }) => {
    await startTour(page);
    // The tour has no other services, so both bridges answer as the stub does: nothing is sent.
    await page.goto("/deals/deal-10");
    await page.getByTestId("deal-delivery-open").click();
    await expect(page.getByTestId("deal-delivery-result")).toContainText(
      "Deliver: Fleet telematics renewal",
    );
    // An open deal offers none.
    await page.goto("/deals/deal-1");
    await expect(page.getByTestId("deal-delivery")).toHaveCount(0);
    await page.goto("/quotas");
    await page.getByTestId("team-import").click();
    await expect(page.getByTestId("team-import-result")).toBeVisible();
    // The employment check (CRM-R94) sends nothing either, and says how many it would ask about.
    await page.getByTestId("employment-check").click();
    await expect(page.getByTestId("employment-check-result")).toContainText(
      /: \d+$/,
    );
  });

  test("credit can be clawed back from one worker and given back", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/quotas");
    const bea = page
      .getByTestId("quota-standing-row")
      .filter({ hasText: "owner-bea" });
    await expect(bea).toContainText("140,000.00");
    await page
      .getByTestId("clawback-deal")
      .selectOption({ label: "Fleet telematics renewal" });
    await page.getByTestId("clawback-owner").fill("owner-bea");
    await page.getByTestId("clawback-amount").fill("8000");
    await page.getByTestId("clawback-reason").fill("customer refund");
    await page.getByTestId("clawback-set").click();
    // £88,000 less £8,000 taken back.
    await expect(bea).toContainText("132,000.00");
    // More than the worker holds for the deal is refused, as the service refuses it.
    await page.getByTestId("clawback-amount").fill("90000");
    await page.getByTestId("clawback-set").click();
    await expect(page.getByTestId("quota-error")).toBeVisible();
    await page.getByTestId("clawback-remove").click();
    await expect(bea).toContainText("140,000.00");
  });

  test("a team adds its workers' standings and can be removed", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/quotas");
    await page.getByTestId("team-name").fill("North");
    await page.getByTestId("team-members").fill("owner-ayo\nowner-bea");
    await page.getByTestId("team-set").click();
    const north = page.getByTestId("team-row").filter({ hasText: "North" });
    // Ayo's 27,500.00 and Bea's 140,000.00 are added in one currency line.
    await expect(north).toContainText("owner-ayo, owner-bea");
    await expect(north).toContainText("150,000.00");
    await expect(north).toContainText("167,500.00");
    // A team with no workers is refused.
    await page.getByTestId("team-members").fill(" ");
    await page.getByTestId("team-set").click();
    await expect(page.getByTestId("quota-error")).toBeVisible();
    await north.getByRole("button").click();
    await expect(page.getByTestId("team-list")).toHaveCount(0);
  });

  test("a territory of named accounts adds what was won on them, and an account is in only one", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/quotas");
    await page.getByTestId("territory-name").fill("Everywhere");
    await page
      .getByTestId("territory-accounts")
      .fill([1, 2, 3, 4, 5, 6].map((n) => `org-demo-${n}`).join("\n"));
    await page.getByTestId("territory-set").click();
    const row = page
      .getByTestId("territory-row")
      .filter({ hasText: "Everywhere" });
    // The three won deals (52,000 + 88,000 + 27,500) are all on these accounts.
    await expect(row).toContainText("167,500.00");
    await expect(page.getByTestId("territory-unassigned")).toHaveCount(0);
    // An account already in a territory cannot be taken by another.
    await page.getByTestId("territory-name").fill("Elsewhere");
    await page.getByTestId("territory-accounts").fill("org-demo-1");
    await page.getByTestId("territory-set").click();
    await expect(page.getByTestId("quota-error")).toContainText(
      "That was not accepted",
    );
    // A territory with only one account leaves the others' revenue unassigned.
    await page.getByTestId("territory-name").fill("Everywhere");
    await page.getByTestId("territory-accounts").fill("org-demo-6");
    await page.getByTestId("territory-set").click();
    await expect(page.getByTestId("territory-unassigned")).toBeVisible();
    await row.getByRole("button").click();
    await expect(page.getByTestId("territory-row")).toHaveCount(0);
  });

  test("an account shows its lifetime value, the margin on it and a discounted look-ahead", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/accounts/acc-2");
    // One won deal of £88,000 closed 30 days ago.
    await expect(page.getByTestId("clv-won")).toContainText("88,000.00");
    await expect(page.getByTestId("clv-projected")).toHaveCount(0);
    await page.getByTestId("clv-margin").fill("50");
    await page.getByTestId("clv-retention").fill("80");
    await page.getByTestId("clv-years").fill("3");
    await page.getByTestId("clv-run").click();
    // Margin £44,000; years 35,200 + 28,160 + 22,528 = 85,888; plus the margin won.
    await expect(page.getByTestId("clv-margin-result")).toContainText(
      "44,000.00",
    );
    await expect(page.getByTestId("clv-projected")).toContainText("85,888.00");
    await expect(page.getByTestId("clv-total")).toContainText("129,888.00");
    // A retention above 100% is refused, as the service refuses it.
    await page.getByTestId("clv-retention").fill("120");
    await page.getByTestId("clv-run").click();
    await expect(page.getByTestId("clv-error")).toBeVisible();
    // From history: nothing was won in the year before, so there is no retention and nothing is
    // projected; what was won still counts.
    await page.getByTestId("clv-history").check();
    await page.getByTestId("clv-run").click();
    await expect(page.getByTestId("clv-history-retention")).toContainText("—");
    await expect(page.getByTestId("clv-projected")).toContainText("0.00");
    await expect(page.getByTestId("clv-total")).toContainText("44,000.00");
    // In one currency: the tour's GBP account in GBP is unchanged, and in a currency with no rate
    // nothing is invented.
    await page.getByTestId("clv-convert").fill("GBP");
    await page.getByTestId("clv-run").click();
    await expect(page.getByTestId("clv-converted")).toContainText("88,000.00");
    await expect(page.getByTestId("clv-converted")).toContainText("44,000.00");
    await page.getByTestId("clv-convert").fill("JPY");
    await page.getByTestId("clv-run").click();
    await expect(page.getByTestId("clv-unconverted")).toContainText("GBP");
  });

  test("a lifetime value can be converted at the rate of the day each deal closed", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/accounts/acc-2");
    await page.getByTestId("clv-convert").fill("USD");
    // The close-day choice is off until a currency is chosen, and is for the total alone.
    await page.getByTestId("clv-run").click();
    // £88,000 at today's 1.30.
    await expect(page.getByTestId("clv-converted")).toContainText("114,400.00");
    // The same deal at the 1.27 in force 30 days ago, when it closed.
    await page.getByTestId("clv-close-day").check();
    await page.getByTestId("clv-run").click();
    await expect(page.getByTestId("clv-converted")).toContainText("111,760.00");
    // With a margin asked for, the choice is unavailable.
    await page.getByTestId("clv-margin").fill("50");
    await expect(page.getByTestId("clv-close-day")).toBeDisabled();
  });

  test("exchange rates are listed, set, replaced and deleted, and a bad rate is refused", async ({
    page,
  }) => {
    await startTour(page);
    await page.goto("/fx-rates");
    // Six rates, from four bases (GBP, EUR, KWD, INR), not all from the pound.
    await expect(page.getByTestId("fx-row")).toHaveCount(6);
    await page.getByTestId("fx-from").fill("EUR");
    await page.getByTestId("fx-to").fill("GBP");
    await page.getByTestId("fx-rate").fill("0.8600");
    await page.getByTestId("fx-source").fill("Board budget rate");
    await page.getByTestId("fx-set").click();
    await expect(page.getByTestId("fx-row")).toHaveCount(7);
    await expect(page.getByTestId("fx-list")).toContainText("0.86");
    await expect(page.getByTestId("fx-list")).toContainText(
      "Board budget rate",
    );
    // The same pair and day replaces.
    await page.getByTestId("fx-rate").fill("0.87");
    await page.getByTestId("fx-set").click();
    await expect(page.getByTestId("fx-row")).toHaveCount(7);
    await expect(page.getByTestId("fx-list")).toContainText("0.87");
    await page.getByTestId("fx-rate").fill("abc");
    await page.getByTestId("fx-set").click();
    await expect(page.getByTestId("fx-error")).toBeVisible();
    await page
      .getByTestId("fx-row")
      .filter({ hasText: "Board budget rate" })
      .getByRole("button")
      .click();
    await expect(page.getByTestId("fx-row")).toHaveCount(6);
  });
});
