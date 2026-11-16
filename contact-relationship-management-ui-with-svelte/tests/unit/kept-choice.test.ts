// docs/privacy-notice.md: a picker's choice is kept only once the person changes it.
import { describe, expect, it } from "vitest";
import { keptChoice, type ChoiceStorage } from "../../src/lib/kept-choice";

function memory(
  initial: Record<string, string> = {},
): ChoiceStorage & { items: Map<string, string> } {
  const items = new Map(Object.entries(initial));
  return {
    items,
    getItem: (key) => items.get(key) ?? null,
    setItem: (key, value) => void items.set(key, value),
  };
}

describe("a kept choice", () => {
  it("keeps nothing for the value the picker starts on", () => {
    const storage = memory();
    const choice = keptChoice("size", "medium", storage);
    expect(choice.value).toBe("medium");
    choice.onChange("medium");
    expect(storage.items.size).toBe(0);
  });

  it("keeps a value the person changes to, and a change back", () => {
    const storage = memory({ size: "large" });
    const choice = keptChoice("size", "medium", storage);
    expect(choice.value).toBe("large");
    choice.onChange("large");
    choice.onChange("medium");
    expect(storage.items.get("size")).toBe("medium");
    choice.onChange("large");
    expect(storage.items.get("size")).toBe("large");
  });

  it("with no start, takes the picker's first report as the start and keeps what follows", () => {
    const storage = memory();
    const choice = keptChoice("zone", undefined, storage);
    expect(choice.value).toBeUndefined();
    choice.onChange("Europe/London");
    expect(storage.items.size).toBe(0);
    choice.onChange("Asia/Tokyo");
    expect(storage.items.get("zone")).toBe("Asia/Tokyo");
  });

  it("works when storage refuses, or there is none", () => {
    const refusing: ChoiceStorage = {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("blocked");
      },
    };
    const choice = keptChoice("size", "medium", refusing);
    expect(choice.value).toBe("medium");
    expect(() => choice.onChange("large")).not.toThrow();
    expect(keptChoice("size", "medium", undefined).value).toBe("medium");
  });
});
