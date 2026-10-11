// Playwright smoke over a page.route-stubbed API (family pattern):
// the stubs mirror the service contract; any unstubbed /api/proxy
// call 404s loudly, so contract drift fails the suite.

import { expect, test, type Page } from "@playwright/test";

/**
 * Every page but / (the splash), /signin and /verify is gated on a session (CRM-T26),
 * so the smoke suite injects a fake `__Host-mxi_session` cookie
 * directly into the browser context before each test — the server
 * only checks the cookie's *presence*, never its validity
 * (`locals.sessionId` is set straight from `cookies.get`), so a
 * fabricated value is enough to pass the gate without a real
 * authentication-service round trip.
 */
async function signIn(page: Page) {
  await page.context().addCookies([
    {
      name: "__Host-mxi_session",
      value: "smoke-test-session",
      domain: "localhost",
      path: "/",
      httpOnly: true,
      secure: true,
      sameSite: "Lax",
    },
  ]);
}

const STAGES = [
  {
    pid: "s1",
    name: "Qualification",
    position: 0,
    probability_percent: 10,
    is_won: false,
    is_lost: false,
  },
  {
    pid: "s2",
    name: "Proposal",
    position: 1,
    probability_percent: 50,
    is_won: false,
    is_lost: false,
  },
  {
    pid: "s3",
    name: "Won",
    position: 2,
    probability_percent: 100,
    is_won: true,
    is_lost: false,
  },
];

const DEAL = {
  pid: "d1",
  name: "Big Deal",
  pipeline_pid: "p1",
  stage_pid: "s2",
  amount_minor: 5000000,
  currency: "GBP",
  won: false,
  closed_at: null,
};

/**
 * Choose a locale from the Lily locale picker.
 *
 * Three things make this more than a `selectOption`:
 *
 * 1. The picker renders a button plus a `ul` listbox, not a `<select>`.
 * 2. The theme picker on the same page also renders
 *    `li[role="option"]`, so the list has to be scoped — unscoped, the
 *    selector matches 58 elements here.
 * 3. The listbox is **still expanded after a pointer selection**
 *    (verified in a browser against Lily as of 2026-07-31), so
 *    clicking the button unconditionally would close it rather than
 *    open it. Open only when collapsed, which is correct either way.
 */
async function chooseLocale(page: Page, label: string) {
  const button = page.locator("header.topbar .locale-picker-button");
  const list = page.locator("ul.locale-picker-list");
  if ((await button.getAttribute("aria-expanded")) !== "true") {
    await button.click();
  }
  await expect(list).toBeVisible();
  await list
    .locator('li[role="option"]')
    .filter({ hasText: label })
    .first()
    .click();
}

test.describe("sign-in gate (CRM-T26)", () => {
  // These tests run with NO session cookie — the opposite of every
  // other test in this file — so they get their own describe block
  // rather than relying on the shared beforeEach below.
  test("a signed-out visitor is redirected from a protected page to /signin", async ({
    page,
  }) => {
    await page.goto("/deals");
    await expect(page).toHaveURL(/\/signin$/);
  });

  test("a signed-out visitor sees the splash page at /, with a sign-in link and no dashboard", async ({
    page,
  }) => {
    await page.goto("/");
    await expect(page).toHaveURL(/\/$/);
    await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
    await expect(page.getByTestId("tile-winrate")).toHaveCount(0);
    await expect(
      page.locator("header.topbar").getByRole("link", { name: "Sign in" }),
    ).toHaveAttribute("href", "/signin");
  });

  test("/signin itself stays reachable with no session", async ({ page }) => {
    const response = await page.goto("/signin");
    expect(response?.status()).toBe(200);
    await expect(page).toHaveURL(/\/signin$/);
  });
});

