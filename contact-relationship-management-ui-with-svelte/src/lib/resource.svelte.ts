import { errorMessage } from "#lib/errors.js";

/**
 * A page's data, fetched when the component starts and again whenever
 * anything `fetcher` reads before its first `await` changes (a route
 * parameter, a filter). Call it while the component initialises.
 *
 * Only the latest request may write: a slow answer for a previous
 * parameter is dropped, so it can never replace the current one. When the
 * parameters change, the old value is cleared at once (the page shows
 * "loading", not the previous record); `reload()` after a write keeps
 * the value on screen until the new one arrives. `{ keepWhileLoading: true }`
 * does the same for a parameter change, for a page whose controls (a
 * pipeline picker) are drawn from the value.
 */
export function resource<T>(
  fetcher: () => Promise<T>,
  options: { keepWhileLoading?: boolean } = {},
) {
  let value = $state<T | null>(null);
  let error = $state<string | null>(null);
  let latest = 0;

  async function run(keep: boolean): Promise<void> {
    const mine = ++latest;
    if (!keep && !options.keepWhileLoading) value = null;
    error = null;
    try {
      const next = await fetcher();
      if (mine === latest) value = next;
    } catch (cause) {
      if (mine === latest) error = errorMessage(cause);
    }
  }

  $effect(() => {
    void run(false);
  });

  return {
    get value(): T | null {
      return value;
    },
    set value(next: T | null) {
      value = next;
    },
    get error(): string | null {
      return error;
    },
    set error(next: string | null) {
      error = next;
    },
    reload: () => run(true),
  };
}
