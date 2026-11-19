import { describe, expect, it } from "vitest";

import {
  SESSION_COOKIE,
  SESSION_COOKIE_OPTIONS,
  parseSessionId,
  sessionIdFromResponse,
} from "../../src/lib/server/session";

describe("server session cookie parsing", () => {
  it("pins the host-only secure cookie policy", () => {
    expect(SESSION_COOKIE).toBe("__Host-mxi_session");
    expect(SESSION_COOKIE_OPTIONS).toEqual({
      path: "/",
      httpOnly: true,
      secure: true,
      sameSite: "lax",
    });
  });

  it("extracts a non-empty session from any cookie segment", () => {
    expect(
      parseSessionId(
        "theme=dark; __Host-mxi_session=opaque-session; Secure; HttpOnly",
      ),
    ).toBe("opaque-session");
    expect(parseSessionId("theme=dark; Path=/")).toBeNull();
    expect(parseSessionId("__Host-mxi_session=; Secure")).toBeNull();
  });

  it("uses every upstream Set-Cookie line when the runtime exposes it", () => {
    const response = new Response(null);
    Object.defineProperty(response.headers, "getSetCookie", {
      value: () => [
        "theme=dark; Path=/",
        "__Host-mxi_session=second-line; Secure; HttpOnly",
      ],
    });
    expect(sessionIdFromResponse(response)).toBe("second-line");
  });

  it("falls back to the combined header and returns null when absent", () => {
    expect(
      sessionIdFromResponse(
        new Response(null, {
          headers: {
            "set-cookie": "__Host-mxi_session=fallback; Secure; HttpOnly",
          },
        }),
      ),
    ).toBe("fallback");
    expect(sessionIdFromResponse(new Response(null))).toBeNull();
  });
});
