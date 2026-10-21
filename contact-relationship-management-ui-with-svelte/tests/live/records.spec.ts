// Links on accounts, deals and leads, driven through the UI against the real
// service (CRM-R27): the record page loads, and a link is added and removed.

import { expect, test, type APIRequestContext } from "@playwright/test";

const API = process.env.CRM_API_URL ?? "http://localhost:5160";

async function create(request: APIRequestContext, path: string, data: unknown) {
  const response = await request.post(`${API}/api/${path}`, { data });
  expect(response.ok(), `${path}: ${response.status()}`).toBeTruthy();
  return (await response.json()) as { pid: string };
}

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

async function linkRoundTrip(
  page: import("@playwright/test").Page,
  url: string,
) {
  await expect(page.getByTestId("links-none")).toBeVisible();
  await page.getByTestId("link-system").selectOption("hubspot");
  await page.getByTestId("link-url").fill(url);
  await page.getByTestId("link-add").click();
  await expect(page.getByTestId("link")).toHaveCount(1);
  await expect(page.getByTestId("link-open")).toHaveAttribute("href", url);
  await page.getByTestId("link-remove").click();
  await expect(page.getByTestId("links-none")).toBeVisible();
}

test("an account page shows the account and takes links", async ({
  page,
  request,
}) => {
  const name = `Linked Account ${Date.now().toString(36)}`;
  const account = await create(request, "accounts", {
    organization_ref: `organization:${crypto.randomUUID()}`,
    display_name: name,
  });
  await page.goto(`/accounts/${account.pid}`);
  await expect(page.getByTestId("record-title")).toHaveText(name);
  await linkRoundTrip(page, "https://app.hubspot.com/contacts/1/company/9");
});

test("a lead page shows the lead and takes links", async ({
  page,
  request,
}) => {
  const name = `Linked Lead ${Date.now().toString(36)}`;
  const lead = await create(request, "leads", {
    display_name: name,
    source: "web",
  });
  await page.goto(`/leads/${lead.pid}`);
  await expect(page.getByTestId("record-title")).toHaveText(name);
  await linkRoundTrip(page, "https://app.hubspot.com/contacts/1/lead/9");
});

test("a deal page shows the deal and takes links", async ({
  page,
  request,
}) => {
  const name = `Linked Deal ${Date.now().toString(36)}`;
  const pipeline = await create(request, "pipelines", {
    name: `P ${name}`,
    stages: [
      { name: "Qualified", probability_percent: 10 },
      { name: "Won", probability_percent: 100, is_won: true },
    ],
  });
  const deal = await create(request, "deals", {
    name,
    pipeline_pid: pipeline.pid,
    amount_minor: 5000,
    currency: "GBP",
  });
  await page.goto(`/deals/${deal.pid}`);
  await expect(page.getByTestId("record-title")).toHaveText(name);
  await linkRoundTrip(page, "https://app.hubspot.com/contacts/1/deal/9");
});

test("a deal is edited on its own page and the service keeps it", async ({
  page,
  request,
}) => {
  const pipeline = await create(request, "pipelines", {
    name: `Edit ${Date.now()}`,
    stages: [
      { name: "Qualified", probability_percent: 10 },
      { name: "Won", probability_percent: 100, is_won: true },
    ],
  });
  const deal = await create(request, "deals", {
    name: "Before",
    pipeline_pid: pipeline.pid,
    amount_minor: 100_000,
    currency: "GBP",
  });
  await page.goto(`/deals/${deal.pid}`);
  await page.getByTestId("deal-edit").click();
  await page.getByTestId("deal-name").fill("After");
  await page.getByTestId("deal-amount").fill("2500");
  await page.getByTestId("deal-close-on").fill("2027-03-31");
  await page.getByTestId("deal-save").click();
  await expect(page.getByTestId("deal-saved")).toBeVisible();
  await expect(page.getByTestId("record-title")).toHaveText("After");

  const list = await request.get(`${API}/api/deals?pipeline=${pipeline.pid}`);
  const found = (await list.json()) as Record<string, unknown>[];
  expect(found[0]).toMatchObject({
    name: "After",
    amount_minor: 250_000,
    expected_close_on: "2027-03-31",
  });
});

test("a won deal is reopened with a reason and the service reopens it", async ({
  page,
  request,
}) => {
  const pipeline = await create(request, "pipelines", {
    name: `Reopen ${Date.now()}`,
    stages: [
      { name: "Qualified", probability_percent: 10 },
      { name: "Won", probability_percent: 100, is_won: true },
    ],
  });
  const deal = await create(request, "deals", {
    name: "Closed one",
    pipeline_pid: pipeline.pid,
    amount_minor: 50_000,
    currency: "GBP",
  });
  const stages = (await (await request.get(`${API}/api/pipelines`)).json()) as {
    pipeline: { pid: string };
    stages: { pid: string; is_won: boolean }[];
  }[];
  const won = stages
    .find((p) => p.pipeline.pid === pipeline.pid)!
    .stages.find((s) => s.is_won)!;
  const moved = await request.post(`${API}/api/deals/${deal.pid}/stage`, {
    data: { stage_pid: won.pid },
  });
  expect(moved.ok(), `move: ${moved.status()}`).toBeTruthy();

  await page.goto(`/deals/${deal.pid}`);
  await expect(page.getByTestId("deal-locked")).toBeVisible();
  await page.getByTestId("deal-reopen").click();
  await page.getByTestId("deal-reopen-reason").fill("Customer came back");
  await page.getByTestId("deal-reopen-confirm").click();
  await expect(page.getByTestId("deal-reopened")).toBeVisible();
  await expect(page.getByTestId("deal-edit")).toBeVisible();

  const list = (await (
    await request.get(`${API}/api/deals?pipeline=${pipeline.pid}`)
  ).json()) as Record<string, unknown>[];
  expect(list[0]).toMatchObject({ pid: deal.pid, closed_at: null, won: false });
});
