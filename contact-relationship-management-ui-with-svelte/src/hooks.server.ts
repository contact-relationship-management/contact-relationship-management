import type { Handle } from "@sveltejs/kit/hooks";

// BFF entry point: read the httpOnly session cookie on every request
// and expose the opaque session id to server endpoints via `locals`.
// The browser never reads the cookie; only the SvelteKit server does,
// and only the server talks to the services (PF-T18, per
// `agents/share/authentication-sessions.md`).
import { SECURITY_HEADERS } from "#lib/server/headers.js";
import { SESSION_COOKIE } from "#lib/server/session.js";
import { readThemeCookie } from "@lilydesignsystem/svelte-theme-picker";
import {
  CHOICES,
  SYSTEM_THEME,
  THEME_COOKIE,
  themeAttribute,
  themeLinks,
} from "#lib/theme.js";

export const handle: Handle = async ({ event, resolve }) => {
  event.locals.sessionId = event.cookies.get(SESSION_COOKIE) ?? null;
  // The theme the person chose (CRM-R96), kept by the picker in a cookie, so the page arrives in
  // it rather than switching once the script runs. A missing or unknown value is "system".
  const theme =
    readThemeCookie(
      event.request.headers.get("cookie"),
      THEME_COOKIE,
      CHOICES,
    ) || SYSTEM_THEME;
  const response = await resolve(event, {
    transformPageChunk: ({ html }) =>
      html
        .replace("%crm.theme%", themeAttribute(theme))
        .replace("%crm.theme-links%", themeLinks(theme)),
  });
  for (const [name, value] of Object.entries(SECURITY_HEADERS)) {
    response.headers.set(name, value);
  }
  return response;
};
