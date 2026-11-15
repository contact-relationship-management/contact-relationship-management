// The UI's theme (CRM-R96): the browser's light/dark setting, unless the person has picked a
// theme in the UI. Only a pick is remembered, so a visitor who never picks one follows the browser
// on every visit, and while a page is open.

/** Where a theme the person picked is kept. */
export const THEME_KEY = "crm-theme";

/**
 * Where Lily's ThemePicker kept the theme before CRM-T246. It saved whatever theme it applied,
 * the browser's included, so it cannot tell a pick from a default; it is removed, not read.
 */
export const OLD_THEME_KEY = "lily-theme";

/** The theme for the browser's setting. */
export function systemTheme(prefersDark: boolean): "dark" | "light" {
  return prefersDark ? "dark" : "light";
}

/** The theme to show: the person's pick, else the browser's setting. */
export function startingTheme(
  chosen: string | null,
  prefersDark: boolean,
): string {
  return chosen || systemTheme(prefersDark);
}

/** The theme the person picked, or `null`; storage that cannot be read counts as none. */
export function readChosen(
  storage: Pick<Storage, "getItem"> | undefined,
): string | null {
  try {
    return storage?.getItem(THEME_KEY) || null;
  } catch {
    return null;
  }
}

/** Remember a theme the person picked; storage that cannot be written is left as it is. */
export function rememberChosen(
  storage: Pick<Storage, "setItem"> | undefined,
  slug: string,
): void {
  try {
    storage?.setItem(THEME_KEY, slug);
  } catch {
    // A private window or blocked storage: the pick lasts for this page only.
  }
}

/** Drop the old key, whose value may be a default rather than a pick. */
export function forgetOldKey(
  storage: Pick<Storage, "removeItem"> | undefined,
): void {
  try {
    storage?.removeItem(OLD_THEME_KEY);
  } catch {
    // Nothing to do.
  }
}
