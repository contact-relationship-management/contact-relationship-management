import { resolve } from "$app/paths";

// resolve() is typed for this app's own route ids; a route built at run time is a plain string.
const resolvePath = resolve as unknown as (route: string) => string;

/** A site route ("/install/") as an href that honours the configured base path. */
export function href(route: string): string {
  return resolvePath(route);
}
