import type { Handle } from "@sveltejs/kit/hooks";

// BFF entry point: read the httpOnly session cookie on every request
// and expose the opaque session id to server endpoints via `locals`.
// The browser never reads the cookie; only the SvelteKit server does,
// and only the server talks to the services (PF-T18, per
// `agents/share/authentication-sessions.md`).
import { SECURITY_HEADERS } from "#lib/server/headers.js";
import { SESSION_COOKIE } from "#lib/server/session.js";

export const handle: Handle = async ({ event, resolve }) => {
  event.locals.sessionId = event.cookies.get(SESSION_COOKIE) ?? null;
  const response = await resolve(event);
  for (const [name, value] of Object.entries(SECURITY_HEADERS)) {
    response.headers.set(name, value);
  }
  return response;
};
