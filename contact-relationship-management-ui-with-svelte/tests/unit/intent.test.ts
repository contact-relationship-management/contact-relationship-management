import { describe, expect, it } from "vitest";
import { createIntent, createIntents } from "#lib/intent.js";

describe("createIntent", () => {
  it("keeps one key while the content is unchanged, across retries", () => {
    const intent = createIntent();
    const first = intent.keyFor({ reason: "r", minutes: 30 });
    expect(intent.keyFor({ reason: "r", minutes: 30 })).toBe(first);
    expect(intent.keyFor({ reason: "r", minutes: 30 })).toBe(first);
  });

  it("starts a new key when the content changes", () => {
    const intent = createIntent();
    const first = intent.keyFor({ reason: "a" });
    const second = intent.keyFor({ reason: "b" });
    expect(second).not.toBe(first);
    // And that new key is then stable for the new content.
    expect(intent.keyFor({ reason: "b" })).toBe(second);
  });

  it("starts a new key after a success", () => {
    const intent = createIntent();
    const first = intent.keyFor({ n: 1 });
    intent.done();
    expect(intent.keyFor({ n: 1 })).not.toBe(first);
  });
});

describe("createIntents", () => {
  it("holds a separate intent per id", () => {
    const intentFor = createIntents();
    const a = intentFor("a").keyFor({ note: "x" });
    const b = intentFor("b").keyFor({ note: "x" });
    expect(a).not.toBe(b);
    expect(intentFor("a").keyFor({ note: "x" })).toBe(a);
  });
});
