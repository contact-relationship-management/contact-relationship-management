// The BFF's calls to the authentication service when that service is down:
// a rejected `fetch` must become a failure the page can show, never an
// uncaught error (which rendered a blank sign-in page).
import { describe, expect, it, vi } from "vitest";
import { requestMagicLink, signout } from "../../src/lib/server/auth";

const unreachable = vi.fn(() =>
  Promise.reject(new TypeError("fetch failed")),
) as unknown as typeof fetch;

describe("an unreachable authentication service", () => {
  it("makes a magic-link request report failure", async () => {
    await expect(
      requestMagicLink(unreachable, "someone@example.com", "http://app"),
    ).resolves.toBe(false);
  });

  it("does not stop a sign-out", async () => {
    await expect(signout(unreachable, "a-session")).resolves.toBeUndefined();
  });

  it("still reports a refusal as failure", async () => {
    const refused = vi.fn(() =>
      Promise.resolve(new Response(null, { status: 403 })),
    ) as unknown as typeof fetch;
    await expect(
      requestMagicLink(refused, "someone@example.com", "http://app"),
    ).resolves.toBe(false);
  });
});
