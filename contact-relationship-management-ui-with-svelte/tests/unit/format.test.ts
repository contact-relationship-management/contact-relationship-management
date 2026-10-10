// Unit tests: honest-ratio rendering, stage labels and SLA-breach helpers.

import { describe, expect, it } from "vitest";

import {
  breachedCount,
  ratioFraction,
  ratioLabel,
  ratioPercent,
  slaBreached,
  stageLabel,
} from "../../src/lib/format";

describe("ratios", () => {
  it("renders a zero-denominator (null value) as a dash, never 0%", () => {
    const zero = { numerator: 0, denominator: 0, value: null };
    expect(ratioPercent(zero)).toBe("—");
    expect(ratioLabel(zero)).toBe("—");
    expect(ratioLabel(zero, "No data")).toBe("No data");
  });

  it("renders absent ratios as a dash with no fraction", () => {
    expect(ratioPercent(null)).toBe("—");
    expect(ratioPercent(undefined)).toBe("—");
    expect(ratioFraction(null)).toBe("");
    expect(ratioLabel(undefined, "No data")).toBe("No data");
  });

  it("keeps a genuine 0% distinct from no data", () => {
    const real = { numerator: 0, denominator: 4, value: 0 };
    expect(ratioPercent(real)).toBe("0%");
    expect(ratioLabel(real)).toBe("0% (0/4)");
  });

  it("rounds to a whole percent and shows provenance", () => {
    expect(ratioLabel({ numerator: 1, denominator: 3, value: 1 / 3 })).toBe(
      "33% (1/3)",
    );
    expect(ratioLabel({ numerator: 2, denominator: 3, value: 2 / 3 })).toBe(
      "67% (2/3)",
    );
    expect(ratioPercent({ numerator: 1, denominator: 1, value: 1 })).toBe(
      "100%",
    );
  });
});

describe("stageLabel", () => {
  it("shows the stage name and its probability", () => {
    expect(stageLabel({ name: "Proposal", probability_percent: 60 })).toBe(
      "Proposal 60%",
    );
    expect(stageLabel({ name: "Lost", probability_percent: 0 })).toBe(
      "Lost 0%",
    );
  });
});

describe("SLA breach", () => {
  it("flags a ticket when either clock breached", () => {
    expect(slaBreached({ live_first_response_breached: true })).toBe(true);
    expect(slaBreached({ live_resolution_breached: true })).toBe(true);
    expect(
      slaBreached({
        live_first_response_breached: true,
        live_resolution_breached: true,
      }),
    ).toBe(true);
  });

  it("does not flag unbreached or unknown flags", () => {
    expect(slaBreached({})).toBe(false);
    expect(
      slaBreached({
        live_first_response_breached: false,
        live_resolution_breached: false,
      }),
    ).toBe(false);
  });

  it("counts breached tickets, and zero for an empty queue", () => {
    expect(breachedCount([])).toBe(0);
    expect(
      breachedCount([
        { live_first_response_breached: true },
        {},
        { live_resolution_breached: true },
      ]),
    ).toBe(2);
  });
});
