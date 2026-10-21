// The boards and the follow-up calendar driven through the UI against the real
// service: every change is read back from the service, not from the page.

import { expect, test, type APIRequestContext } from "@playwright/test";

const API = process.env.CRM_API_URL ?? "http://localhost:5160";
const stamp = Date.now().toString(36);

async function create(request: APIRequestContext, path: string, data: unknown) {
  const response = await request.post(`${API}/api/${path}`, { data });
  expect(response.ok(), `${path}: ${response.status()}`).toBeTruthy();
  return (await response.json()) as { pid: string };
}
const list = async (request: APIRequestContext, path: string) =>
  (await (await request.get(`${API}/api/${path}`)).json()) as Record<
    string,
    unknown
  >[];

// The service runs with authentication off, so any opaque session id gets past
// the UI's sign-in gate; the BFF forwards without a token.
test.beforeEach(async ({ context }) => {
  await context.addCookies([
    {
      name: "__Host-mxi_session",
      value: "live-test",
      domain: "localhost",
      path: "/",
      secure: true,
    },
  ]);
});

test("a ticket is renamed on the board and the service has the new title", async ({
  page,
  request,
}) => {
  const ticket = await create(request, "tickets", {
    title: `Live rename ${stamp}`,
  });
  await page.goto("/tickets/board");
  const board = page.getByTestId("ticket-board");
  await expect(board).toContainText(`Live rename ${stamp}`);
  const card = board
    .getByRole("gridcell")
    .filter({ hasText: `Live rename ${stamp}` });
  // The title edits in place: double-click it (or F2), Enter commits.
  await card.locator(".kanban-board-card-title").dblclick();
  const box = board.getByRole("textbox", {
    name: new RegExp(`Live rename ${stamp}`),
  });
  await box.fill(`Live renamed ${stamp}`);
  await box.press("Enter");
  await expect(board).toContainText(`Live renamed ${stamp}`);
  await expect
    .poll(
      async () =>
        (await list(request, "tickets")).find((x) => x.pid === ticket.pid)
          ?.title,
    )
    .toBe(`Live renamed ${stamp}`);
});

test("a deal's amount is edited on the board and the service has it", async ({
  page,
  request,
}) => {
  // Its own deal, with an amount nothing else shows, so the test needs no
  // data to be there already.
  const pipeline = await create(request, "pipelines", {
    name: `Live pipeline ${stamp}`,
    stages: [
      { name: "Qualification", probability_percent: 10 },
      { name: "Won", probability_percent: 100, is_won: true },
      { name: "Lost", probability_percent: 0, is_lost: true },
    ],
  });
  const amount = 7_000_000_00 + Math.floor(Math.random() * 10_000) * 100;
  const deal = {
    ...(await create(request, "deals", {
      name: `Live amount ${stamp}`,
      pipeline_pid: pipeline.pid,
      amount_minor: amount,
      currency: "GBP",
    })),
    currency: "GBP",
    amount_minor: amount,
  };
  const target = 4_242_00 + Math.floor(Math.random() * 100) * 100;
  await page.goto("/deals");
  // The page opens on the first pipeline; this deal is in its own.
  await page.getByRole("combobox").selectOption(pipeline.pid);
  const board = page.getByTestId("deal-board");
  const shown = new Intl.NumberFormat("en-GB", {
    style: "currency",
    currency: String(deal.currency),
  }).format(Number(deal.amount_minor) / 100);
  await board.getByRole("button", { name: shown }).first().click();
  const editor = board.getByRole("spinbutton");
  await editor.fill(String(target / 100));
  await editor.press("Enter");
  await expect(page.getByTestId("action-error")).toHaveCount(0);
  await expect
    .poll(
      async () =>
        (await list(request, "deals")).find((d) => d.pid === deal.pid)
          ?.amount_minor,
    )
    .toBe(target);
  const stored = (await list(request, "deals")).find(
    (d) => d.pid === deal.pid,
  )!;
  // The trigger moved updated_at past created_at.
  expect(Date.parse(String(stored.updated_at))).toBeGreaterThan(
    Date.parse(String(stored.created_at)),
  );
});

test("a card moved above another in its column with the keyboard keeps that order", async ({
  page,
  request,
}) => {
  const a = await create(request, "tickets", { title: `Order A ${stamp}` });
  const b = await create(request, "tickets", { title: `Order B ${stamp}` });
  await page.goto("/tickets/board");
  const board = page.getByTestId("ticket-board");
  const card = (title: string) =>
    board.getByRole("gridcell", { name: new RegExp(title) });
  await card(`Order B ${stamp}`).focus();
  await page.keyboard.press("Alt+ArrowUp");
  await expect
    .poll(async () => {
      const order = (await list(request, "tickets")).map((x) => x.pid);
      return order.indexOf(b.pid) < order.indexOf(a.pid);
    })
    .toBe(true);
});

test("a follow-up is rescheduled with the keyboard and the service has the new day", async ({
  page,
  request,
}) => {
  // Its own contact: a fresh database has none to borrow.
  const contact = await create(request, "contacts", {
    person_ref: `person:${crypto.randomUUID()}`,
    display_name: `Follow-up ${stamp}`,
  });
  // A random day in the 30-day window: the calendar shows 3 events a day, and
  // earlier runs against the same database leave tasks behind.
  const ahead = 3 + Math.floor(Math.random() * 20);
  const due = new Date(Date.now() + ahead * 86_400_000)
    .toISOString()
    .slice(0, 10);
  const summary = `Live follow-up ${stamp}`;
  const activity = await create(request, "activities", {
    subject_kind: "contact",
    subject_pid: contact.pid,
    kind: "task",
    summary,
    due_on: due,
  });
  await page.goto("/followups");
  const event = page
    .getByTestId("followups-calendar")
    .getByText(`task: ${summary}`);
  await expect(event).toBeVisible();
  await event.focus();
  await page.keyboard.press("Alt+ArrowRight");
  await expect(page.getByTestId("move-error")).toHaveCount(0);
  await expect
    .poll(async () => {
      const view = (await (
        await request.get(`${API}/api/insights/followups`)
      ).json()) as {
        upcoming_30d: { pid: string; due_on: string }[];
      };
      return view.upcoming_30d.find((x) => x.pid === activity.pid)?.due_on;
    })
    .not.toBe(due);
});

test("a card dragged with the mouse above another keeps that order", async ({
  page,
  request,
}) => {
  const a = await create(request, "tickets", { title: `Drag A ${stamp}` });
  const b = await create(request, "tickets", { title: `Drag B ${stamp}` });
  await page.goto("/tickets/board");
  const board = page.getByTestId("ticket-board");
  const card = (title: string) =>
    board.getByRole("gridcell", { name: new RegExp(title) });
  // The card's title is the drag handle; the drop target is the other card's cell.
  const handle = card(`Drag B ${stamp}`).locator(".kanban-board-card-title");
  await expect(handle).toBeVisible();
  await handle.dragTo(card(`Drag A ${stamp}`));
  await expect
    .poll(async () => {
      const order = (await list(request, "tickets")).map((x) => x.pid);
      return order.indexOf(b.pid) < order.indexOf(a.pid);
    })
    .toBe(true);
});
