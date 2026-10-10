// Pins the security header set every UI response carries (CRM-T33).

import { describe, expect, it } from "vitest";

import { SECURITY_HEADERS } from "../../src/lib/server/headers";

describe("SECURITY_HEADERS", () => {
  it("denies sniffing, framing, and referrer leakage", () => {
    expect(SECURITY_HEADERS["X-Content-Type-Options"]).toBe("nosniff");
    expect(SECURITY_HEADERS["X-Frame-Options"]).toBe("DENY");
    expect(SECURITY_HEADERS["Referrer-Policy"]).toBe("no-referrer");
  });

  it("pins HTTPS for at least a year, with subdomains", () => {
    const hsts = SECURITY_HEADERS["Strict-Transport-Security"];
    const age = Number(/max-age=(\d+)/.exec(hsts)?.[1]);
    expect(age).toBeGreaterThanOrEqual(31536000);
    expect(hsts).toContain("includeSubDomains");
  });

  it("switches off powerful browser features", () => {
    for (const feature of ["camera", "microphone", "geolocation"]) {
      expect(SECURITY_HEADERS["Permissions-Policy"]).toContain(`${feature}=()`);
    }
  });
});
