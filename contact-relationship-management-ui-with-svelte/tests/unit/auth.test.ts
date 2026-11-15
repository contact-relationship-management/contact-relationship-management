// The BFF's calls to the authentication service when that service is down:
// a rejected `fetch` must become a failure the page can show, never an
// uncaught error (which rendered a blank sign-in page).
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  exchangeToken,
  requestMagicLink,
  signout,
  verifyMagicLink,
} from "../../src/lib/server/auth";

const unreachable = vi.fn(() =>
  Promise.reject(new TypeError("fetch failed")),
) as unknown as typeof fetch;

describe("an unreachable authentication service", () => {
  it("makes a magic-link request report failure", async () => {
    await expect(
      requestMagicLink(
        "someone@example.com",
        "http://app",
        undefined,
        unreachable,
      ),
    ).resolves.toBe(false);
  });

  it("does not stop a sign-out", async () => {
    await expect(signout("a-session", unreachable)).resolves.toBeUndefined();
  });

  it("still reports a refusal as failure", async () => {
    const refused = vi.fn(() =>
      Promise.resolve(new Response(null, { status: 403 })),
    ) as unknown as typeof fetch;
    await expect(
      requestMagicLink("someone@example.com", "http://app", undefined, refused),
    ).resolves.toBe(false);
  });
});

// CRM-T240: with no `fetch` given, each call uses the platform's, and sends no `Origin`. SvelteKit's
// event `fetch` would set one naming the UI, which a service on axa 0.13 refuses on a write.
describe("the calls to the authentication service", () => {
  afterEach(() => vi.unstubAllGlobals());

  function recordCalls(token = "a-token") {
    const calls: { url: string; headers: Headers }[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn((input: RequestInfo | URL, init?: RequestInit) => {
        calls.push({ url: String(input), headers: new Headers(init?.headers) });
        return Promise.resolve(Response.json({ token }));
      }),
    );
    return calls;
  }

  it("use the platform fetch by default and send no Origin", async () => {
    const calls = recordCalls();
    await requestMagicLink("someone@example.com", "http://app");
    await verifyMagicLink("a-link-token");
    expect(await exchangeToken("a-session")).toBe("a-token");
    await signout("a-session");
    expect(calls.map((c) => new URL(c.url).pathname)).toEqual([
      "/api/auth/magic-link",
      "/api/auth/magic-link/a-link-token",
      "/api/auth/token",
      "/api/auth/token",
      "/api/auth/signout",
    ]);
    for (const call of calls) expect(call.headers.has("origin")).toBe(false);
  });
});
