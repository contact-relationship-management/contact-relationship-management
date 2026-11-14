/**
 * Stress test (k6): the mixed workload (lib/crm.js) in steps of one, two and four times VUS,
 * with less pause between calls, to find where latency and errors start to climb. The
 * limits are looser than the basic test's; the summary per step is the result.
 *
 * Environment: TEST_URL, CRM_AUTH_TOKEN, and
 *   VUS       virtual users at the first step   default 10
 *   DURATION  length of each step               default 2m
 */

import { COMMON_THRESHOLDS, envInt, mixed, prepare } from '../lib/crm.js';

const VUS = envInt('VUS', 10);
const STEP = __ENV.DURATION || '2m';

export const options = {
  scenarios: {
    stress: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '30s', target: VUS },
        { duration: STEP, target: VUS },
        { duration: '30s', target: VUS * 2 },
        { duration: STEP, target: VUS * 2 },
        { duration: '30s', target: VUS * 4 },
        { duration: STEP, target: VUS * 4 },
        { duration: '30s', target: 0 },
      ],
      gracefulRampDown: '30s',
    },
  },
  thresholds: {
    ...COMMON_THRESHOLDS,
    http_req_failed: ['rate<0.05'],
    checks: ['rate>0.95'],
    http_req_duration: ['p(95)<2000'],
  },
  summaryTrendStats: ['avg', 'med', 'p(90)', 'p(95)', 'p(99)', 'max'],
};

export function setup() {
  return prepare();
}

export default function (data) {
  mixed(data, 0.5);
}
