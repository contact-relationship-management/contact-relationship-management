// Unit tests for the /verify BFF load function, run directly (no browser,
// no server) — SvelteKit server `load` functions are plain async
// functions, so this is the cheapest way to pin the error-branch
// behaviour without standing up a whole Playwright + stub-auth-server
// harness. See `+page.server.ts`'s own comment for the bug this pins.
import { afterEach, describe, expect, it, vi } from "vitest";
import { load } from "../../src/routes/verify/+page.server";

// `cookies.set` is never called on any of these error branches (it only
// runs after a successful upstream response), so a no-op stub is enough.
const cookies = { set: vi.fn() } as unknown as Parameters<
  typeof load
>[0]["cookies"];

// The load calls the authentication service with the platform's `fetch`, not the event's
// (CRM-T240), so each test stubs the global one.
describe("verify load", () => {
  afterEach(() => vi.unstubAllGlobals());

  it("returns missingToken when no token is present", async () => {
    const result = await load({
      url: new URL("http://localhost/verify"),
      cookies,
    } as unknown as Parameters<typeof load>[0]);
    expect(result).toEqual({
      error: "missingToken",
      title: "Sign-in link — Contact Relationship Management",
    });
  });

  // Pins the fix: a network-level failure (fetch rejects) must not
  // propagate out of `load` — it must resolve to a friendly error
  // state instead.
  it("returns serviceUnavailable when the upstream fetch rejects", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockRejectedValue(new Error("fetch failed")),
    );
    const result = await load({
      url: new URL("http://localhost/verify?token=abc"),
      cookies,
    } as unknown as Parameters<typeof load>[0]);
    expect(result).toEqual({ error: "serviceUnavailable" });
  });

  it("returns invalidToken when the upstream responds not-ok", async () => {
    const upstream = vi
      .fn()
      .mockResolvedValue(new Response(null, { status: 401 }));
    vi.stubGlobal("fetch", upstream);
    const result = await load({
      url: new URL("http://localhost/verify?token=abc"),
      cookies,
    } as unknown as Parameters<typeof load>[0]);
    expect(result).toEqual({
      error: "invalidToken",
      title: "Sign-in link — Contact Relationship Management",
    });
    expect(upstream).toHaveBeenCalledOnce();
  });
});
