import { flushSync } from "svelte";
import { describe, expect, it } from "vitest";
import { ApiError } from "#lib/api/client.js";
import { translate } from "#lib/i18n.svelte.js";
import { resource } from "#lib/resource.svelte.js";

/** A promise the test settles by hand. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (cause: unknown) => void;
  const promise = new Promise<T>((ok, no) => {
    resolve = ok;
    reject = no;
  });
  return { promise, resolve, reject };
}
const settle = () => new Promise((done) => setTimeout(done, 0));

// `errorMessage` itself is covered in errors.test.ts; here it is only what `resource` stores.

describe("resource", () => {
  it("loads once, then shows the value", async () => {
    let calls = 0;
    let r!: ReturnType<typeof resource<number>>;
    const stop = $effect.root(() => {
      r = resource(async () => ++calls);
    });
    flushSync();
    await settle();
    expect(calls).toBe(1);
    expect(r.value).toBe(1);
    expect(r.error).toBeNull();
    stop();
  });

  it("a slow answer for a previous parameter never replaces the current one", async () => {
    const pending = new Map<string, ReturnType<typeof deferred<string>>>();
    let id = $state("a");
    let r!: ReturnType<typeof resource<string>>;
    const stop = $effect.root(() => {
      r = resource(() => {
        const wanted = id; // read before the await, so a change reruns it
        const d = deferred<string>();
        pending.set(wanted, d);
        return d.promise;
      });
    });
    flushSync();
    id = "b";
    flushSync();
    // The newer request answers first, then the older one arrives late.
    pending.get("b")!.resolve("record b");
    await settle();
    pending.get("a")!.resolve("record a");
    await settle();
    expect(r.value).toBe("record b");
    stop();
  });

  it("a late failure of a previous parameter does not show an error", async () => {
    const pending = new Map<string, ReturnType<typeof deferred<string>>>();
    let id = $state("a");
    let r!: ReturnType<typeof resource<string>>;
    const stop = $effect.root(() => {
      r = resource(() => {
        const wanted = id;
        const d = deferred<string>();
        pending.set(wanted, d);
        return d.promise;
      });
    });
    flushSync();
    id = "b";
    flushSync();
    pending.get("b")!.resolve("record b");
    await settle();
    pending.get("a")!.reject(new Error("old failure"));
    await settle();
    expect(r.error).toBeNull();
    expect(r.value).toBe("record b");
    stop();
  });

  it("clears the old record when the parameter changes, but keeps it on reload", async () => {
    let id = $state("a");
    let r!: ReturnType<typeof resource<string>>;
    const gate: { current: ReturnType<typeof deferred<string>> } = {
      current: deferred(),
    };
    const stop = $effect.root(() => {
      r = resource(() => {
        void id;
        gate.current = deferred<string>();
        return gate.current.promise;
      });
    });
    flushSync();
    gate.current.resolve("record a");
    await settle();
    expect(r.value).toBe("record a");

    void r.reload();
    expect(r.value).toBe("record a"); // a reload keeps what is on screen

    gate.current.resolve("record a again");
    await settle();

    id = "b";
    flushSync();
    expect(r.value).toBeNull(); // a new parameter shows loading, not record a
    gate.current.resolve("record b");
    await settle();
    expect(r.value).toBe("record b");
    stop();
  });

  it("reports a failure as text and clears it on the next load", async () => {
    let fail = $state(true);
    let r!: ReturnType<typeof resource<string>>;
    const stop = $effect.root(() => {
      r = resource(async () => {
        if (fail) throw new ApiError(500, { description: "internal detail" });
        return "ok";
      });
    });
    flushSync();
    await settle();
    // The reader gets the page's own words for a failure, not the service's.
    expect(r.error).toBe(translate("error.unavailable", "en-001"));
    fail = false;
    flushSync();
    await settle();
    expect(r.error).toBeNull();
    expect(r.value).toBe("ok");
    stop();
  });
});
