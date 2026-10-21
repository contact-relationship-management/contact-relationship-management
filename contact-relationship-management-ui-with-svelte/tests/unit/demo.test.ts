import { readFileSync } from "node:fs";
import { beforeEach, describe, expect, it } from "vitest";
import {
  DEMO_EMAIL,
  MAX_DEMO_SESSIONS,
  demoHandle,
  demoHandleFor,
  demoSessionCount,
  endDemoSession,
  isDemoEmail,
  isDemoSession,
  resetDemo,
  startDemoSession,
} from "../../src/lib/server/demo";

const get = (path: string) => {
  const [p, q] = path.split("?");
  return demoHandle("GET", p!, new URLSearchParams(q ?? ""), null);
};
const post = (path: string, body: unknown = null) =>
  demoHandle("POST", path, new URLSearchParams(), body);

beforeEach(() => resetDemo());

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
