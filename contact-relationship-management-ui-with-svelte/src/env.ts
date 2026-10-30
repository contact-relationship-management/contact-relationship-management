import { defineEnvVars } from "@sveltejs/kit/env";

// Unset reads as "". That is intended: `src/lib/server/config.ts` turns ""
// into the defaults (the service's `/api` and the auth service on
// localhost:5150, the tour on), so the UI starts with nothing set, as the
// tour needs.
export const variables = defineEnvVars({
  PUBLIC_OIDC_SIGNIN_ENABLED: { public: true, schema: (input) => input ?? "" },
  CRM_API_URL: { schema: (input) => input ?? "" },
  AUTH_API_URL: { schema: (input) => input ?? "" },
  CRM_DEMO_TOUR: { schema: (input) => input ?? "" },
});
