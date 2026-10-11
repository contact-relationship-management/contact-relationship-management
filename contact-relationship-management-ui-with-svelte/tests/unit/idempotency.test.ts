import { describe, expect, it } from "vitest";
import { api, newIdempotencyKey } from "#lib/api/client.js";

type Call = { url: string; init: RequestInit };

function recorder(delayMs = 0) {
  const calls: Call[] = [];
  const fetcher = (async (url: string, init: RequestInit) => {
    calls.push({ url, init });
    if (delayMs) await new Promise((r) => setTimeout(r, delayMs));
    return new Response(JSON.stringify({ ok: true }), { status: 200 });
  }) as unknown as typeof fetch;
  return { calls, fetcher };
}

const header = (call: Call | undefined) =>
  (call?.init.headers as Record<string, string> | undefined)?.[
    "idempotency-key"
  ];

describe("Idempotency-Key", () => {
  it("is sent on POST, with a valid key, and not on GET", async () => {
    const { calls, fetcher } = recorder();
    await api("/contacts", { method: "POST", body: { a: 1 }, fetch: fetcher });
    await api("/contacts", { fetch: fetcher });
    const key = header(calls[0]);
    expect(key).toMatch(/^[\x21-\x7e]{1,255}$/);
    expect(header(calls[1])).toBeUndefined();
  });

  it("honours an explicit key so retries can reuse it", async () => {
    const { calls, fetcher } = recorder();
    await api("/x", {
      method: "POST",
      idempotencyKey: "intent-1",
      fetch: fetcher,
    });
    await api("/x", {
      method: "POST",
      idempotencyKey: "intent-1",
      fetch: fetcher,
    });
    expect(calls.map(header)).toEqual(["intent-1", "intent-1"]);
  });

  it("uses a fresh key for each separate call", async () => {
    const { calls, fetcher } = recorder();
    await api("/x", { method: "POST", body: 1, fetch: fetcher });
    await api("/x", { method: "POST", body: 1, fetch: fetcher });
    expect(header(calls[0])).not.toEqual(header(calls[1]));
  });

  it("sends one request for identical POSTs issued while one is in flight", async () => {
    const { calls, fetcher } = recorder(20);
    const [a, b] = await Promise.all([
      api("/x", { method: "POST", body: { n: 1 }, fetch: fetcher }),
      api("/x", { method: "POST", body: { n: 1 }, fetch: fetcher }),
    ]);
    expect(calls).toHaveLength(1);
    expect(a).toEqual(b);
    // Different content is a different action.
    await Promise.all([
      api("/x", { method: "POST", body: { n: 1 }, fetch: fetcher }),
      api("/x", { method: "POST", body: { n: 2 }, fetch: fetcher }),
    ]);
    expect(calls).toHaveLength(3);
  });

  it("generates distinct valid keys", () => {
    const keys = new Set(Array.from({ length: 50 }, newIdempotencyKey));
    expect(keys.size).toBe(50);
    for (const k of keys) expect(k).toMatch(/^[\x21-\x7e]{1,255}$/);
  });
});