// Every test below exercises a gated page, so it needs a session
// already present.
test.describe("signed-in smoke coverage", () => {
  test.beforeEach(async ({ page }) => {
    await signIn(page);
    await page.route("**/api/proxy/**", (route) =>
      route.fulfill({
        status: 404,
        body: "unstubbed: " + route.request().url(),
      }),
    );
    await page.route("**/api/proxy/dashboards/sales", (route) =>
      route.fulfill({
        json: {
          win_rate: { numerator: 2, denominator: 4, value: 0.5 },
          open_deals: 8,
          pipeline_by_stage: {},
        },
      }),
    );
    await page.route("**/api/proxy/dashboards/sla", (route) =>
      route.fulfill({
        json: {
          open_tickets: 3,
          by_priority: [{ priority: "normal", open: 3, breached: 1 }],
        },
      }),
    );
    await page.route("**/api/proxy/forecast", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-18T12:00:00Z",
          open_deals: 8,
          totals_minor: { GBP: 4887500 },
        },
      }),
    );
  });

  test("dashboard renders honest KPIs from the stubbed API", async ({
    page,
  }) => {
    await page.goto("/");
    await expect(page.getByTestId("tile-winrate")).toContainText("50% (2/4)");
    await expect(page.getByTestId("tile-deals")).toContainText("8");
    await expect(page.getByTestId("tile-tickets")).toContainText("3");
    await expect(page.getByTestId("tile-forecast")).toContainText("£48,875.00");
  });

  test("deal board renders stage columns and the forecast strip", async ({
    page,
  }) => {
    await page.route("**/api/proxy/pipelines", (route) =>
      route.fulfill({
        json: [
          { pipeline: { pid: "p1", name: "New Business" }, stages: STAGES },
        ],
      }),
    );
    await page.route("**/api/proxy/insights/funnel**", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          pipeline: { pid: "p1", name: "New Business" },
          derivation: "entered from recorded to_stage audits",
          stages: [
            {
              stage: "Qualification",
              position: 0,
              is_won: false,
              is_lost: false,
              entered: 4,
              conversion_from_previous: null,
            },
            {
              stage: "Proposal",
              position: 1,
              is_won: false,
              is_lost: false,
              entered: 2,
              conversion_from_previous: {
                numerator: 2,
                denominator: 4,
                value: 0.5,
              },
            },
          ],
        },
      }),
    );
    await page.route("**/api/proxy/deals?pipeline=p1", (route) =>
      route.fulfill({ json: [DEAL] }),
    );
    await page.goto("/deals");
    const board = page.getByTestId("deal-board");
    await expect(board).toContainText("Qualification");
    await expect(board).toContainText("Proposal");
    await expect(board).toContainText("Big Deal");
    await expect(board).toContainText("£50,000.00");
    await expect(page.getByTestId("forecast")).toContainText("£48,875.00");
  });

  test("lead queue expands the score breakdown", async ({ page }) => {
    const lead = {
      pid: "l1",
      source: "referral",
      display_name: "Hot Prospect",
      email: null,
      score: 45,
      status: "contacted",
    };
    await page.route("**/api/proxy/leads", (route) =>
      route.fulfill({ json: [lead] }),
    );
    await page.route("**/api/proxy/leads/l1", (route) =>
      route.fulfill({
        json: {
          lead,
          score: {
            score: 45,
            label: "warm",
            rules: [
              { rule: "source_referral", points: 20 },
              { rule: "known_contact", points: 15 },
              { rule: "corporate_domain", points: 10 },
            ],
          },
        },
      }),
    );
    await page.goto("/leads");
    await expect(page.getByTestId("lead-queue")).toContainText("Hot Prospect");
    await page.getByRole("button", { name: "Score breakdown" }).click();
    const breakdown = page.getByTestId("breakdown");
    await expect(breakdown).toContainText("45 · warm");
    await expect(breakdown).toContainText("source_referral: 20");
  });

  test("ticket queue flags breaches; locale switch retranslates + flips RTL", async ({
    page,
  }) => {
    await page.route("**/api/proxy/tickets", (route) =>
      route.fulfill({
        json: [
          {
            pid: "t1",
            title: "Login broken",
            priority: "urgent",
            status: "open",
            first_response_due_at: "2026-07-18T10:00:00Z",
            live_first_response_breached: true,
            live_resolution_breached: false,
          },
        ],
      }),
    );
    await page.goto("/tickets");
    await expect(page.getByTestId("breached")).toBeVisible();
    await page.getByRole("button", { name: "Toggle navigation" }).click();
    await expect(page.locator("nav.primary-nav")).toContainText("Tickets");
    await chooseLocale(page, "Español");
    await expect(page.locator("nav.primary-nav")).toContainText("Contactos");
    await chooseLocale(page, "العربية");
    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
  });

  test("lead board renders columns with scored cards", async ({ page }) => {
    await page.route("**/api/proxy/leads", (route) =>
      route.fulfill({
        json: [
          {
            pid: "l1",
            source: "web form",
            display_name: "Alix Chen",
            email: null,
            score: 42,
            status: "new",
          },
          {
            pid: "l2",
            source: "referral",
            display_name: "Sam Ortiz",
            email: null,
            score: 65,
            status: "qualified",
          },
        ],
      }),
    );
    await page.goto("/leads/board");
    await expect(
      page.getByTestId("lead-board").getByText("Alix Chen"),
    ).toBeVisible();
    await expect(
      page.getByTestId("lead-board").getByText("referral · score 65"),
    ).toBeVisible();
  });

  test("ticket board renders columns with SLA badges", async ({ page }) => {
    await page.route("**/api/proxy/tickets", (route) =>
      route.fulfill({
        json: [
          {
            pid: "t1",
            title: "Login broken",
            priority: "high",
            status: "open",
            first_response_due_at: null,
            live_first_response_breached: true,
          },
        ],
      }),
    );
    await page.goto("/tickets/board");
    await expect(
      page.getByTestId("ticket-board").getByText("Login broken"),
    ).toBeVisible();
    await expect(
      page.getByTestId("ticket-board").getByText("high · SLA breached"),
    ).toBeVisible();
  });

  test("follow-ups page renders overdue aging and the calendar", async ({
    page,
  }) => {
    // The calendar always opens on today's month, so the stubbed
    // upcoming follow-up's due date is computed relative to the
    // actual test-run date — a fixed date would eventually scroll
    // outside the widget's default view.
    const today = new Date();
    const dueInThisMonth = new Date(today.getFullYear(), today.getMonth(), 18)
      .toISOString()
      .slice(0, 10);
    await page.route("**/api/proxy/insights/followups", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          note: "actor_ref is the recording actor",
          overdue: [
            {
              pid: "a1",
              kind: "call",
              summary: "Chase the proposal",
              subject_kind: "deal",
              subject_pid: "d1",
              actor_ref: "worker:x",
              due_on: "2026-07-01",
              overdue_days: 19,
            },
          ],
          upcoming_30d: [
            {
              pid: "a2",
              kind: "meeting",
              summary: "QBR",
              subject_kind: "account",
              subject_pid: "ac1",
              actor_ref: null,
              due_on: dueInThisMonth,
              overdue_days: null,
            },
          ],
          open_by_recorder: { "worker:x": 1, unattributed: 1 },
        },
      }),
    );
    await page.goto("/followups");
    await expect(
      page.getByTestId("followups-overdue").getByText("Chase the proposal"),
    ).toBeVisible();
    await expect(
      page.getByTestId("followups-overdue").getByText("19"),
    ).toBeVisible();
    const calendar = page.getByTestId("followups-calendar");
    await expect(calendar).toBeVisible();
    // Pins the widget actually rendering the event, not just its
    // wrapper — `@svar-ui/calendar-store` silently drops an all-day
    // event whose `end` is not strictly after `start` (see the fix in
    // +page.svelte), and the wrapper alone renders regardless.
    await expect(
      calendar.getByText("meeting: QBR", { exact: false }),
    ).toBeVisible();
  });

  test("executive area renders the pack, hygiene findings, and trends", async ({
    page,
  }) => {
    await page.route("**/api/proxy/insights/executive", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          window: { from: "2026-06-20T00:00:00Z", to: "2026-07-20T00:00:00Z" },
          deals_won: 3,
          deals_lost: 1,
          won_value_by_currency_minor: { GBP: 750000 },
          lost_reasons: { price: 1 },
          new_leads: 12,
          tickets_opened: 5,
          tickets_resolved: 4,
          campaigns_started: 1,
          activities_logged: 40,
          consent_withdrawals: 2,
          note: "per-currency won value is never merged",
        },
      }),
    );
    await page.route("**/api/proxy/insights/stale-deals", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          derivation: "stage entry = newest deal_stage_changed audit",
          threshold_days: 14,
          open_deals: 2,
          stale_deals: 1,
          deals: [
            {
              pid: "d9",
              name: "Sleepy deal",
              stage: "Proposal",
              owner_ref: null,
              amount_minor: 100000,
              currency: "GBP",
              days_in_stage: 30,
              stale: true,
            },
          ],
        },
      }),
    );
    await page.route("**/api/proxy/insights/pipeline-hygiene", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          threshold_days: 14,
          findings: [
            {
              rule: "open_deal_without_amount",
              detail: "an open deal carries no amount (forecast blind spot)",
            },
          ],
        },
      }),
    );
    await page.route("**/api/proxy/insights/forecast-trends", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          note: "stored snapshots only; no interpolated history",
          series: [{ taken_on: "2026-07-19", totals: { GBP: 4887500 } }],
        },
      }),
    );
    await page.goto("/executive");
    await expect(
      page.getByTestId("exec-won-value").getByText("£7,500.00"),
    ).toBeVisible();
    await expect(
      page.getByTestId("exec-stale").getByText("Sleepy deal"),
    ).toBeVisible();
    await expect(
      page.getByTestId("exec-hygiene").getByText("open_deal_without_amount"),
    ).toBeVisible();
    await expect(
      page.getByTestId("exec-trends").getByText("£48,875.00"),
    ).toBeVisible();
  });

  test("dpo area renders coverage, sources, and duplicate hygiene", async ({
    page,
  }) => {
    await page.route("**/api/proxy/insights/consent-by-account", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          window_days: 30,
          note: "coverage counts verbatim",
          accounts: [
            {
              pid: "ac1",
              display_name: "Meridian University",
              consent_coverage: { granted: 3 },
              withdrawals_in_window: 1,
            },
          ],
        },
      }),
    );
    await page.route("**/api/proxy/insights/dpo", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          note: "identity dedup stays upstream in the person service",
          contacts: 3,
          consent_coverage: { granted: 2, withdrawn: 1 },
          window_days: 30,
          withdrawals_in_window: 1,
          consent_events_by_source: { "web form": 2, "email link": 1 },
          duplicate_person_refs: [
            {
              person_ref: "person:abc",
              contacts: [
                { pid: "c1", display_name: "Row One" },
                { pid: "c2", display_name: "Row Two" },
              ],
            },
          ],
        },
      }),
    );
    await page.goto("/dpo");
    await expect(
      page.getByTestId("dpo-tiles").getByText("consent: granted"),
    ).toBeVisible();
    await expect(
      page.getByTestId("dpo-sources").getByText("web form"),
    ).toBeVisible();
    await expect(
      page.getByTestId("dpo-duplicates").getByText("person:abc"),
    ).toBeVisible();
  });

  test("deal board funnel strip shows honest conversion ratios", async ({
    page,
  }) => {
    await page.route("**/api/proxy/pipelines", (route) =>
      route.fulfill({
        json: [
          { pipeline: { pid: "p1", name: "New Business" }, stages: STAGES },
        ],
      }),
    );
    await page.route("**/api/proxy/deals**", (route) =>
      route.fulfill({ json: [DEAL] }),
    );
    await page.route("**/api/proxy/insights/funnel**", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          pipeline: { pid: "p1", name: "New Business" },
          derivation: "entered from recorded to_stage audits",
          stages: [
            {
              stage: "Qualification",
              position: 0,
              is_won: false,
              is_lost: false,
              entered: 4,
              conversion_from_previous: null,
            },
            {
              stage: "Proposal",
              position: 1,
              is_won: false,
              is_lost: false,
              entered: 2,
              conversion_from_previous: {
                numerator: 2,
                denominator: 4,
                value: 0.5,
              },
            },
          ],
        },
      }),
    );
    await page.goto("/deals");
    await expect(page.getByTestId("pipeline-select")).toBeVisible();
    await expect(
      page.getByTestId("deal-funnel").getByText("50% (2/4)"),
    ).toBeVisible();
  });

  test("engagement area renders cadence, workload, and member health", async ({
    page,
  }) => {
    await page.route("**/api/proxy/insights/cadence**", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          derivation: "touch = a recorded activity",
          threshold_days: 30,
          untouched_contacts: [
            {
              pid: "c9",
              display_name: "Silent Sam",
              stakeholder_role: "regulator",
              days_since_touch: 44,
              has_next_touch: false,
            },
          ],
          untouched_accounts: [],
          contacts_without_next_touch: 5,
        },
      }),
    );
    await page.route("**/api/proxy/insights/engagement**", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          window_days: 90,
          touches: 12,
          per_recorder_month: { "worker:x 2026-07": 12 },
          per_kind: { meeting: 7, call: 5 },
          sentiment: { positive: 4, unrecorded: 8 },
          note: "recorded declarations only",
        },
      }),
    );
    await page.route("**/api/proxy/insights/members**", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          derivation: "account touch includes its contacts' activities",
          threshold_days: 30,
          silent_accounts: 1,
          accounts: [
            {
              pid: "ac1",
              display_name: "Meridian University",
              tier: "standard",
              stakeholder_role: "partner",
              membership: {
                status: "active",
                joined_on: "2024-01-01",
                renewal_on: "2026-08-01",
              },
              contacts: 3,
              days_since_touch: 2,
              silent: false,
              open_followups: 1,
              open_tickets: 0,
            },
          ],
        },
      }),
    );
    await page.goto("/engagement");
    await expect(
      page.getByTestId("cadence-contacts").getByText("Silent Sam"),
    ).toBeVisible();
    await expect(
      page.getByTestId("workload-sentiment").getByText("unrecorded"),
    ).toBeVisible();
    await expect(
      page.getByTestId("member-health").getByText("Meridian University"),
    ).toBeVisible();
  });

  test("partners area renders the register, grid, partnerships, and renewals", async ({
    page,
  }) => {
    await page.route("**/api/proxy/insights/stakeholders", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          note: "declared, never inferred",
          by_role: {
            partner: [
              {
                pid: "c1",
                display_name: "Prof Reyes",
                marketing_consent: "granted",
                influence: 4,
                interest: 5,
                days_since_touch: 3,
              },
            ],
          },
          grid: { p4i5: 1 },
          stakeholders_without_grid_scores: 0,
          undeclared_contacts: 7,
          account_roles: [
            {
              pid: "ac1",
              display_name: "Meridian University",
              role: "partner",
            },
          ],
        },
      }),
    );
    await page.route("**/api/proxy/insights/partnerships", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          by_kind: { university: 1 },
          by_stage: { pilot: 1 },
          register: [
            {
              pid: "pn1",
              account_pid: "ac1",
              account: "Meridian University",
              kind: "university",
              stage: "pilot",
              summary: "Joint ML lab",
              started_on: null,
            },
          ],
        },
      }),
    );
    await page.route("**/api/proxy/insights/memberships**", (route) =>
      route.fulfill({
        json: {
          as_of: "2026-07-20T00:00:00Z",
          window_days: 90,
          memberships: 1,
          renewals_due: [
            {
              pid: "m1",
              account: "Meridian University",
              status: "active",
              joined_on: "2024-01-01",
              renewal_on: "2026-08-01",
            },
          ],
          lapsed: [],
        },
      }),
    );
    await page.goto("/partners");
    await expect(
      page.getByTestId("stakeholder-register").getByText("Prof Reyes"),
    ).toBeVisible();
    await expect(page.getByTestId("stakeholder-grid")).toBeVisible();
    await expect(
      page.getByTestId("partnership-register").getByText("Joint ML lab"),
    ).toBeVisible();
    await expect(
      page.getByTestId("membership-renewals").getByText("2026-08-01"),
    ).toBeVisible();
  });
});

