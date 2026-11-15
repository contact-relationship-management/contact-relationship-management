// BFF server-side calls to the authentication service (never bundled into
// the browser). The SvelteKit server holds the session cookie and does all
// token handling here; the browser never sees an access token.

import { AUTH_API_URL } from "./config";
import { SESSION_COOKIE } from "./session";

type FetchFn = typeof fetch;

// Each call defaults to the platform's `fetch`. Do not pass SvelteKit's event `fetch`: on a request
// to another origin it sets `Origin` to this UI, and a service on axa 0.13 refuses a write whose
// `Origin` is not its own host when there is no `Sec-Fetch-Site` (CRM-T240, as CRM-T237 found in
// the API proxy). The parameter is there for tests.
const platformFetch: FetchFn = (input, init) => globalThis.fetch(input, init);

/** Consume a magic-link token; returns the raw upstream response so the
 *  caller can read the `Set-Cookie` that establishes the session. */
export function verifyMagicLink(
  token: string,
  fetchFn: FetchFn = platformFetch,
): Promise<Response> {
  return fetchFn(
    `${AUTH_API_URL}/api/auth/magic-link/${encodeURIComponent(token)}`,
  );
}

/** Request a magic link for an existing account, telling the auth service
 *  to return to THIS app's `/verify` (per-app login). `false` when the
 *  service refuses or cannot be reached, so the page says it failed. */
export async function requestMagicLink(
  email: string,
  returnUrl: string,
  locale?: string,
  fetchFn: FetchFn = platformFetch,
): Promise<boolean> {
  try {
    const res = await fetchFn(`${AUTH_API_URL}/api/auth/magic-link`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ email, locale, return_url: returnUrl }),
    });
    return res.ok;
  } catch {
    return false;
  }
}

/** Exchange the opaque session id for a short-lived PASETO (server-to-
 *  server; sends the session as a `Cookie` header). `null` if invalid. */
export async function exchangeToken(
  sid: string,
  fetchFn: FetchFn = platformFetch,
): Promise<string | null> {
  const res = await fetchFn(`${AUTH_API_URL}/api/auth/token`, {
    method: "POST",
    headers: { cookie: `${SESSION_COOKIE}=${sid}` },
  });
  if (!res.ok) return null;
  const body = (await res.json()) as { token?: string };
  return body.token ?? null;
}

/** Revoke the session server-side (best-effort) before the cookie clear:
 *  an unreachable auth service must not stop the sign-out. */
export async function signout(
  sid: string,
  fetchFn: FetchFn = platformFetch,
): Promise<void> {
  try {
    const token = await exchangeToken(sid, fetchFn);
    if (!token) return;
    await fetchFn(`${AUTH_API_URL}/api/auth/signout`, {
      method: "POST",
      headers: { authorization: `Bearer ${token}` },
    });
  } catch {
    // Best-effort: the cookie is cleared either way.
  }
}

/**
 * The auth service's OIDC login URL (EV-2, `authentication-sessions.md`
 * §7a). A BROWSER NAVIGATION target, not a BFF `fetch`: the browser itself
 * must visit the identity provider and come back to `originForReturn`.
 */
export function oidcLoginUrl(originForReturn: string): string {
  const params = new URLSearchParams({ return_url: originForReturn });
  return `${AUTH_API_URL}/api/auth/oidc/login?${params.toString()}`;
}
