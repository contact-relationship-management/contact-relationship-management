/**
 * Breakpoint test (k6): the mixed workload (lib/crm.js) at an arrival rate that climbs steadily
 * from 1 request a second to MAX_RATE, with no pause, until the service fails its limits; the
 * run then stops. The result is the rate it had reached: k6 prints it, as the last
 * `iterations` rate and the scenario's progress, when the threshold aborts the run. A run that
 * reaches MAX_RATE without failing did not find the breakpoint; raise MAX_RATE.
 *
 * Run it only against a throwaway service on its own machine: it is meant to overload it.
 *
 * Environment: TEST_URL, CONTACT_RELATIONSHIP_MANAGEMENT_AUTH_TOKEN, and
 *   MAX_RATE  the highest rate, iterations a second   default 200
 *   DURATION  time to climb to it                     default 10m
 */

import { envInt, mixed, prepare } from '../lib/crm.js';

const MAX_RATE = envInt('MAX_RATE', 200);

export const options = {
  scenarios: {
    breakpoint: {
      executor: 'ramping-arrival-rate',
      startRate: 1,
      timeUnit: '1s',
      preAllocatedVUs: 20,
      maxVUs: Math.max(50, MAX_RATE * 2),
      stages: [{ duration: __ENV.DURATION || '10m', target: MAX_RATE }],
    },
  },
  thresholds: {
    // Wait 30 seconds before judging, so the first slow requests of a cold start do not stop it.
    http_req_failed: [{ threshold: 'rate<0.05', abortOnFail: true, delayAbortEval: '30s' }],
    http_req_duration: [{ threshold: 'p(95)<2000', abortOnFail: true, delayAbortEval: '30s' }],
  },
  summaryTrendStats: ['avg', 'med', 'p(95)', 'p(99)', 'max'],
};

export function setup() {
  return prepare();
}

export default function (data) {
  mixed(data, 0);
}
