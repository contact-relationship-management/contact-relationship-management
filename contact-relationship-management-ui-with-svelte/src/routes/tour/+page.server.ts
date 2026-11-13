// `page.data.title` convention (see `../+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title.
import { error, redirect } from "@sveltejs/kit";
import type { Actions, PageServerLoad } from "./$types";
import { DEMO_TOUR_ENABLED } from "#lib/server/config.js";
import { isDemoSession, startDemoSession } from "#lib/server/demo.js";
import { SESSION_COOKIE, SESSION_COOKIE_OPTIONS } from "#lib/server/session.js";

// Reading the page starts nothing: a crawler that fetches it (and keeps no
// cookie) must not create tour sessions, which are capped and evict the
// oldest. A visitor starts the tour with the button, a form POST (CRM-T233).
export const load: PageServerLoad = () => {
  return { title: "Tour · Main X · CRM", demoTour: DEMO_TOUR_ENABLED };
};

export const actions: Actions = {
  // One click into the tour, the same session the sign-in page gives
  // example@example.com. A visitor already in the tour keeps their copy.
  start: ({ cookies, locals }) => {
    if (!DEMO_TOUR_ENABLED) error(404, "The tour is off");
    if (!isDemoSession(locals.sessionId)) {
      cookies.set(SESSION_COOKIE, startDemoSession(), SESSION_COOKIE_OPTIONS);
    }
    redirect(303, "/");
  },
};
