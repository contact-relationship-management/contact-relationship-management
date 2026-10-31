import { readFileSync } from "node:fs";
import { beforeEach, describe, expect, it } from "vitest";
import {
  COLUMN_NAMES,
  DEMO_EMAIL,
  TEMPLATE_FALLBACKS,
  fiscalLabel,
  MAX_DEMO_SESSIONS,
  demoHandle,
  demoHandleFor,
  demoSessionCount,
  endDemoSession,
  isDemoEmail,
  isDemoSession,
  resetDemo,
  startDemoSession,
  userRef,
} from "../../src/lib/server/demo";

const get = (path: string) => {
  const [p, q] = path.split("?");
  return demoHandle("GET", p!, new URLSearchParams(q ?? ""), null);
};
const post = (path: string, body: unknown = null) =>
  demoHandle("POST", path, new URLSearchParams(), body);

beforeEach(() => resetDemo());

describe("a CRM user is a person: reference (CRM-D18)", () => {
  const id = "3f2c6b7e-0000-4000-8000-000000000001";
  it("reads a legacy worker: reference as person: with the same id", () => {
    expect(userRef(`worker:${id}`)).toBe(`person:${id}`);
    expect(userRef(` worker:${id} `)).toBe(`person:${id}`);
    expect(userRef(`person:${id}`)).toBe(`person:${id}`);
    expect(userRef("organization:x")).toBe("organization:x");
  });
  it("stores a team's legacy members as person:, as the service does", () => {
    const put = demoHandle("PUT", "quotas/teams", new URLSearchParams(), {
      name: "North",
      members: [`worker:${id}`],
    });
    expect(put.status).toBe(200);
    expect((put.body as { members: string[] }).members).toEqual([
      `person:${id}`,
    ]);
  });
});

describe("the guided tour's identity", () => {
  it("accepts only example@example.com, ignoring case and spacing", () => {
    expect(isDemoEmail(DEMO_EMAIL)).toBe(true);
    expect(isDemoEmail("  Example@Example.COM ")).toBe(true);
    expect(isDemoEmail("someone@example.com")).toBe(false);
    expect(isDemoEmail("example@example.com.evil.test")).toBe(false);
  });

  it("recognises only its own session ids", () => {
    expect(isDemoSession(startDemoSession())).toBe(true);
    expect(isDemoSession("a-real-session")).toBe(false);
    expect(isDemoSession(null)).toBe(false);
  });

  it("gives every sign-in a different session id", () => {
    expect(startDemoSession()).not.toBe(startDemoSession());
  });
});

describe("each visitor's own copy of the tour", () => {
  const consent = (sid: string, action: string) =>
    demoHandleFor(
      sid,
      "POST",
      "contacts/con-1/consent",
      new URLSearchParams(),
      {
        action,
      },
    );
  const consentOf = (sid: string) =>
    (
      demoHandleFor(sid, "GET", "contacts/con-1", new URLSearchParams(), null)
        .body as { contact: { marketing_consent: string } }
    ).contact.marketing_consent;

  it("keeps one visitor's writes out of another's", () => {
    const a = startDemoSession();
    const b = startDemoSession();
    const before = consentOf(b);
    consent(a, "withdrawn");
    expect(consentOf(a)).toBe("withdrawn");
    expect(consentOf(b)).toBe(before);
    // ...and out of the default copy that `demoHandle` uses.
    expect(
      (get("contacts/con-1").body as { contact: { marketing_consent: string } })
        .contact.marketing_consent,
    ).toBe(before);
  });

  it("keeps at most MAX_DEMO_SESSIONS copies, dropping the least recently used", () => {
    const first = startDemoSession();
    consent(first, "withdrawn");
    for (let i = 0; i < MAX_DEMO_SESSIONS; i += 1) startDemoSession();
    expect(demoSessionCount()).toBe(MAX_DEMO_SESSIONS);
    // The first copy went, so that visitor starts again from the original data.
    expect(consentOf(first)).not.toBe("withdrawn");
  });

  it("forgets a copy at sign-out", () => {
    const sid = startDemoSession();
    consent(sid, "withdrawn");
    endDemoSession(sid);
    expect(consentOf(sid)).not.toBe("withdrawn");
  });
});

const SAMPLE_ID: Record<string, string> = {
  contacts: "con-1",
  leads: "lead-1",
  campaigns: "camp-1",
  workflows: "wf-1",
  reports: "rep-1",
  deals: "deal-1",
  tickets: "tkt-1",
  articles: "kb-1",
  accounts: "acc-1",
};

