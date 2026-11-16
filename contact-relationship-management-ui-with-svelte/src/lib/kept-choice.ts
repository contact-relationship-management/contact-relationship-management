// A choice made in one of Lily's pickers, kept in `localStorage` only once the person changes it
// (docs/privacy-notice.md: nothing is stored until a choice is made). Lily's text-size, time-zone and
// locale pickers write their `storageKey` as soon as they apply a value, the starting one included, so
// the app gives them no `storageKey` and keeps the choice here instead.

/** The part of `Storage` this uses; `undefined` where there is none. */
export type ChoiceStorage = Pick<Storage, "getItem" | "setItem">;

function defaultStorage(): ChoiceStorage | undefined {
  try {
    return typeof window === "undefined" ? undefined : window.localStorage;
  } catch {
    return undefined;
  }
}

/**
 * The kept value under `key` (or `start`, or nothing), and an `onChange` for the picker that writes a
 * value only when it differs from the one shown. With no kept value and no `start`, the picker's first
 * report is the value it started on, which is not a choice, so it is not kept.
 */
export function keptChoice(
  key: string,
  start?: string,
  storage: ChoiceStorage | undefined = defaultStorage(),
): { value: string | undefined; onChange: (next: string) => void } {
  let kept: string | null = null;
  try {
    kept = storage?.getItem(key) ?? null;
  } catch {
    // storage refused: nothing is kept
  }
  const value = kept || start || undefined;
  let shown = value;
  return {
    value,
    onChange(next: string) {
      if (shown === undefined) {
        shown = next;
        return;
      }
      if (next === shown) return;
      shown = next;
      try {
        storage?.setItem(key, next);
      } catch {
        // storage refused: the choice lasts for this page only
      }
    },
  };
}
