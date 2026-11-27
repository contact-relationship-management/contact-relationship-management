import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import * as crm from "../../src/lib/api/crm";
import { demoHandle, resetDemo } from "../../src/lib/server/demo";

const call = (method: string, path: string, body: unknown = null) => {
  const url = new URL(path, "https://demo.invalid/");
  return demoHandle(
    method,
    url.pathname.replace(/^\//, ""),
    url.searchParams,
    body,
  );
};

const succeeds = (method: string, path: string, body: unknown = null) => {
  const reply = call(method, path, body);
  expect(
    reply.status,
    `${method} ${path}: ${JSON.stringify(reply.body)}`,
  ).toBeLessThan(300);
  return reply.body as Record<string, unknown>;
};

const fails = (
  method: string,
  path: string,
  body: unknown = null,
  status = 422,
) => {
  const reply = call(method, path, body);
  expect(reply.status, `${method} ${path}: ${JSON.stringify(reply.body)}`).toBe(
    status,
  );
  return reply.body;
};

beforeEach(() => resetDemo());
afterEach(() => vi.unstubAllGlobals());

const connectApiToDemo = () =>
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: string | URL | Request, init?: RequestInit) => {
      const url = new URL(String(input), "https://demo.invalid");
      const reply = demoHandle(
        init?.method ?? "GET",
        url.pathname.replace(/^\/api\/proxy\//, ""),
        url.searchParams,
        init?.body ? JSON.parse(String(init.body)) : null,
      );
      return new Response(JSON.stringify(reply.body), {
        status: reply.status,
        headers: { "content-type": "application/json" },
      });
    }),
  );

