import { describe, expect, it } from "vitest";
import { api, readerZone } from "#lib/api/client.js";

// The service's "today" is the reader's when the reader names a zone (CRM-T171).
describe("the reader's time zone goes with every request", () => {
  it("is the browser's IANA zone", () => {
    expect(readerZone()).toBe(Intl.DateTimeFormat().resolvedOptions().timeZone);
  });

  it("is sent as CRM-Time-Zone", async () => {
    let sent: Record<string, string> = {};
    const fetchStub = (async (_url: string, init?: RequestInit) => {
      sent = init?.headers as Record<string, string>;
      return new Response("{}", { status: 200 });
    }) as typeof fetch;
    await api("/settings/organization", { fetch: fetchStub });
    expect(sent["crm-time-zone"]).toBe(readerZone());
  });
});
