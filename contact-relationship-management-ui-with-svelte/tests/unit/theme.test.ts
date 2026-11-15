// The theme follows the browser's light/dark setting unless the person picked one (CRM-R96).
import { describe, expect, it } from "vitest";
import {
  OLD_THEME_KEY,
  THEME_KEY,
  forgetOldKey,
  readChosen,
  rememberChosen,
  startingTheme,
  systemTheme,
} from "../../src/lib/theme";

function memory(initial: Record<string, string> = {}) {
  const items = new Map(Object.entries(initial));
  return {
    items,
    getItem: (key: string) => items.get(key) ?? null,
    setItem: (key: string, value: string) => void items.set(key, value),
    removeItem: (key: string) => void items.delete(key),
  };
}

const broken = {
  getItem: () => {
    throw new Error("blocked");
  },
  setItem: () => {
    throw new Error("blocked");
  },
  removeItem: () => {
    throw new Error("blocked");
  },
};

describe("the theme", () => {
  it("is the browser's setting when nothing was picked", () => {
    expect(systemTheme(true)).toBe("dark");
    expect(systemTheme(false)).toBe("light");
    expect(startingTheme(null, true)).toBe("dark");
    expect(startingTheme(null, false)).toBe("light");
  });

  it("is the person's pick when there is one, whatever the browser says", () => {
    expect(startingTheme("cupcake", true)).toBe("cupcake");
    expect(startingTheme("light", true)).toBe("light");
  });

  it("remembers only under its own key, and drops the old one", () => {
    const storage = memory({ [OLD_THEME_KEY]: "dark" });
    expect(readChosen(storage)).toBeNull();
    rememberChosen(storage, "cupcake");
    expect(readChosen(storage)).toBe("cupcake");
    expect(storage.items.get(THEME_KEY)).toBe("cupcake");
    forgetOldKey(storage);
    expect(storage.items.has(OLD_THEME_KEY)).toBe(false);
  });

  it("treats storage that cannot be used as no pick", () => {
    expect(readChosen(broken)).toBeNull();
    expect(readChosen(undefined)).toBeNull();
    expect(() => rememberChosen(broken, "dark")).not.toThrow();
    expect(() => forgetOldKey(broken)).not.toThrow();
  });
});