describe("the demo supports complete editing workflows", () => {
  it("edits records, positions cards, links records, and restores a merged contact", () => {
    succeeds("PUT", "contacts/con-1", {
      display_name: "Alex Renamed",
      given_name: "Alex",
      family_name: "Example",
      job_title: null,
    });
    succeeds("PUT", "accounts/acc-1", {
      display_name: "Acme Renamed",
      tier: "gold",
      industry: null,
    });
    succeeds("PUT", "leads/lead-1", { display_name: "Lead Renamed" });
    succeeds("PUT", "tickets/tkt-1", { title: "Ticket Renamed" });
    succeeds("PUT", "deals/deal-1", {
      name: "Deal Renamed",
      amount_minor_text: "9007199254740991",
      expected_close_on: "2030-01-01",
    });
    for (const [kind, pid] of [
      ["leads", "lead-1"],
      ["tickets", "tkt-1"],
      ["deals", "deal-1"],
    ])
      succeeds("PUT", `${kind}/${pid}/position`, { position: 1 });

    const link = succeeds("POST", "contacts/con-1/external-links", {
      system: "other",
      url: "https://example.test/contact/1",
      label: "Example",
    });
    succeeds("GET", "contacts/con-1/external-links");
    succeeds("DELETE", `external-links/${String(link.pid)}`);

    succeeds("POST", "contacts/con-2/merge", {
      into_pid: "con-1",
      reason: "Duplicate created during the test",
      dry_run: true,
    });
    succeeds("POST", "contacts/con-2/merge", {
      into_pid: "con-1",
      reason: "Duplicate created during the test",
      dry_run: false,
    });
    succeeds("GET", "contacts/con-1/merges");
    succeeds("POST", "contacts/con-2/unmerge", {
      reason: "Restore the separate synthetic contact",
      dry_run: true,
    });
    succeeds("POST", "contacts/con-2/unmerge", {
      reason: "Restore the separate synthetic contact",
      dry_run: false,
    });
  });

  it("walks sales, support, campaign, workflow, and notification state machines", () => {
    succeeds("POST", "leads/lead-1/status", { to: "contacted" });
    succeeds("POST", "deals/deal-1/stage", { stage_pid: "stg-5" });
    succeeds("POST", "deals/deal-1/reopen", { reason: "Requalified" });
    succeeds("POST", "tickets/tkt-1/status", { to: "pending" });
    succeeds("POST", "tickets/tkt-1/customer-reply");
    succeeds("GET", "tickets/tkt-1/suggested-articles");
    succeeds("POST", "articles/kb-1/status", { to: "published" });

    const campaign = succeeds("POST", "campaigns", {
      name: "Coverage campaign",
      currency: "EUR",
      subject: "Hello",
      body: "{{unsubscribe_url}}",
    });
    succeeds("POST", `campaigns/${String(campaign.pid)}/status`, {
      to: "scheduled",
    });
    succeeds("GET", `campaigns/${String(campaign.pid)}/funnel`);
    succeeds("POST", "campaigns/camp-3/run");

    const workflow = succeeds("POST", "workflows", {
      name: "Coverage workflow",
      trigger: "contact_created",
      actions: [{ type: "create_task", summary: "Follow up" }],
    });
    const workflowPid = String(workflow.pid);
    succeeds("POST", `workflows/${workflowPid}/dry-run`);
    succeeds("GET", `workflows/${workflowPid}/runs`);
    succeeds("PUT", `workflows/${workflowPid}`, {
      name: "Coverage workflow changed",
      actions: [{ type: "create_task", summary: "Call" }],
    });
    succeeds("DELETE", `workflows/${workflowPid}`);

    succeeds("PUT", "notifications/note-1/read");
    succeeds("POST", "notifications/read-all");
  });

  it("creates, updates, runs, and deletes user-defined configuration", () => {
    const view = succeeds("POST", "views", {
      kind: "deal",
      name: "Coverage view",
      filter: { currency: ["EUR"] },
      sort: { field: "name", direction: "asc" },
      columns: ["name"],
      shared: false,
    });
    succeeds("PUT", `views/${String(view.pid)}`, { name: "Changed view" });
    succeeds("DELETE", `views/${String(view.pid)}`);

    const template = succeeds("POST", "email-templates", {
      name: "Coverage template",
      language: "en",
      subject: "Hello {{given_name}}",
      body: "Read this {{unsubscribe_url}}",
      fallbacks: {},
    });
    succeeds("POST", "email-templates/preview", {
      language: "en",
      subject: "Hello {{given_name}}",
      body: "Read this {{unsubscribe_url}}",
      fallbacks: {},
      blank: ["given_name"],
    });
    succeeds("PUT", `email-templates/${String(template.pid)}`, {
      name: "Changed template",
      language: "en",
      subject: "Hello",
      body: "{{unsubscribe_url}}",
      fallbacks: {},
    });
    succeeds("DELETE", `email-templates/${String(template.pid)}`);

    const field = succeeds("POST", "custom-fields", {
      entity: "contact",
      key: "coverage_level",
      label: "Coverage level",
      kind: "number",
      sensitivity: "business",
    });
    succeeds("PUT", "contacts/con-1/custom-fields", {
      values: { coverage_level: 95 },
    });
    succeeds("GET", "contacts/con-1/custom-fields");
    succeeds("DELETE", `custom-fields/${String(field.pid)}`);

    succeeds("PUT", "contacts/con-1/tags", { tags: ["priority", "renewal"] });
    succeeds("GET", "contacts/con-1/tags");
    succeeds("GET", "contacts?tag=priority");

    const attachment = succeeds(
      "POST",
      "contacts/con-1/attachments?name=coverage.txt&size=12",
    );
    succeeds("GET", "contacts/con-1/attachments");
    succeeds("DELETE", `attachments/${String(attachment.pid)}`);
  });

  it("previews, commits, reads, and undoes each import shape", () => {
    const leadCsv = "Name,Email,Company\nAda Example,ada@example.test,Example";
    succeeds("POST", "lead-imports/preview", { csv: leadCsv });
    const leadImport = succeeds("POST", "lead-imports", {
      csv: leadCsv,
      mapping: { display_name: "Name", email: "Email", company: "Company" },
      source_note: "Synthetic coverage data",
      obtained_on: "2026-11-19",
      skip_likely_duplicates: true,
    });
    succeeds("POST", `lead-imports/${String(leadImport.pid)}/undo`);

    for (const [kind, csv, mapping] of [
      [
        "contact",
        "Name,Email\nAda Example,ada@example.test",
        { display_name: "Name", email: "Email" },
      ],
      [
        "account",
        "Name,Tier\nExample Ltd,gold",
        { display_name: "Name", tier: "Tier" },
      ],
    ] as const) {
      const path = `${kind}-imports`;
      succeeds("POST", `${path}/preview`, { csv });
      const made = succeeds("POST", path, {
        csv,
        mapping,
        source_note: "Synthetic coverage data",
        obtained_on: "2026-11-19",
        skip_likely_duplicates: true,
      });
      succeeds("POST", `${path}/${String(made.pid)}/undo`);
    }
  });

  it("reports precise validation and conflict failures without changing data", () => {
    fails("GET", "contacts/missing", null, 404);
    fails("PUT", "contacts/missing", { display_name: "Missing" }, 404);
    fails("PUT", "contacts/con-1", {});
    fails("PUT", "contacts/con-1", { display_name: " " });
    fails("GET", "contacts/missing/duplicates", null, 404);
    fails("POST", "contacts/con-1/merge", { into_pid: "con-2" });
    fails("POST", "contacts/con-1/merge", {
      into_pid: "con-1",
      reason: "Same record",
    });
    fails(
      "POST",
      "contacts/missing/merge",
      {
        into_pid: "con-1",
        reason: "Missing record",
      },
      404,
    );
    fails("POST", "contacts/con-2/unmerge", { reason: "Not merged" }, 409);
    fails("DELETE", "external-links/missing", null, 404);

    for (const [system, url] of [
      ["unknown", "https://example.test"],
      ["other", "not a URL"],
      ["other", "http://example.test"],
      ["other", "https://name:secret@example.test"],
      ["hubspot", "https://example.test"],
    ])
      fails("POST", "contacts/con-1/external-links", { system, url });
    fails(
      "POST",
      "contacts/missing/external-links",
      {
        system: "other",
        url: "https://example.test",
      },
      404,
    );
    succeeds("POST", "contacts/con-1/external-links", {
      system: "other",
      url: "https://example.test/duplicate",
    });
    fails(
      "POST",
      "contacts/con-1/external-links",
      {
        system: "other",
        url: "https://example.test/duplicate",
      },
      409,
    );

    fails("POST", "contacts/con-1/erase", null);
    fails("POST", "contacts/con-3/erase", null);
    fails("GET", "leads/missing", null, 404);
    fails("POST", "leads/missing/status", { to: "contacted" }, 404);
    fails("POST", "leads/lead-1/status", {});
    fails("POST", "deals/missing/stage", { stage_pid: "stg-1" }, 404);
    fails("POST", "deals/deal-1/stage", { stage_pid: "missing" });
    fails("POST", "deals/deal-1/reopen", { reason: " " });
    fails("POST", "deals/deal-1/reopen", { reason: "Still open" });
    fails("PUT", "deals/missing", { name: "Missing" }, 404);
    fails("PUT", "deals/deal-1", {});
    fails("PUT", "deals/deal-1", { name: " " });
    fails("PUT", "deals/deal-1", { amount_minor: -1 });
    fails("PUT", "deals/deal-1", { amount_minor_text: "-1" });
    fails("PUT", "deals/deal-1", {
      amount_minor: 1,
      amount_minor_text: "1",
    });
    fails("PUT", "tickets/missing", { title: "Missing" }, 404);
    fails("PUT", "tickets/tkt-1", {});
    fails("PUT", "tickets/tkt-1", { title: " " });
  });

  it("rejects unsafe configuration, reporting, and compensation requests", () => {
    fails("POST", "workflows", {});
    fails("POST", "workflows", { name: "No actions" });
    fails("GET", "workflows/missing", null, 404);
    fails("POST", "reports", {});
    fails("GET", "reports/missing/run", null, 404);
    fails("POST", "reports/run", {
      kind: "contact",
      group_by: "status",
      measure: "sum_amount",
      filter: {},
    });
    fails("POST", "reports/run", {
      kind: "deal",
      group_by: "currency",
      measure: "sum_amount",
      filter: {},
      rate_basis: "close_day",
    });
    fails("POST", "reports/run", {
      kind: "deal",
      group_by: "currency",
      measure: "sum_amount",
      filter: {},
      convert_to: "EUR",
      rate_basis: "latest",
    });
    fails("POST", "reports/run", {
      kind: "deal",
      group_by: "currency",
      measure: "sum_amount",
      filter: {},
      convert_to: "EUR",
      rate_basis: "close_day",
      rate_date: "2026-01-01",
    });

    fails("PUT", "settings/organization", {
      time_zone: "not/a zone",
      fiscal_year_start_month: 1,
    });
    fails("PUT", "settings/organization", {
      time_zone: "UTC",
      fiscal_year_start_month: 13,
    });
    fails("PUT", "settings/organization", {
      time_zone: "UTC",
      fiscal_year_start_month: 1,
      stalled_after_days: 0,
    });
    fails("PUT", "fx-rates", {
      from: "XXX",
      to: "EUR",
      rate: "1",
      effective_on: "2026-01-01",
      source: "Test",
    });
    fails("PUT", "fx-rates", {
      from: "EUR",
      to: "EUR",
      rate: "1",
      effective_on: "2026-01-01",
      source: "Test",
    });
    fails("PUT", "fx-rates", {
      from: "EUR",
      to: "GBP",
      rate: "0",
      effective_on: "2026-01-01",
      source: "Test",
    });
    fails("PUT", "fx-rates", {
      from: "EUR",
      to: "GBP",
      rate: "1",
      source: "Test",
    });
    fails("DELETE", "fx-rates/missing", null, 404);

    fails("PUT", "commission-plans", { name: "", tiers: [] });
    fails("PUT", "commission-plans", {
      name: "Bad tiers",
      tiers: [{ from_percent: 10, rate_bps: 500 }],
    });
    fails("DELETE", "commission-plans/missing", null, 404);
    fails("DELETE", "commission-plans/plan-1", null, 409);
    fails("PUT", "quotas", {
      owner_ref: "owner-ayo",
      currency: "XXX",
      amount_minor: 1,
      period_from: "2026-01-01",
      period_to: "2026-12-31",
    });
    fails("PUT", "quotas", {
      owner_ref: "owner-ayo",
      currency: "EUR",
      amount_minor: 0,
      period_from: "2026-01-01",
      period_to: "2026-12-31",
    });
    fails("PUT", "quotas", {
      owner_ref: "owner-ayo",
      currency: "EUR",
      amount_minor: 1,
      period_from: "2026-12-31",
      period_to: "2026-01-01",
    });
    fails("DELETE", "quotas/missing", null, 404);
    fails("PUT", "quotas/splits/missing", { shares: [] }, 404);
    fails("PUT", "quotas/splits/deal-9", {
      shares: [{ owner_ref: "owner-bea", share_bps: 9000 }],
    });
    fails("DELETE", "quotas/splits/deal-9", null, 404);
    fails("PUT", "quotas/clawbacks/deal-1", {
      owner_ref: "owner-ayo",
      amount_minor: 1,
    });
    fails("DELETE", "quotas/clawbacks/deal-9?owner_ref=missing", null, 404);
    fails("PUT", "quotas/teams", { name: "", members: [] });
    fails("PUT", "quotas/teams", {
      name: "Duplicates",
      members: ["owner-ayo", "owner-ayo"],
    });
    fails("DELETE", "quotas/teams/missing", null, 404);
    fails("PUT", "quotas/territories", { name: "", accounts: [] });
    fails("PUT", "quotas/territories", {
      name: "Duplicates",
      accounts: ["org-demo-1", "org-demo-1"],
    });
    fails("DELETE", "quotas/territories/missing", null, 404);
    fails("GET", "quotas/territory-attainment?from=2026-12-31&to=2026-01-01");
  });

  it("rejects malformed metadata, attachments, tags, and templates", () => {
    fails("PUT", "contacts/con-1/tags", { tags: [""] });
    fails("PUT", "contacts/con-1/tags", {
      tags: Array.from({ length: 21 }, (_, i) => `tag-${i}`),
    });
    fails("POST", "contacts/con-1/attachments");
    fails("POST", "contacts/con-1/attachments?name=malware.exe");
    fails("DELETE", "attachments/missing", null, 404);
    fails("POST", "custom-fields", { key: "Not Valid" });
    const field = succeeds("POST", "custom-fields", {
      entity: "contact",
      key: "risk_level",
      label: "Risk level",
      kind: "choice",
      choices: ["low", "high"],
      sensitivity: "business",
    });
    fails("POST", "custom-fields", {
      entity: "contact",
      key: "risk_level",
    });
    fails("PUT", "contacts/con-1/custom-fields", { values: {} });
    fails("PUT", "contacts/con-1/custom-fields", {
      values: { missing_field: "value" },
    });
    fails("PUT", "contacts/con-1/custom-fields", {
      values: { risk_level: "medium" },
    });
    succeeds("PUT", "contacts/con-1/custom-fields", {
      values: { risk_level: "high" },
    });
    succeeds("PUT", "contacts/con-1/custom-fields", {
      values: { risk_level: null },
    });
    fails("DELETE", "custom-fields/missing", null, 404);
    succeeds("DELETE", `custom-fields/${String(field.pid)}`);

    fails("GET", "email-templates/tokens?language=not_a_tag");
    fails("POST", "email-templates/preview", {
      subject: "Hello {{unknown_token}}",
      body: "{{unsubscribe_url}}",
      fallbacks: {},
    });
    fails("POST", "email-templates", {
      subject: "Hello",
      body: "{{unsubscribe_url}}",
      fallbacks: {},
    });
    fails("POST", "email-templates", {
      name: "Unknown token",
      subject: "{{unknown_token}}",
      body: "{{unsubscribe_url}}",
      fallbacks: {},
    });
    fails("POST", "email-templates", {
      name: "Missing unsubscribe",
      subject: "Hello",
      body: "Body only",
      fallbacks: {},
    });
    fails("GET", "email-templates/missing", null, 404);
  });

  it("rejects malformed operational mutations and import previews", () => {
    fails("POST", "break-glass", { reason: "too short", minutes: 15 });
    const grant = succeeds("POST", "break-glass", {
      reason: "A sufficiently detailed synthetic emergency reason",
      minutes: 100,
    });
    succeeds("GET", "break-glass");
    fails("POST", "break-glass/missing/review", { note: "Reviewed" }, 404);
    fails("POST", `break-glass/${String(grant.pid)}/review`, {
      note: "Self review is forbidden",
    });

    fails("POST", "views", {});
    fails("PUT", "views/missing", { name: "Missing" }, 404);
    fails("PUT", "accounts/missing", { display_name: "Missing" }, 404);
    fails("PUT", "accounts/acc-1", {});
    fails("PUT", "accounts/acc-1", { display_name: " " });
    fails("GET", "accounts/missing", null, 404);
    fails("GET", "accounts/missing/clv", null, 404);

    fails("POST", "campaigns", {});
    fails("POST", "campaigns", { name: "Bad currency", currency: "XXX" });
    fails("POST", "campaigns", {
      name: "Mixed content",
      currency: "EUR",
      template_pid: "template-1",
      subject: "Also supplied",
    });
    fails("POST", "campaigns/missing/run", null, 404);
    fails("POST", "campaigns/camp-1/run");
    fails("GET", "campaigns/missing/funnel", null, 404);
    fails("GET", "campaigns/camp-1/funnel?rate_basis=close_day");
    fails("GET", "campaigns/camp-1/funnel?convert=true&rate_basis=latest");
    fails(
      "GET",
      "campaigns/camp-1/funnel?convert=true&rate_basis=close_day&rate_date=2026-01-01",
    );
    fails("POST", "tickets/missing/customer-reply", null, 404);
    fails("GET", "tickets/missing", null, 404);
    fails("GET", "articles/missing/versions", null, 404);
    fails("GET", "insights/stuck-tickets?days=0");
    fails("GET", "insights/executive?from=2026-02-01&to=2026-01-01");
    expect(
      succeeds("GET", "insights/sla-attainment?from=2026-01-01&to=2026-01-31")
        .window,
    ).toEqual({ from: "2026-01-01", to: "2026-01-31" });
    fails("POST", "tickets/missing/assign", { reason: "x" }, 404);
    fails("POST", "tickets/tkt-1/assign", { assignee_ref: "person:ada" });
    fails("POST", "tickets/tkt-1/assign", { assignee_ref: "ada", reason: "x" });
    fails("GET", "tickets/missing/suggested-articles", null, 404);

    fails("POST", "lead-imports/preview", { csv: "" });
    fails("POST", "lead-imports/preview", {
      csv: "Name,Email\nAda,ada@example.test",
      mapping: { display_name: "Unknown" },
    });
    fails("GET", "lead-imports/missing", null, 404);
  });

  it("rejects invalid attribution controls independently", () => {
    for (const path of [
      "attribution?model=unknown&window_days=90",
      "attribution?model=linear&window_days=0",
      "attribution?model=linear&window_days=90&touches=robot",
      "attribution?model=linear&window_days=90&half_life_days=7",
      "attribution?model=time_decay&window_days=90&half_life_days=2",
      "attribution?model=linear&window_days=90&end_percent=40",
      "attribution?model=position&window_days=90&end_percent=50",
      "attribution?model=linear&window_days=90&min_open_seconds=0&touches=opened",
      "attribution?model=linear&window_days=90&min_open_seconds=10",
      "attribution?model=linear&window_days=90&min_click_seconds=0&touches=clicked",
      "attribution?model=linear&window_days=90&min_click_seconds=10",
      "attribution?model=linear&window_days=90&rate_date=2026-01-01",
      "attribution?model=linear&window_days=90&rate_basis=close_day",
      "attribution?model=linear&window_days=90&convert_to=EUR&rate_basis=latest",
      "attribution?model=linear&window_days=90&convert_to=EUR&rate_basis=close_day&rate_date=2026-01-01",
    ])
      fails("GET", path);
  });

  it("rejects invalid lifetime-value and public-help requests", () => {
    for (const path of [
      "accounts/acc-5/clv?margin_bps=-1",
      "accounts/acc-5/clv?retention_bps=10001&years=2",
      "accounts/acc-5/clv?discount_bps=10001&retention_bps=8000&years=2",
      "accounts/acc-5/clv?retention_bps=8000",
      "accounts/acc-5/clv?retention_bps=8000&years=21",
      "accounts/acc-5/clv?convert_to=XXX",
      "accounts/acc-5/clv?convert_to=EUR&rate_basis=latest",
      "accounts/acc-5/clv?convert_to=EUR&rate_basis=close_day&rate_date=2026-01-01",
      "accounts/acc-5/clv?convert_to=EUR&rate_basis=close_day&margin_bps=5000",
    ])
      fails("GET", path);
    succeeds("GET", "kb/articles?q=no-result-anywhere");
    fails("GET", "kb/missing", null, 404);
    fails("GET", "kb/articles/missing", null, 404);
    fails("POST", "kb/articles/kb-1/feedback", { helpful: "yes" });
  });

  it("rejects remaining report and rate boundary cases", () => {
    const report = {
      name: "Duplicate report",
      spec: {
        kind: "deal",
        group_by: "currency",
        measure: "count",
        filter: {},
      },
      shared: false,
    };
    succeeds("POST", "reports", report);
    fails("POST", "reports", report);
    fails("POST", "reports/run", {
      kind: "unknown",
      group_by: "status",
      measure: "count",
      filter: {},
    });
    fails("POST", "reports/run", {
      kind: "deal",
      group_by: "currency",
      measure: "count",
      filter: {},
      convert_to: "EUR",
    });
    fails("POST", "reports/run", {
      kind: "deal",
      group_by: "currency",
      measure: "sum_amount",
      filter: {},
      convert_to: "XXX",
    });
    fails("POST", "reports/run", {
      kind: "deal",
      group_by: "currency",
      measure: "sum_amount",
      filter: {},
      convert_to: "EUR",
      rate_date: "2999-01-01",
    });
    fails("POST", "reports/run", {
      kind: "deal",
      group_by: "currency",
      measure: "sum_amount",
      filter: {},
      convert_to: "EUR",
      rate_basis: "close_day",
    });
    fails("PUT", "fx-rates", {
      from: "EUR",
      to: "GBP",
      rate: "1",
      effective_on: "2026-01-01",
      source: " ",
    });
    fails("POST", "contacts/con-1/tags", { tags: [] }, 405);
    fails("PUT", "contacts/con-1/attachments", null, 405);
    const numeric = succeeds("POST", "custom-fields", {
      entity: "contact",
      key: "coverage_number",
      kind: "number",
    });
    fails("PUT", "contacts/con-1/custom-fields", {
      values: { coverage_number: "not a number" },
    });
    succeeds("DELETE", `custom-fields/${String(numeric.pid)}`);
    succeeds("POST", "email-templates/preview", {
      subject: "Hello {{given_name}}",
      body: "{{unsubscribe_url}}",
      language: "not_a_tag",
      fallbacks: {},
      blank: ["given_name"],
    });
    succeeds("POST", "email-templates/preview", {
      subject: "Hello {{given_name}}",
      body: "{{unsubscribe_url}}",
      language: "ja",
      fallbacks: {},
      blank: ["given_name"],
    });
  });
});