describe("every read the UI makes has sample data", () => {
  // Scan the typed API surface so a new endpoint cannot ship without tour data.
  const source = readFileSync("src/lib/api/crm.ts", "utf8");
  const reads = [
    ...source.matchAll(/api(?:All)?\(\s*[`"]([^`"]+)[`"]([^)]{0,80})/g),
  ]
    .filter((m) => !/method:/.test(m[2] ?? ""))
    // A `${…}` straight after a path segment starts a query string; one after
    // a slash is an id.
    .map((m) =>
      m[1]!
        .replace(/([^/])\$\{.*$/, "$1")
        .replace(/\$\{[^}]*\}/g, "x")
        .replace(/\?.*$/, ""),
    )
    .map((p) => p.replace(/^\//, ""))
    // The record kind is a parameter (links work on four kinds): try contacts.
    .map((p) => p.replace(/^x\/x\//, "contacts/x/"))
    // The public help centre names an article one segment further in.
    .map((p) => p.replace(/^kb\/articles\/x/, "kb/articles/kb-1"))
    .map((p) =>
      p.replace(
        /^(\w+)\/x/,
        (_, head: string) => `${head}/${SAMPLE_ID[head] ?? "x"}`,
      ),
    );

  it("finds the reads it expects", () => {
    expect(reads.length).toBeGreaterThan(25);
  });

  it.each([...new Set(reads)])("GET /%s answers 200", (path) => {
    const reply = get(path);
    expect(reply.status, JSON.stringify(reply.body)).toBe(200);
  });
});

describe("the sample data hangs together", () => {
  it("the forecast counts the open deals and weights by stage", () => {
    const f = get("forecast").body as {
      open_deals: number;
      totals_minor: { GBP: number };
    };
    expect(f.open_deals).toBe(8);
    expect(f.totals_minor.GBP).toBeGreaterThan(0);
  });

  it("moving a deal to Won updates the forecast and the win rate", () => {
    const before = get("dashboards/sales").body as {
      win_rate: { numerator: number };
    };
    const moved = post("deals/deal-1/stage", { stage_pid: "stg-5" });
    expect(moved.status).toBe(200);
    expect((moved.body as { won: boolean }).won).toBe(true);
    const after = get("dashboards/sales").body as {
      win_rate: { numerator: number };
    };
    expect(after.win_rate.numerator).toBe(before.win_rate.numerator + 1);
    expect((get("forecast").body as { open_deals: number }).open_deals).toBe(7);
  });

  it("refuses to erase a contact with an open deal, and erases one without", () => {
    const refused = post("contacts/con-1/erase");
    expect(refused.status).toBe(422);
    const erased = post("contacts/con-4/erase");
    expect(erased.status).toBe(200);
    const row = get("contacts/con-4").body as {
      contact: { display_name: string };
    };
    expect(row.contact.display_name).toBe("Erased contact");
  });

  it("records consent and resolves a ticket", () => {
    post("contacts/con-9/consent", { action: "granted" });
    expect(
      (get("contacts/con-9").body as { contact: { marketing_consent: string } })
        .contact.marketing_consent,
    ).toBe("granted");
    post("tickets/tkt-1/status", { to: "resolved" });
    const sla = get("dashboards/sla").body as { open_tickets: number };
    expect(sla.open_tickets).toBe(5);
  });

  it("searches articles and runs a scheduled campaign", () => {
    const hits = get("articles?q=forecast").body as unknown[];
    expect(hits).toHaveLength(1);
    const run = post("campaigns/camp-3/run").body as {
      status: string;
      delivered: number;
    };
    expect(run.status).toBe("completed");
    expect(run.delivered).toBeGreaterThan(0);
    expect(post("campaigns/camp-4/run").status).toBe(422);
  });

  it("break-glass needs a real reason, and a grantee cannot review their own", () => {
    expect(post("break-glass", { reason: "short", minutes: 15 }).status).toBe(
      422,
    );
    const made = post("break-glass", {
      reason: "Customer is on the phone and I need to see unmasked values.",
      minutes: 15,
    });
    expect(made.status).toBe(200);
    const pid = (made.body as { pid: string }).pid;
    expect(post(`break-glass/${pid}/review`, { note: "mine" }).status).toBe(
      422,
    );
    expect(
      post("break-glass/bg-2/review", {
        note: "Checked with the account owner.",
      }).status,
    ).toBe(200);
    expect(post("break-glass/bg-2/review", { note: "again" }).status).toBe(422);
  });

  it("answers an unknown path with a 404 problem, not a crash", () => {
    expect(get("nope").status).toBe(404);
    expect(get("contacts/missing").status).toBe(404);
  });

  it("resetDemo restores the original data", () => {
    post("deals/deal-1/stage", { stage_pid: "stg-5" });
    resetDemo();
    expect((get("forecast").body as { open_deals: number }).open_deals).toBe(8);
  });
});

describe("the tour's notifications inbox (CRM-R32)", () => {
  it("lists two unread, marks one read, then the rest", () => {
    expect((get("notifications").body as unknown[]).length).toBe(2);
    expect(get("notifications/unread-count").body).toEqual({
      unread: 2,
      capped: false,
    });
    const read = demoHandle(
      "PUT",
      "notifications/note-1/read",
      new URLSearchParams(),
      null,
    );
    expect(read.status).toBe(200);
    expect((get("notifications?unread=true").body as unknown[]).length).toBe(1);
    expect(post("notifications/read-all").body).toEqual({ marked: 1 });
    expect(
      demoHandle("PUT", "notifications/nope/read", new URLSearchParams(), null)
        .status,
    ).toBe(404);
  });
});

describe("the tour's data is in more than one shape (CRM-T176)", () => {
  const exponent = (code: string) =>
    new Intl.NumberFormat("en", {
      style: "currency",
      currency: code,
    }).resolvedOptions().maximumFractionDigits;

  it("has deals in a zero-exponent and a three-exponent currency, totalled apart", () => {
    const deals = get("deals").body as Array<{ currency: string }>;
    const codes = new Set(deals.map((d) => d.currency));
    expect([...codes].map(exponent)).toEqual(expect.arrayContaining([0, 2, 3]));
    const forecast = get("forecast").body as {
      totals_minor: Record<string, number>;
    };
    expect(Object.keys(forecast.totals_minor).sort()).toEqual([
      "GBP",
      "JPY",
      "KWD",
    ]);
  });

  it("has rates from more than one base", () => {
    const rates = get("fx-rates").body as Array<{ from: string }>;
    expect(new Set(rates.map((r) => r.from)).size).toBeGreaterThanOrEqual(3);
  });

  it("has names in more than one script and order, parts never derived", () => {
    const contacts = get("contacts").body as Array<{
      display_name: string;
      given_name: string | null;
      family_name: string | null;
    }>;
    const scripts = ["Han", "Arabic", "Devanagari", "Latin"].filter((s) =>
      contacts.some((c) =>
        new RegExp(`\\p{Script=${s}}`, "u").test(c.display_name),
      ),
    );
    expect(scripts).toHaveLength(4);
    // Family name first, as stored.
    expect(
      contacts.some(
        (c) => c.family_name && c.display_name.startsWith(c.family_name),
      ),
    ).toBe(true);
    // A name with no parts stored keeps them empty.
    expect(contacts.some((c) => c.given_name === null)).toBe(true);
  });

  it("names amounts in minor units, never pounds times a unit", () => {
    const source = readFileSync("src/lib/server/demo.ts", "utf8");
    expect(source).not.toMatch(/\bpounds\b/);
  });
});

describe("template fallbacks and import columns in the UI's languages (CRM-T169)", () => {
  const service = (path: string) =>
    readFileSync(
      `../contact-relationship-management-service-with-rust/src/rules/${path}`,
      "utf8",
    );

  it("has the service's built-in fallbacks, language by language", () => {
    const source = service("templates.rs");
    const table = source.slice(
      source.indexOf("pub const FALLBACKS"),
      source.indexOf("];", source.indexOf("pub const FALLBACKS")),
    );
    const rows = [...table.matchAll(/"([a-z]{2})",\s*\[([^\]]*)\]/g)].map(
      ([, code, list]) => [
        code!,
        [...list!.matchAll(/"([^"]*)"/g)].map((m) => m[1]!),
      ],
    );
    expect(Object.fromEntries(rows)).toEqual(TEMPLATE_FALLBACKS);
  });

  it("gives the tokens' fallbacks in the language asked for, and none where it has none", () => {
    const de = get("email-templates/tokens?language=de-AT").body as Array<{
      name: string;
      fallback: string | null;
    }>;
    expect(de.find((x) => x.name === "given_name")?.fallback).toBe("zusammen");
    const ja = get("email-templates/tokens?language=ja").body as Array<{
      fallback: string | null;
    }>;
    expect(ja.every((x) => x.fallback === null)).toBe(true);
  });

  it("refuses a template in a language with no built-in fallbacks until it has its own", () => {
    const draft = {
      name: "Japanese",
      language: "ja",
      subject: "{{given_name}}様",
      body: "{{unsubscribe_url}}",
    };
    expect(post("email-templates", draft).status).toBe(422);
    expect(
      post("email-templates", { ...draft, fallbacks: { given_name: "お客" } })
        .status,
    ).toBe(200);
  });

  it("guesses import columns named in other languages, as the service does", () => {
    const preview = post("lead-imports/preview", {
      csv: "Vollständiger Name,E-Mail-Adresse,Firma\nAnna Schmidt,anna@example.org,Muster",
    }).body as { suggested_mapping: Record<string, string | null> };
    expect(preview.suggested_mapping).toEqual({
      display_name: "Vollständiger Name",
      email: "E-Mail-Adresse",
      company: "Firma",
    });
    const source = service("column_names.rs");
    for (const name of Object.values(COLUMN_NAMES).flat())
      expect(source, name).toContain(`"${name}"`);
  });
});

describe("the organization's calendar (CRM-T171)", () => {
  it("labels fiscal years and quarters from any start month, as the service does", () => {
    expect(fiscalLabel("2026-03", true, 1)).toBe("2026-Q1");
    expect(fiscalLabel("2026-03", true, 4)).toBe("2025/26-Q4");
    expect(fiscalLabel("2026-04", false, 4)).toBe("2026/27");
    expect(fiscalLabel("2026-12", true, 10)).toBe("2026/27-Q1");
    expect(fiscalLabel("2026-06", true, 7)).toBe("2025/26-Q4");
  });

  it("is set and checked, and the reports follow it", () => {
    expect(
      demoHandle("PUT", "settings/organization", new URLSearchParams(), {
        time_zone: "Tokyo",
        fiscal_year_start_month: 4,
      }).status,
    ).toBe(422);
    const set = demoHandle(
      "PUT",
      "settings/organization",
      new URLSearchParams(),
      {
        time_zone: "Asia/Kolkata",
        fiscal_year_start_month: 4,
      },
    );
    expect(set.status).toBe(200);
    const run = post("reports/run", {
      kind: "deal",
      group_by: "created_fiscal_quarter",
      measure: "count",
      filter: {},
    }).body as { time_zone: string; rows: Array<{ group: string }> };
    expect(run.time_zone).toBe("Asia/Kolkata");
    expect(run.rows.map((r) => r.group)).toContain("2026/27-Q3");
  });
});

describe("the stalled deal flag, as the service derives it (CRM-R4, CRM-D20)", () => {
  type Listed = {
    pid: string;
    stalled: boolean;
    last_touched_at: string;
    closed_at: string | null;
  };
  const deals = () => get("deals").body as Listed[];
  const put = (body: unknown) =>
    demoHandle("PUT", "settings/organization", new URLSearchParams(), body);

  it("flags the open deals untouched for the organization's days, and no closed one", () => {
    const stalled = deals()
      .filter((d) => d.stalled)
      .map((d) => d.pid);
    expect(stalled.sort()).toEqual(["deal-4", "deal-6"]);
    expect(deals().some((d) => d.stalled && d.closed_at !== null)).toBe(false);
  });

  it("clears the flag on a stage move, which is a touch", () => {
    const before = Date.now();
    expect(post("deals/deal-6/stage", { stage_pid: "stg-2" }).status).toBe(200);
    const moved = deals().find((d) => d.pid === "deal-6")!;
    expect(moved.stalled).toBe(false);
    expect(Date.parse(moved.last_touched_at)).toBeGreaterThanOrEqual(
      before - 1000,
    );
    expect(deals().find((d) => d.pid === "deal-4")?.stalled).toBe(true);
  });

  it("takes N from the organization's settings, checked from 1 to 365, and keeps it when absent", () => {
    expect(
      put({
        time_zone: "UTC",
        fiscal_year_start_month: 1,
        stalled_after_days: 0,
      }).status,
    ).toBe(422);
    expect(
      put({
        time_zone: "UTC",
        fiscal_year_start_month: 1,
        stalled_after_days: 366,
      }).status,
    ).toBe(422);
    expect(
      put({
        time_zone: "UTC",
        fiscal_year_start_month: 1,
        stalled_after_days: 60,
      }).status,
    ).toBe(200);
    expect(deals().filter((d) => d.stalled)).toEqual([]);
    const kept = put({ time_zone: "UTC", fiscal_year_start_month: 1 });
    expect(
      (kept.body as { stalled_after_days: number }).stalled_after_days,
    ).toBe(60);
    expect(
      put({
        time_zone: "UTC",
        fiscal_year_start_month: 1,
        stalled_after_days: 1,
      }).status,
    ).toBe(200);
    expect(
      deals().filter((d) => d.stalled && d.closed_at === null).length,
    ).toBeGreaterThan(2);
  });
});
