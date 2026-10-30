// Pins the BFF proxy's upstream URL: the default `CRM_API_URL` reaches the
// service's `/api` routes, and a trailing slash does not double up.

import { describe, expect, it } from "vitest";

import {
  DEFAULT_CRM_API_URL,
  crmApiBase,
  upstreamUrl,
} from "../../src/lib/server/api-url";

describe("crmApiBase", () => {
  it("defaults to the service's /api path when unset or blank", () => {
    expect(DEFAULT_CRM_API_URL).toBe("http://localhost:5150/api");
    expect(crmApiBase(undefined)).toBe(DEFAULT_CRM_API_URL);
    expect(crmApiBase("")).toBe(DEFAULT_CRM_API_URL);
    expect(crmApiBase("  ")).toBe(DEFAULT_CRM_API_URL);
  });

  it("keeps a set value, dropping trailing slashes only", () => {
    expect(crmApiBase("http://crm:5150/api")).toBe("http://crm:5150/api");
    expect(crmApiBase("http://crm:5150/api/")).toBe("http://crm:5150/api");
    expect(crmApiBase("https://x.example/crm/api//")).toBe(
      "https://x.example/crm/api",
    );
  });

  it("adds no /api of its own", () => {
    expect(crmApiBase("http://crm:5150")).toBe("http://crm:5150");
  });
});

describe("upstreamUrl", () => {
  it("maps /api/proxy/<path>?<query> to <base>/<path>?<query>", () => {
    expect(upstreamUrl(crmApiBase(undefined), "contacts/42", "?limit=5")).toBe(
      "http://localhost:5150/api/contacts/42?limit=5",
    );
    expect(upstreamUrl(crmApiBase("http://crm/api/"), "forecast", "")).toBe(
      "http://crm/api/forecast",
    );
  });
});