describe("the typed API can drive the complete demo", () => {
  it("covers the relationship, board, communication, and insight surfaces", async () => {
    connectApiToDemo();
    await crm.listContacts(undefined, "priority");
    await crm.getContact("con-1");
    await crm.getContactDuplicates("con-1");
    await crm.recordConsent("con-1", "granted");
    await crm.editContact("con-1", { display_name: "API Contact" });
    await crm.listAccounts(undefined, "priority");
    await crm.getAccount("acc-1");
    await crm.editAccount("acc-1", { display_name: "API Account" });
    await crm.listLeads(undefined, "priority");
    await crm.getLead("lead-1");
    await crm.editLead("lead-1", { display_name: "API Lead" });
    await crm.leadStatus("lead-1", "contacted");
    await crm.listPipelines();
    await crm.listDeals("pipe-1");
    await crm.editDeal("deal-1", { amount_minor_text: "123456" });
    await crm.placeCard("deals", "deal-1", 1);
    await crm.moveDeal("deal-1", "stg-5");
    await crm.reopenDeal("deal-1", "API coverage");
    await crm.listSegments();
    await crm.listCampaigns();
    await crm.campaignFunnel("camp-1");
    await crm.listTickets();
    await crm.editTicket("tkt-1", { title: "API Ticket" });
    await crm.placeCard("tickets", "tkt-1", 1);
    await crm.ticketStatus("tkt-1", "pending");
    await crm.ticketCustomerReply("tkt-1");
    await crm.ticketSuggestions("tkt-1");
    await crm.assignTicket("tkt-1", "person:ada", "API coverage");
    expect(await crm.ticketMovements("tkt-1")).toMatchObject([
      { to_ref: "person:ada", via: "person", reason: "API coverage" },
    ]);
    await crm.listArticles("forecast");
    await crm.articleStatus("kb-1", "published");
    const kept = await crm.articleVersions("kb-2");
    expect(kept.map((v) => v.version)).toEqual([1]);
    await crm.salesDashboard();
    await crm.slaDashboard();
    await crm.staleDeals(14);
    await crm.followups("task");
    await crm.pipelineHygiene();
    await crm.executivePack();
    await crm.forecastTrends();
    await crm.slaRegister();
    const stuck = await crm.stuckTickets(3);
    expect(stuck.tickets.every((s) => s.idle_days >= 3)).toBe(true);
    const attained = await crm.slaAttainment();
    expect(attained.first_response.attained.denominator).toBe(
      attained.first_response.met + attained.first_response.missed,
    );
    await crm.dpo();
    await crm.cadence(30);
    await crm.engagementWorkload();
    await crm.funnel("pipe-1");
    await crm.membersHealth();
    await crm.consentByAccount();
    await crm.stakeholdersView();
    await crm.partnershipsRegister();
    await crm.membershipsView(90);
  });

  it("covers typed mutations and their optional query parameters", async () => {
    connectApiToDemo();
    await crm.mergeContact("con-2", "con-1", "API duplicate", true);
    await crm.mergeContact("con-2", "con-1", "API duplicate", false);
    await crm.getContactMerges("con-1");
    await crm.unmergeContact("con-2", "Restore API contact", true);
    await crm.unmergeContact("con-2", "Restore API contact", false);
    const link = await crm.addLink("contacts", "con-1", {
      system: "other",
      url: "https://example.test/api",
      label: " ",
    });
    await crm.getLinks("contacts", "con-1");
    await crm.removeLink(link.pid);
    await crm.eraseContact("con-9");
    await crm.editFollowup("fu-1", { summary: "API follow-up" });
    await crm.forecast({ convertTo: "EUR", rateDate: "2026-11-19" });
    await crm.createCampaign({
      name: "API campaign",
      currency: "EUR",
      subject: "Hello",
      body: "{{unsubscribe_url}}",
    } as never);
    await crm.campaignFunnel("camp-1", {
      convert: true,
      rateDate: "2026-11-19",
    });
    await crm.campaignFunnel("camp-1", {
      convert: true,
      rateBasis: "close_day",
    });
    await crm.runCampaign("camp-3");
    await crm.campaignStatus("camp-1", "scheduled");

    const csv = "Name,Email\nTyped Person,typed@example.test";
    const imported = await crm.commitLeadImport({
      csv,
      mapping: { display_name: "Name", email: "Email" },
      source_note: "Synthetic typed API test",
      obtained_on: "2026-11-19",
      skip_likely_duplicates: true,
    });
    await crm.undoLeadImport(imported.pid);
    const records = await crm.commitRecordImport("contacts", {
      csv,
      mapping: { display_name: "Name", email: "Email" },
      source_note: "Synthetic typed API test",
      obtained_on: "2026-11-19",
      skip_likely_duplicates: true,
    });
    await crm.undoRecordImport("contacts", records.pid);
    await crm.listAttachments("contacts", "con-1");
    const attachment = succeeds(
      "POST",
      "contacts/con-1/attachments?name=api.txt&size=3",
    );
    await crm.deleteAttachment(String(attachment.pid));
    expect(crm.attachmentUrl("att-1")).toBe(
      "/api/proxy/attachments/att-1/content",
    );
  });

  it("covers user-defined resources and analytical configuration", async () => {
    connectApiToDemo();
    await crm.listViews("deal");
    const view = await crm.saveView({
      kind: "deal",
      name: "API view",
      filter: {},
      sort: null,
      columns: null,
      shared: false,
    });
    await crm.replaceView(view.pid, {
      kind: "deal",
      name: "Changed API view",
      filter: {},
      sort: null,
      columns: ["name"],
      shared: false,
    });
    await crm.deleteView(view.pid);

    await crm.listTemplates();
    await crm.mergeTokens("en");
    await crm.previewTemplate(
      {
        subject: "Hello {{given_name}}",
        body: "{{unsubscribe_url}}",
        fallbacks: {},
        language: "en",
      },
      ["given_name"],
    );
    const template = await crm.saveTemplate({
      name: "API template",
      subject: "Hello",
      body: "{{unsubscribe_url}}",
      fallbacks: {},
      language: "en",
    });
    await crm.replaceTemplate(template.pid, {
      name: "Changed API template",
      subject: "Hello",
      body: "{{unsubscribe_url}}",
      fallbacks: {},
      language: "en",
    });
    await crm.deleteTemplate(template.pid);

    await crm.listWorkflows();
    const workflow = await crm.saveWorkflow({
      name: "API workflow",
      enabled: true,
      trigger: "contact_created",
      every_minutes: null,
      conditions: {
        statuses: [],
        channels: [],
        account_tiers: [],
        sources: [],
      },
      actions: [{ type: "create_task", summary: "Call" }],
    } as never);
    await crm.dryRunWorkflow(workflow.pid);
    await crm.workflowRuns(workflow.pid);
    await crm.replaceWorkflow(workflow.pid, {
      name: "Changed API workflow",
      enabled: true,
      trigger: "contact_created",
      every_minutes: null,
      conditions: {
        statuses: [],
        channels: [],
        account_tiers: [],
        sources: [],
      },
      actions: [{ type: "create_task", summary: "Write" }],
    } as never);
    await crm.deleteWorkflow(workflow.pid);

    await crm.listCustomFields();
    const field = await crm.createCustomField({
      entity: "contact",
      key: "api_score",
      label: "API score",
      kind: "number",
      choices: [],
      sensitivity: "business",
    });
    await crm.setCustomFieldValues("contacts", "con-1", { api_score: 95 });
    await crm.getCustomFieldValues("contacts", "con-1");
    await crm.deleteCustomField(field.pid);
    await crm.listTags();
    await crm.setRecordTags("contacts", "con-1", ["api"]);
    await crm.getRecordTags("contacts", "con-1");
  });

  it("covers reports, rates, public help, imports, quotas, and bridges", async () => {
    connectApiToDemo();
    const spec = {
      kind: "deal",
      group_by: "currency",
      measure: "sum_amount",
      filter: {},
    } as const;
    await crm.runReport(spec);
    await crm.listReports();
    const report = await crm.saveReport({
      name: "API report",
      spec,
      shared: false,
    });
    await crm.runSavedReport(report.pid);
    await crm.deleteReport(report.pid);
    await crm.attribution({ model: "linear", windowDays: 90 });
    await crm.attribution({
      model: "time_decay",
      windowDays: 90,
      halfLifeDays: 14,
      touches: ["sent", "opened", "clicked"],
      minOpenSeconds: 5,
      minClickSeconds: 5,
    });
    await crm.listFxRates();
    const rate = await crm.setFxRate({
      from: "EUR",
      to: "GBP",
      rate: "0.875",
      effective_on: "2026-11-19",
      source: "Synthetic test",
    });
    await crm.deleteFxRate(rate.pid);
    await crm.kbSearch("forecast");
    await crm.kbArticle("kb-1");
    await crm.kbFeedback("kb-1", true);
    await crm.kbDeflection(30);

    const csv = "Name,Email\nAPI Person,api@example.test";
    await crm.previewLeadImport(csv);
    await crm.previewRecordImport("contacts", csv);
    await crm.previewRecordImport("accounts", "Name,Tier\nAPI Ltd,gold");
    await crm.searchRecords("Acme");
    await crm.notifications();
    await crm.unreadNotifications();
    await crm.markNotificationRead("note-1");
    await crm.markAllNotificationsRead();
    await crm.organizationSettings();
    await crm.setOrganizationSettings({
      time_zone: "Asia/Kolkata",
      fiscal_year_start_month: 4,
      stalled_after_days: 21,
    });
    await crm.listCommissionPlans();
    await crm.listQuotas();
    await crm.listTeams();
    await crm.listTerritories();
    await crm.quotaAttainment();
    await crm.teamAttainment();
    await crm.territoryAttainment();
    await crm.importTeams();
    await crm.checkEmployment();
  });

  it("calculates compensation, ownership, delivery, and lifetime value", async () => {
    connectApiToDemo();
    const today = new Date().toISOString().slice(0, 10);
    const from = new Date(Date.now() - 100 * 86_400_000)
      .toISOString()
      .slice(0, 10);
    const to = new Date(Date.now() + 40 * 86_400_000)
      .toISOString()
      .slice(0, 10);

    const plan = await crm.setCommissionPlan({
      name: "Coverage accelerator",
      tiers: [
        { from_percent: 0, rate_bps: 500 },
        { from_percent: 100, rate_bps: 900 },
      ],
    });
    await crm.setCommissionPlan({
      name: "coverage accelerator",
      tiers: [{ from_percent: 0, rate_bps: 600 }],
    });
    const quota = await crm.setQuota({
      owner_ref: "worker:ayo",
      period_from: from,
      period_to: to,
      currency: "EUR",
      amount_minor: 2_000_000,
      plan_pid: plan.pid,
    });
    await crm.setQuota({
      owner_ref: "worker:ayo",
      period_from: from,
      period_to: to,
      currency: "EUR",
      amount_minor: 2_500_000,
      plan_pid: plan.pid,
    });

    await crm.setDealSplit("deal-9", [
      { owner_ref: "owner-bea", share_bps: 7000 },
      { owner_ref: "worker:ayo", share_bps: 3000 },
    ]);
    await crm.setClawback("deal-9", {
      owner_ref: "owner-bea",
      amount_minor: 100_000,
      reason: "Synthetic adjustment",
    });
    await crm.quotaAttainment(today);
    await crm.deleteClawback("deal-9", "owner-bea");
    await crm.deleteDealSplit("deal-9");
    await crm.openDelivery("deal-9");

    const team = await crm.setTeam({
      name: "Coverage team",
      members: ["worker:ayo", "owner-bea"],
    });
    await crm.setTeam({ name: "coverage team", members: ["owner-bea"] });
    await crm.teamAttainment(today);
    await crm.deleteTeam(team.pid);

    const territory = await crm.setTerritory({
      name: "Coverage territory",
      owner_ref: "worker:ayo",
      accounts: ["org-demo-5"],
    });
    await crm.setTerritory({
      name: "coverage territory",
      owner_ref: null,
      accounts: ["org-demo-5", "org-demo-6"],
    });
    await crm.territoryAttainment({ from, to });
    await crm.deleteTerritory(territory.pid);

    await crm.accountClv("acc-5", {
      marginBps: 6500,
      retentionBps: 8500,
      discountBps: 500,
      years: 3,
      convertTo: "EUR",
    });
    await crm.accountClv("acc-5", {
      retentionFromHistory: true,
      years: 2,
    });
    await crm.accountClv("acc-5", {
      convertTo: "EUR",
      rateBasis: "close_day",
    });

    await crm.deleteQuota(quota.pid);
    await crm.deleteCommissionPlan(plan.pid);
  });

  it("runs every attribution and report calculation path", async () => {
    connectApiToDemo();
    const today = new Date().toISOString().slice(0, 10);
    for (const model of ["first", "last", "linear"] as const)
      await crm.attribution({ model, windowDays: 90 });
    await crm.attribution({
      model: "position",
      windowDays: 90,
      endPercent: 35,
      convertTo: "EUR",
      rateDate: today,
    });
    await crm.attribution({
      model: "time_decay",
      windowDays: 90,
      halfLifeDays: 10,
      convertTo: "EUR",
      rateBasis: "close_day",
    });

    const reports = [
      { kind: "contact", group_by: "marketing_consent", measure: "count" },
      { kind: "account", group_by: "tier", measure: "count" },
      { kind: "lead", group_by: "source", measure: "count" },
      { kind: "ticket", group_by: "priority", measure: "count" },
      { kind: "deal", group_by: "tag", measure: "count" },
      {
        kind: "deal",
        group_by: "pipeline",
        measure: "sum_amount",
        convert_to: "EUR",
        rate_date: today,
      },
      {
        kind: "deal",
        group_by: "closed_fiscal_quarter",
        measure: "sum_amount",
        filter: { state: ["won"] },
        convert_to: "EUR",
        rate_basis: "close_day",
      },
    ];
    for (const spec of reports)
      await crm.runReport({ filter: {}, ...spec } as never);
  });
});
