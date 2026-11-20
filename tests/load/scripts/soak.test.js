/**
 * Soak test (k6): the basic test's mixed workload (lib/crm.js) for hours, to show what only
 * time shows: a leak, a pool that runs dry, a table that slows as it grows. For a run that
 * also compares the first and last tenth and probes health on its own, use
 * tests/long-running/, which is built for that.
 *
 * Environment: TEST_URL, CONTACT_RELATIONSHIP_MANAGEMENT_AUTH_TOKEN, and
 *   VUS       virtual users       default 10
 *   DURATION  steady duration     default 2h
 */

import { COMMON_THRESHOLDS, envInt, mixed, prepare } from '../lib/crm.js';

const VUS = envInt('VUS', 10);

export const options = {
  scenarios: {
    soak: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '2m', target: VUS },
        { duration: __ENV.DURATION || '2h', target: VUS },
        { duration: '2m', target: 0 },
      ],
      gracefulRampDown: '30s',
    },
  },
  thresholds: {
    ...COMMON_THRESHOLDS,
    http_req_duration: ['p(95)<500', 'p(99)<1500'],
  },
  summaryTrendStats: ['avg', 'med', 'p(90)', 'p(95)', 'p(99)', 'max'],
};

export function setup() {
  return prepare();
}

export default function (data) {
  mixed(data, 2);
}