test.describe("contact subject rights", () => {
  test.beforeEach(async ({ page }) => {
    await signIn(page);
  });

  const CONTACT = {
    pid: "c1",
    display_name: "Ada Subject",
    job_title: "CTO",
    status: "active",
    marketing_consent: "granted",
  };

  async function stubContact(page: Page) {
    await page.route("**/api/proxy/contacts/c1", (route) =>
      route.fulfill({
        json: { contact: CONTACT, activities: [], deals: [], tickets: [] },
      }),
    );
    await page.route("**/api/proxy/contacts", (route) =>
      route.fulfill({ json: [] }),
    );
  }

  test("offers a subject-access download and a confirm-gated erase", async ({ page }) => {
    await stubContact(page);
    let erased = false;
    await page.route("**/api/proxy/contacts/c1/erase", (route) => {
      erased = true;
      return route.fulfill({ json: { erased: "c1", note: "anonymised" } });
    });
    page.on("dialog", (dialog) => void dialog.accept());
    await page.goto("/contacts/c1");
    await expect(page.getByTestId("subject-access")).toHaveAttribute(
      "href",
      "/api/proxy/contacts/c1/subject-access",
    );
    await page.getByTestId("erase").click();
    await expect(page).toHaveURL(/\/contacts$/);
    expect(erased).toBe(true);
  });

  test("declining the confirm does not erase", async ({ page }) => {
    await stubContact(page);
    let erased = false;
    await page.route("**/api/proxy/contacts/c1/erase", (route) => {
      erased = true;
      return route.fulfill({ json: {} });
    });
    page.on("dialog", (dialog) => void dialog.dismiss());
    await page.goto("/contacts/c1");
    await page.getByTestId("erase").click();
    await expect(page).toHaveURL(/\/contacts\/c1$/);
    expect(erased).toBe(false);
  });

  test("surfaces the service's 422 reason when erase is blocked", async ({ page }) => {
    await stubContact(page);
    await page.route("**/api/proxy/contacts/c1/erase", (route) =>
      route.fulfill({
        status: 422,
        json: { description: "contact has an active nurture enrolment" },
      }),
    );
    page.on("dialog", (dialog) => void dialog.accept());
    await page.goto("/contacts/c1");
    await page.getByTestId("erase").click();
    await expect(page.getByTestId("action-error")).toContainText(
      "active nurture enrolment",
    );
    await expect(page).toHaveURL(/\/contacts\/c1$/);
  });
});

