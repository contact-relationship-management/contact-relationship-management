// Unit tests: the money formatter's masked/absent honesty, the API
// path map (contract mirror), and the seven-locale i18n parity pin.

import { describe, expect, it, vi } from "vitest";

import { money } from "../../src/lib/money";
import {
  DEFAULT_LOCALE,
  LOCALES,
  LOCALE_LABELS,
  STRING_KEYS,
  loadAllLocales,
  loadLocale,
  i18n,
  isRtl,
  translate,
} from "../../src/lib/i18n.svelte";

describe("money", () => {
  it("formats minor units as locale currency", () => {
    expect(money(500000, "GBP", "en")).toBe("£5,000.00");
  });

  it("renders the masked/absent state as an em dash, never zero", () => {
    expect(money(null, "GBP", "en")).toBe("—");
    expect(money(undefined, "GBP", "en")).toBe("—");
    expect(money(500000, null, "en")).toBe("—");
  });
});

describe("i18n", () => {
  it("covers every key in every locale (parity)", async () => {
    const tables = await loadAllLocales();
    for (const locale of LOCALES) {
      const table = tables[locale];
      for (const key of STRING_KEYS) {
        expect(table[key], `${locale} missing ${key}`).toBeTruthy();
      }
      expect(Object.keys(table).sort()).toEqual([...STRING_KEYS].sort());
    }
  });

  it("supports exactly the seven family locales, sorted, no parentheses", () => {
    expect([...LOCALES]).toEqual([
      "ar-001",
      "cy-001",
      "de-de",
      "en-001",
      "es-001",
      "fr-001",
      "hi-001",
      "zh-cn",
    ]);
    expect(DEFAULT_LOCALE).toBe("en-001");
    for (const label of Object.values(LOCALE_LABELS)) {
      expect(label).not.toMatch(/[()]/);
    }
    expect(LOCALE_LABELS["zh-cn"]).toBe("中文 - 中国");
  });

  it("translates with en fallback and flags RTL locales", async () => {
    await loadLocale("es-001");
    expect(translate("nav.contacts", "es-001")).toBe("Contactos");
    expect(translate("nav.contacts", DEFAULT_LOCALE)).toBe("Contacts");
    expect(translate("nav.contacts", "xx" as never)).toBe("Contacts");
    expect(translate("share.copy_link", "en-001")).toBe("Copy Link");
    expect(isRtl("ar-001")).toBe(true);
    expect(isRtl("ar")).toBe(true);
    expect(isRtl("en-001")).toBe(false);
    expect(isRtl("zh-cn")).toBe(false);
  });

  it("answers in English until a locale has loaded, then in that locale", async () => {
    // A fresh module registry: nothing but English is loaded yet.
    vi.resetModules();
    const fresh = await import("../../src/lib/i18n.svelte");
    expect(fresh.translate("nav.leads", "fr-001")).toBe("Leads");
    const pending = fresh.loadLocale("fr-001");
    expect(fresh.loadLocale("fr-001")).toBe(pending); // one fetch, however often asked
    await pending;
    expect(fresh.translate("nav.leads", "fr-001")).not.toBe("Leads");
  });

  it("reads every locale directory of the role guides as one of the UI locales", async () => {
    // The root `locales/` tree holds role guides, in more regional variants
    // than the UI has strings for; each must still resolve to a UI locale.
    const { readdirSync } = await import("node:fs");
    const { resolve } = await import("node:path");
    const root = resolve(process.cwd(), "../locales"); // vitest runs in the UI folder
    const dirs = readdirSync(root, { withFileTypes: true })
      .filter((entry) => entry.isDirectory())
      .map((entry) => entry.name);
    expect(dirs.length).toBeGreaterThan(0);
    for (const dir of dirs) {
      i18n.set(dir);
      expect((LOCALES as readonly string[]).includes(i18n.locale), dir).toBe(
        true,
      );
      // An unknown code falls back to English; a guide locale must not.
      if (!dir.startsWith("en")) expect(i18n.locale, dir).not.toBe("en-001");
    }
  });

  it("normalises legacy and regional codes by primary language", () => {
    i18n.set("en_US");
    expect(i18n.locale).toBe("en-001");
    i18n.set("es-MX");
    expect(i18n.locale).toBe("es-001");
    i18n.set("zh");
    expect(i18n.locale).toBe("zh-cn");
    i18n.set("zh-CN");
    expect(i18n.locale).toBe("zh-cn");
    i18n.set("de-DE");
    expect(i18n.locale).toBe("de-de");
    i18n.set("ja");
    expect(i18n.locale).toBe("en-001");
  });
});

