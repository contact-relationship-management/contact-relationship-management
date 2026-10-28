import { describe, expect, it } from "vitest";

import { ticketReference } from "#lib/api/crm.js";

describe("ticketReference (CRM-R88)", () => {
  it("is T- and the first eight hex digits of the pid, as the service makes it", () => {
    // The service's own test uses this pid and expects T-0a1b2c3d.
    expect(ticketReference("0a1b2c3d-1111-2222-3333-444455556666")).toBe(
      "T-0a1b2c3d",
    );
  });

  it("is in lower case whatever the pid's case", () => {
    expect(ticketReference("0A1B2C3D-1111-2222-3333-444455556666")).toBe(
      "T-0a1b2c3d",
    );
  });
});
