// Likely duplicates on the contact page, driven through the UI against the
// real service: the section shows what the service's matcher returned.

import { expect, test, type APIRequestContext } from "@playwright/test";

const API = process.env.CRM_API_URL ?? "http://localhost:5160";
// Records from earlier runs stay behind and can resemble these, so the test
// looks at the twin's own row and its rank, never at a count.
const surname = `Qvx${Date.now().toString(36)}`;

async function create(request: APIRequestContext, path: string, data: unknown) {
  const response = await request.post(`${API}/api/${path}`, { data });
  expect(response.ok(), `${path}: ${response.status()}`).toBeTruthy();
  return (await response.json()) as { pid: string };
}
// Ten random letters: resembles nothing left behind by an earlier run.
const letters = () =>
  Array.from(crypto.getRandomValues(new Uint8Array(10)), (b) =>
    String.fromCharCode(97 + (b % 26)),
  ).join("");
const person = () => `person:${crypto.randomUUID()}`;

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

test("the contact page lists the service's likely duplicates, and a unique contact has none", async ({
  page,
  request,
}) => {
  const account = await create(request, "accounts", {
    organization_ref: `organization:${crypto.randomUUID()}`,
    display_name: `Live Duplicates Ltd ${surname}`,
  });
  const target = await create(request, "contacts", {
    person_ref: person(),
    display_name: `Robert ${surname}`,
    job_title: "VP Sales",
    account_pid: account.pid,
  });
  // The same person, written differently: a nickname and a spelled-out title.
  const twin = await create(request, "contacts", {
    person_ref: person(),
    display_name: `Bob ${surname}`,
    job_title: "Vice President, Sales",
    account_pid: account.pid,
  });
  const stranger = await create(request, "contacts", {
    person_ref: person(),
    display_name: `${letters()} ${letters()}`,
  });

  await page.goto(`/contacts/${target.pid}`);
  const row = page
    .getByTestId("duplicate")
    .filter({ hasText: `Bob ${surname}` });
  await expect(row).toContainText("Likely the same person");
  await expect(row.getByTestId("duplicate-probability")).toHaveText("99%");
  await expect(row).toContainText("Agrees:");
  await expect(
    row.getByRole("link", { name: `Bob ${surname}` }),
  ).toHaveAttribute("href", `/contacts/${twin.pid}`);

  // The page shows what the service returned: the twin, ranked first.
  const body = (await (
    await request.get(`${API}/api/contacts/${target.pid}/duplicates`)
  ).json()) as { duplicates: { contact: { pid: string } }[] };
  expect(body.duplicates[0]?.contact.pid).toBe(twin.pid);
  expect(body.duplicates.map((d) => d.contact.pid)).not.toContain(stranger.pid);

  await page.goto(`/contacts/${stranger.pid}`);
  await expect(page.getByTestId("duplicates-none")).toHaveText(
    "No likely duplicates.",
  );
});

test("a duplicate is merged into this contact through the UI, and the service closes it", async ({
  page,
  request,
}) => {
  const name = `Merge${Date.now().toString(36)}`;
  const survivor = await create(request, "contacts", {
    person_ref: person(),
    display_name: `Robert ${name}`,
    job_title: "VP Sales",
  });
  const twin = await create(request, "contacts", {
    person_ref: person(),
    display_name: `Bob ${name}`,
  });
  await create(request, "activities", {
    subject_kind: "contact",
    subject_pid: twin.pid,
    kind: "note",
    summary: "Met at the conference",
  });

  await page.goto(`/contacts/${survivor.pid}`);
  await page
    .getByTestId("duplicate")
    .filter({ hasText: `Bob ${name}` })
    .getByTestId("merge")
    .click();
  // The preview comes from the service's dry run and names what will move.
  await expect(page.getByTestId("merge-moves")).toContainText("Activities: 1");
  await expect(page.getByTestId("merge-consent")).toContainText("never");
  await page.getByTestId("merge-reason").fill("Same person, nickname");
  await page.getByTestId("merge-confirm").click();
  await expect(page.getByTestId("merge-done")).toContainText(`Bob ${name}`);

  // The twin is closed, and its activity now belongs to the survivor.
  expect((await request.get(`${API}/api/contacts/${twin.pid}`)).status()).toBe(
    404,
  );
  const detail = (await (
    await request.get(`${API}/api/contacts/${survivor.pid}`)
  ).json()) as { activities: unknown[] };
  expect(detail.activities).toHaveLength(1);
});