describe("api path map", () => {
  it("calls the exact proxy paths the service mounts", async () => {
    const calls: string[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string | URL) => {
        calls.push(String(url));
        return new Response("[]", {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }),
    );
    const crm = await import("../../src/lib/api/crm");
    await crm.listContacts();
    await crm.getContact("c1");
    await crm.listLeads();
    await crm.listDeals("p1");
    await crm.forecast();
    await crm.campaignFunnel("k1");
    await crm.listTickets();
    await crm.salesDashboard();
    await crm.leadStatus("l1", "contacted");
    await crm.staleDeals(7);
    await crm.followups();
    await crm.pipelineHygiene();
    await crm.executivePack();
    await crm.forecastTrends();
    await crm.slaRegister();
    await crm.dpo();
    await crm.followups("task");
    await crm.cadence(30);
    await crm.engagementWorkload();
    await crm.funnel("p1");
    await crm.membersHealth();
    await crm.consentByAccount();
    await crm.stakeholdersView();
    await crm.partnershipsRegister();
    await crm.membershipsView(90);
    expect(calls).toEqual([
      "/api/proxy/contacts",
      "/api/proxy/contacts/c1",
      "/api/proxy/leads",
      "/api/proxy/deals?pipeline=p1",
      "/api/proxy/forecast",
      "/api/proxy/campaigns/k1/funnel",
      "/api/proxy/tickets",
      "/api/proxy/dashboards/sales",
      "/api/proxy/leads/l1/status",
      "/api/proxy/insights/stale-deals?days=7",
      "/api/proxy/insights/followups",
      "/api/proxy/insights/pipeline-hygiene",
      "/api/proxy/insights/executive",
      "/api/proxy/insights/forecast-trends",
      "/api/proxy/insights/sla",
      "/api/proxy/insights/dpo",
      "/api/proxy/insights/followups?kind=task",
      "/api/proxy/insights/cadence?days=30",
      "/api/proxy/insights/engagement",
      "/api/proxy/insights/funnel?pipeline=p1",
      "/api/proxy/insights/members",
      "/api/proxy/insights/consent-by-account",
      "/api/proxy/insights/stakeholders",
      "/api/proxy/insights/partnerships",
      "/api/proxy/insights/memberships?days=90",
    ]);
    vi.unstubAllGlobals();
  });
});

describe("tour", () => {
  const TOUR_KEYS = [
    "nav.tour",
    "tour.head",
    "tour.intro",
    ...[1, 2, 3, 4, 5, 6].flatMap((n) => [
      `tour.s${n}.title`,
      `tour.s${n}.summary`,
      ...[1, 2, 3, 4].map((k) => `tour.s${n}.step.${k}`),
    ]),
  ];

  it("has six workflow sections of four steps in every locale", async () => {
    const tables = await loadAllLocales();
    for (const locale of LOCALES) {
      for (const key of TOUR_KEYS) {
        expect(tables[locale][key], `${locale} missing ${key}`).toBeTruthy();
      }
    }
  });

  it("is public: the root layout gate exempts /tour", async () => {
    const { load } = await import("../../src/routes/+layout.server");
    const run = (path: string) =>
      load({
        locals: { sessionId: null },
        url: new URL(`http://localhost${path}`),
      } as unknown as Parameters<typeof load>[0]);
    expect(run("/tour")).toEqual({ signedIn: false, demo: false });
    expect(() => run("/contacts")).toThrow();
  });
});

import { fieldErrors } from "../../src/lib/errors";

describe("fieldErrors", () => {
  it("maps each pointer to the member it names", () => {
    const cause = {
      body: {
        errors: [
          { pointer: "/display_name", detail: "display_name is required" },
          { pointer: "/stages/2/name", detail: "too long" },
          { detail: "about the whole request" },
        ],
      },
    };
    // The members are named; the service's own words are not carried (see errors.test.ts).
    expect(fieldErrors(cause)).toEqual({
      display_name: "Not accepted.",
      "stages/2/name": "Not accepted.",
    });
  });

  it("keeps the first problem for a member, and gives {} for anything else", () => {
    expect(
      fieldErrors({
        body: {
          errors: [
            { pointer: "/a", detail: "first" },
            { pointer: "/a", detail: "second" },
          ],
        },
      }),
    ).toEqual({ a: "Not accepted." });
    expect(fieldErrors(new Error("x"))).toEqual({});
    expect(fieldErrors(null)).toEqual({});
    expect(fieldErrors({ body: { errors: "no" } })).toEqual({});
  });
});
