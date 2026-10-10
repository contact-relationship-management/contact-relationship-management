// Build/runtime configuration for the contact-relationship-management operator front-end.
//
// API calls go to the same-origin BFF proxy (`/api/proxy/...`); the
// browser never holds a token (see `../spec/auth.md`). The base is
// deliberately **relative**: browser fetch resolves it against the
// page origin, and server `load` functions pass SvelteKit's `fetch`,
// which resolves relative URLs internally — so the same client works
// under dev, preview, and production without knowing its own port.

/** Base path the API client calls: the same-origin BFF proxy. */
export const API_BASE_URL = "/api/proxy";

/** Whiteboard poll interval (ms). */
export const BOARD_POLL_MS = 15_000;

/**
 * Idle sign-out (CRM-T43): after this long with no pointer, key, or scroll
 * activity in the tab, the session is revoked and the user is sent to sign
 * in. Polling does not count as activity. 15 minutes matches the common
 * HIPAA/NIST AC-11 guidance for clinical and government workstations.
 */
export const IDLE_TIMEOUT_MS = 15 * 60 * 1000;
