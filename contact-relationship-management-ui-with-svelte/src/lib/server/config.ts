// Server-side (BFF) configuration. Never imported by browser code.

import * as env from "$app/env/private";

/** Base URL of the contact-relationship-management service the BFF proxies to. */
export const CRM_API_URL: string = env.CRM_API_URL || "http://localhost:5150";

/** Authentication service base URL — for the session→PASETO exchange
 *  and the magic-link login flow (PF-T18). */
export const AUTH_API_URL: string = env.AUTH_API_URL || "http://localhost:5150";

/**
 * The guided tour (sign in as example@example.com, explore fictitious data).
 * On unless `CRM_DEMO_TOUR=off`; a deployment that must not offer it sets that.
 * Demo sessions never reach the service, whatever this is set to.
 */
export const DEMO_TOUR_ENABLED: boolean =
  (env.CRM_DEMO_TOUR || "on").toLowerCase() !== "off";
