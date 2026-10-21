// Sign-in action (BFF, per-app magic-link login, PF-T18): the server
// asks the authentication service for a magic link that returns to
// THIS app's own `/verify` (via `return_url` = this origin, honoured
// by the auth service's allow-list). No credential is held
// client-side.

import type { Actions, PageServerLoad } from "./$types";
import { redirect } from "@sveltejs/kit";
import { requestMagicLink } from "#lib/server/auth.js";
import { DEMO_TOUR_ENABLED } from "#lib/server/config.js";
import { DEMO_EMAIL, isDemoEmail, startDemoSession } from "#lib/server/demo.js";
import { SESSION_COOKIE, SESSION_COOKIE_OPTIONS } from "#lib/server/session.js";

// `page.data.title` convention (see `../+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title
// without reading the DOM.
export const load: PageServerLoad = () => {
  return {
    title: "Sign in — Contact Relationship Management",
    demoTour: DEMO_TOUR_ENABLED ? DEMO_EMAIL : null,
  };
};

export const actions: Actions = {
  default: async ({ request, fetch, url, cookies }) => {
    const form = await request.formData();
    const email = String(form.get("email") ?? "").trim();
    if (!email) {
      return { sent: false, error: "email-required" };
    }
    // The guided tour: no email is sent; the session is a new demo session
    // with its own copy of the data.
    if (DEMO_TOUR_ENABLED && isDemoEmail(email)) {
      cookies.set(SESSION_COOKIE, startDemoSession(), SESSION_COOKIE_OPTIONS);
      redirect(303, "/");
    }
    const ok = await requestMagicLink(fetch, email, url.origin);
    return ok ? { sent: true, error: null } : { sent: false, error: "failed" };
  },
};