test.describe("masked amounts (CRM-T32)", () => {
  test.beforeEach(async ({ page }) => {
    await signIn(page);
  });

  test("campaign funnel renders masked revenue and ROI as dashes", async ({ page }) => {
    const campaign = {
      pid: "k1",
      name: "Spring push",
      status: "completed",
      recipients: 10,
      delivered: 9,
      opened: 5,
      clicked: 2,
      currency: "GBP",
      cost_minor: null,
    };
    await page.route("**/api/proxy/campaigns", (route) => route.fulfill({ json: [campaign] }));
    await page.route("**/api/proxy/campaigns/k1/funnel", (route) =>
      route.fulfill({
        json: { campaign, leads: 3, won_deals: 1, won_revenue_minor: null, roi: null },
      }),
    );
    await page.goto("/campaigns");
    await page.getByRole("button", { name: "Funnel" }).click();
    const funnel = page.getByTestId("funnel");
    await expect(funnel).toContainText("—");
    await expect(funnel).not.toContainText("0%");
  });
});

test("an idle signed-in session is signed out (CRM-T43)", async ({ page }) => {
  await signIn(page);
  let signedOut = false;
  await page.route("**/signout", (route) => {
    signedOut = true;
    return route.fulfill({ status: 303, headers: { location: "/" } });
  });
  await page.clock.install();
  await page.route("**/api/proxy/**", (route) => route.fulfill({ json: [] }));
  await page.goto("/contacts");
  await expect(page.locator("main")).toBeVisible();
  await page.clock.runFor(15 * 60 * 1000 + 1000);
  await expect.poll(() => signedOut).toBe(true);
  await expect(page).toHaveURL(/\/signin$/);
});

