import { describe, expect, it } from "vitest";
import { demoTourEnabled } from "../../src/lib/server/demo-config";

describe("guided-tour configuration", () => {
  it("is disabled unless the operator explicitly opts in", () => {
    for (const value of [undefined, "", "off", "false", "1", "yes"]) {
      expect(demoTourEnabled(value), String(value)).toBe(false);
    }
  });

  it("accepts on without depending on case or surrounding whitespace", () => {
    for (const value of ["on", "ON", " On "]) {
      expect(demoTourEnabled(value), value).toBe(true);
    }
  });
});
