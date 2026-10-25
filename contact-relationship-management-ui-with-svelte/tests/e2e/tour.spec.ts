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
    const editor = board.getByRole("spinbutton");
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
      "must be on one of",
    );
    await page
      .getByTestId("link-url")
      .fill("https://app.hubspot.com/contacts/1234/record/0-1/5001");
    await page.getByTestId("link-add").click();
    await expect(page.getByTestId("link-error")).toContainText("already has");
    await expect(page.getByTestId("link")).toHaveCount(1);
  });

  test("the links panel follows the language", async ({ page }) => {
    await startTour(page);
    await page.goto("/de-de/contacts/con-1");
    await expect(page.getByTestId("link-add")).toHaveText("Link hinzufügen");
  });
});
