// The UI's theme (CRM-R96): "Follow this device", the browser's light/dark setting, unless the
// person picks a theme. Lily's ThemePicker offers the choice and keeps it in a cookie; the server
// reads that cookie to send the page already in the chosen theme (`hooks.server.ts`).
import { DEFAULT_THEMES } from "@lilydesignsystem/svelte-picker-bar";
import { SYSTEM_THEME } from "@lilydesignsystem/svelte-theme-picker";

export { SYSTEM_THEME };

/** The cookie that keeps the choice: `system` or a theme's slug. */
export const THEME_COOKIE = "crm-theme";

/**
 * Where the choice was kept in `localStorage` before (`lily-theme` by Lily's picker until CRM-T246,
 * `crm-theme` by this UI until CRM-T249). Both are removed, not read.
 */
export const OLD_THEME_KEYS = ["lily-theme", "crm-theme"] as const;

/** The themes the picker offers, below "Follow this device". */
export const THEMES: readonly string[] = DEFAULT_THEMES;

/** Every value the cookie may hold; anything else is ignored. */
export const CHOICES: readonly string[] = [SYSTEM_THEME, ...THEMES];

const THEMES_URL = "/assets/themes/";

/** Marks the links the server sends, which the picker does not manage. */
const SERVER_LINK = "data-crm-server-theme";

/** The link Lily's ThemePicker manages (its `name` is `theme`). */
const PICKER_LINK = 'link[data-lily-theme-picker="theme"]';

/** The stylesheet links the server puts in the page for a choice. */
export function themeLinks(choice: string): string {
  // The server cannot see the browser's setting, so for "system" it links both themes, each for
  // its own `prefers-color-scheme`, and the browser uses the one that applies.
  if (choice === SYSTEM_THEME) {
    return (
      `<link rel="stylesheet" href="${THEMES_URL}light.css" media="(prefers-color-scheme: light)" ${SERVER_LINK}>` +
      `<link rel="stylesheet" href="${THEMES_URL}dark.css" media="(prefers-color-scheme: dark)" ${SERVER_LINK}>`
    );
  }
  // `choice` is one of CHOICES, so it is a known slug, never text from the request.
  return `<link rel="stylesheet" href="${THEMES_URL}${choice}.css" ${SERVER_LINK}>`;
}

/**
 * Remove the server's links once the picker's own stylesheet has loaded, so a theme picked later
 * is not mixed with them, and the page is never left without a theme while it loads.
 */
export function dropServerLinks(doc: Document): void {
  const server = [...doc.head.querySelectorAll(`link[${SERVER_LINK}]`)];
  if (server.length === 0) return;
  const drop = () => server.forEach((link) => link.remove());
  const picker = doc.head.querySelector<HTMLLinkElement>(PICKER_LINK);
  if (!picker) return;
  if (picker.sheet) drop();
  else picker.addEventListener("load", drop, { once: true });
}

/** The `data-theme` the server puts on `<html>`: none for "system", which the browser resolves. */
export function themeAttribute(choice: string): string {
  return choice === SYSTEM_THEME ? "" : choice;
}

/** Drop the old keys; storage that cannot be used is left as it is. */
export function forgetOldKeys(
  storage: Pick<Storage, "removeItem"> | undefined,
): void {
  for (const key of OLD_THEME_KEYS) {
    try {
      storage?.removeItem(key);
    } catch {
      // A private window or blocked storage: nothing to remove.
    }
  }
}
