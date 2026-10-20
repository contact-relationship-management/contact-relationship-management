// Links to other systems on the contact page, driven through the UI against the
// real service (CRM-R27): the service checks the address; the page shows its reason.

import { expect, test } from "@playwright/test";

const API = process.env.CRM_API_URL ?? "http://localhost:5160";

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

test("a link is added, listed, refused when it is on the wrong domain, and removed", async ({
  page,
  request,
}) => {
  const created = await request.post(`${API}/api/contacts`, {
    data: {
      person_ref: `person:${crypto.randomUUID()}`,
      display_name: "Linked Person",
    },
  });
  expect(created.ok()).toBeTruthy();
  const { pid } = (await created.json()) as { pid: string };

  await page.goto(`/contacts/${pid}`);
  await expect(page.getByTestId("links-none")).toBeVisible();

  await page.getByTestId("link-system").selectOption("salesforce");
  await page
    .getByTestId("link-url")
    .fill("https://acme.lightning.force.com/lightning/r/Contact/003/view");
  await page.getByTestId("link-label").fill("Salesforce contact");
  await page.getByTestId("link-add").click();
  await expect(page.getByTestId("link")).toHaveCount(1);
  await expect(page.getByTestId("link-open")).toHaveAttribute(
    "href",
    "https://acme.lightning.force.com/lightning/r/Contact/003/view",
  );

  // The service, not the page, refuses a lookalike host.
  await page.getByTestId("link-system").selectOption("hubspot");
  await page.getByTestId("link-url").fill("https://hubspot.com.evil.example/x");
  await page.getByTestId("link-add").click();
  await expect(page.getByTestId("link-error")).toContainText("hubspot");
  await expect(page.getByTestId("link")).toHaveCount(1);

  await page.getByTestId("link-remove").click();
  await expect(page.getByTestId("links-none")).toBeVisible();
});
