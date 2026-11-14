/**
 * Spike test (k6): VUS virtual users, then ten times as many within ten seconds, held for
 * DURATION, then back to VUS. It passes if the service keeps answering through the spike
 * and recovers after it; the summary shows what the spike cost.
 *
 * Environment: TEST_URL, CRM_AUTH_TOKEN, and
 *   VUS       virtual users before and after the spike   default 5
 *   DURATION  length of the spike                         default 1m
 */

import { COMMON_THRESHOLDS, envInt, mixed, prepare } from '../lib/crm.js';

const VUS = envInt('VUS', 5);

export const options = {
  scenarios: {
    spike: {
      executor: 'ramping-vus',
      startVUs: VUS,
      stages: [
        { duration: '1m', target: VUS },
        { duration: '10s', target: VUS * 10 },
        { duration: __ENV.DURATION || '1m', target: VUS * 10 },
        { duration: '10s', target: VUS },
        { duration: '2m', target: VUS },
        { duration: '10s', target: 0 },
      ],
      gracefulRampDown: '30s',
    },
  },
  thresholds: {
    ...COMMON_THRESHOLDS,
    http_req_failed: ['rate<0.05'],
    checks: ['rate>0.95'],
    http_req_duration: ['p(95)<3000'],
  },
  summaryTrendStats: ['avg', 'med', 'p(90)', 'p(95)', 'p(99)', 'max'],
};

export function setup() {
  return prepare();
}

export default function (data) {
  mixed(data, 1);
}