test.describe("break-glass page (CRM-T44)", () => {
  test.beforeEach(async ({ page }) => {
    await signIn(page);
  });

  test("a thin reason cannot be submitted; a good one shows the expiry", async ({ page }) => {
    let body: { reason?: string; minutes?: number } = {};
    let idempotencyKey: string | undefined;
    await page.route("**/api/proxy/break-glass", (route) => {
      idempotencyKey = route.request().headers()["idempotency-key"];
      body = route.request().postDataJSON();
      return route.fulfill({
        json: { pid: "g1", expires_at: new Date(Date.now() + 30 * 60_000).toISOString(), minutes: 30 },
      });
    });
    await page.goto("/break-glass");
    await expect(page.getByTestId("bg-request")).toBeDisabled();
    await page.getByLabel("Reason").fill("too short");
    await expect(page.getByTestId("bg-request")).toBeDisabled();
    await page.getByLabel("Reason").fill("Patient collapsed, covering colleague absent");
    await page.getByTestId("bg-request").click();
    await expect(page.getByTestId("bg-granted")).toBeVisible();
    expect(body.reason).toBe("Patient collapsed, covering colleague absent");
    expect(body.minutes).toBe(30);
    // CRM-T52: every POST carries an Idempotency-Key.
    expect(idempotencyKey).toMatch(/^[\x21-\x7e]{1,255}$/);
  });

  test("the review page lists grants and records a review", async ({ page }) => {
    const grant = {
      pid: "g1",
      subject: "dr-reader",
      reason: "Patient collapsed, covering colleague absent",
      granted_at: "2026-10-10T09:00:00Z",
      expires_at: "2026-10-10T09:30:00Z",
      reviewed_at: null,
      reviewed_by: null,
      review_note: null,
    };
    let reviewed: { note?: string } = {};
    await page.route("**/api/proxy/break-glass", (route) =>
      route.fulfill({ json: { unreviewed: reviewed.note ? 0 : 1, grants: [reviewed.note ? { ...grant, reviewed_at: "2026-10-10T10:00:00Z", reviewed_by: "dpo", review_note: reviewed.note } : grant] } }),
    );
    await page.route("**/api/proxy/break-glass/g1/review", (route) => {
      reviewed = route.request().postDataJSON();
      return route.fulfill({ json: { pid: "g1", reviewed_by: "dpo" } });
    });
    await page.goto("/break-glass/review");
    await expect(page.getByTestId("unreviewed")).toContainText("1");
    const record = page.getByRole("button", { name: "Record review" });
    await expect(record).toBeDisabled();
    await page.getByLabel("Review note").fill("Confirmed against the incident log");
    await record.click();
    await expect(page.getByTestId("unreviewed")).toContainText("0");
    await expect(page.getByTestId("grants")).toContainText("Confirmed against the incident log");
    expect(reviewed.note).toBe("Confirmed against the incident log");
  });

  test("the review page shows a refusal for a non-reviewer", async ({ page }) => {
    await page.route("**/api/proxy/break-glass", (route) =>
      route.fulfill({ status: 403, json: { description: "forbidden" } }),
    );
    await page.goto("/break-glass/review");
    await expect(page.getByTestId("error")).toContainText("forbidden");
  });

  test("shows the service's refusal", async ({ page }) => {
    await page.route("**/api/proxy/break-glass", (route) =>
      route.fulfill({ status: 403, json: { description: "denied by policy" } }),
    );
    await page.goto("/break-glass");
    await page.getByLabel("Reason").fill("Patient collapsed, covering colleague absent");
    await page.getByTestId("bg-request").click();
    await expect(page.getByTestId("bg-error")).toContainText("denied by policy");
  });
});

test("responses carry the security headers and a CSP (CRM-T33)", async ({ page }) => {
  const response = await page.goto("/tour");
  const headers = response?.headers() ?? {};
  expect(headers["x-content-type-options"]).toBe("nosniff");
  expect(headers["x-frame-options"]).toBe("DENY");
  expect(headers["referrer-policy"]).toBe("no-referrer");
  expect(headers["strict-transport-security"]).toContain("max-age=");
  expect(headers["content-security-policy"]).toContain("frame-ancestors 'none'");
});

test("the accessibility statement is public and linked from the footer (CRM-T45)", async ({ page }) => {
  await page.goto("/tour");
  await page.getByRole("link", { name: "Accessibility" }).click();
  await expect(page).toHaveURL(/\/accessibility$/);
  await expect(page.getByRole("heading", { name: "Accessibility statement" })).toBeVisible();
  await expect(page.getByText("not yet shown to comply")).toBeVisible();
});

test("the tour renders for a signed-out visitor", async ({ page }) => {
  await page.goto("/tour");
  await expect(page).toHaveURL(/\/tour$/);
  await expect(page.locator(".sections section:not(.closing)")).toHaveCount(7);
  await expect(page.locator(".steps li")).toHaveCount(28);
});
