/**
 * Basic load test (k6): a steady number of virtual users on the mixed workload (lib/crm.js),
 * the load the service should carry on an ordinary day.
 *
 * Environment: TEST_URL, CONTACT_RELATIONSHIP_MANAGEMENT_AUTH_TOKEN, and
 *   VUS       virtual users       default 10
 *   DURATION  steady duration     default 5m
 */

import { COMMON_THRESHOLDS, envInt, mixed, prepare } from '../lib/crm.js';

export const options = {
  scenarios: {
    basic: {
      executor: 'constant-vus',
      vus: envInt('VUS', 10),
      duration: __ENV.DURATION || '5m',
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
  mixed(data, 1);
}
