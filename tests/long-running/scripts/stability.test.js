/**
 * Long-running stability test (k6).
 *
 * A steady, mostly-read workload against the CRM service for hours, with a
 * separate health probe. It fails if errors, latency or health failures cross
 * the thresholds below, and it reports latency for the first and last tenth of
 * the workload side by side so drift shows in the summary.
 *
 * It writes synthetic contacts and activities and never deletes them (the API
 * has no contact delete; erasure is a privacy operation). Run it against a
 * throwaway database only, such as `podman compose up --build` from the
 * repository root.
 *
 * Environment (k6 `--env` or `-e`):
 *   TEST_URL        service base URL        default http://localhost:1241
 *   CRM_AUTH_TOKEN  bearer token            default none (demo has auth off)
 *   VUS             steady virtual users    default 5
 *   DURATION        steady-state duration   default 6h
 *   RAMP            ramp-up and ramp-down   default 5m
 */

import http from 'k6/http';
import exec from 'k6/execution';
import { check, sleep } from 'k6';
import { Counter } from 'k6/metrics';

const BASE = (__ENV.TEST_URL || 'http://localhost:1241').replace(/\/+$/, '');
const TOKEN = __ENV.CRM_AUTH_TOKEN || '';
const VUS = parseInt(__ENV.VUS || '5', 10);
const DURATION = __ENV.DURATION || '6h';
const RAMP = __ENV.RAMP || '5m';

const healthFailures = new Counter('health_failures');

export const options = {
  scenarios: {
    workload: {
      executor: 'ramping-vus',
      exec: 'workload',
      startVUs: 0,
      stages: [
        { duration: RAMP, target: VUS },
        { duration: DURATION, target: VUS },
        { duration: RAMP, target: 0 },
      ],
      gracefulRampDown: '30s',
    },
    health: {
      executor: 'constant-arrival-rate',
      exec: 'health',
      rate: 2,
      timeUnit: '1m',
      duration: DURATION,
      startTime: RAMP,
      preAllocatedVUs: 1,
    },
  },
  thresholds: {
    http_req_failed: ['rate<0.01'],
    'http_req_duration{scenario:workload}': ['p(95)<1000', 'p(99)<3000'],
    // The early series has no real limit; the threshold makes k6 report it
    // next to the late one, so drift over the run shows in the summary.
    'http_req_duration{phase:early}': ['p(95)>=0'],
    'http_req_duration{phase:late}': ['p(95)<1000'],
    checks: ['rate>0.99'],
    health_failures: ['count==0'],
  },
  summaryTrendStats: ['avg', 'med', 'p(90)', 'p(95)', 'p(99)', 'max'],
};

function headers() {
  const h = { 'Content-Type': 'application/json' };
  if (TOKEN) h.Authorization = `Bearer ${TOKEN}`;
  return h;
}

// "early" for the first tenth of the workload, "late" for the last tenth.
function phase() {
  const p = exec.scenario.progress;
  if (p < 0.1) return 'early';
  if (p > 0.9) return 'late';
  return 'middle';
}

function params(name) {
  return { headers: headers(), tags: { name, phase: phase() } };
}

function uuid() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    return (c === 'x' ? r : (r & 0x3) | 0x8).toString(16);
  });
}

function pick(list) {
  return list[Math.floor(Math.random() * list.length)];
}

// Contacts this VU created, so updates and activities target real rows.
const myContacts = [];
const MAX_TRACKED = 200;

const READS = [
  '/api/contacts?limit=50',
  '/api/accounts?limit=50',
  '/api/leads?limit=50',
  '/api/deals?limit=50',
  '/api/activities?limit=50',
  '/api/tickets?limit=50',
];

function read() {
  const path = pick(READS);
  const res = http.get(`${BASE}${path}`, params(path.split('?')[0]));
  check(res, { 'read 200': (r) => r.status === 200 });
}

function createContact() {
  const n = uuid().slice(0, 8);
  const body = {
    person_ref: `person:${uuid()}`,
    display_name: `Stability ${n}`,
    given_name: 'Stability',
    family_name: n,
    job_title: 'Synthetic test contact',
  };
  const res = http.post(`${BASE}/api/contacts`, JSON.stringify(body), params('/api/contacts POST'));
  const ok = check(res, { 'create contact 200': (r) => r.status === 200 });
  if (ok) {
    const pid = res.json('pid');
    if (pid) {
      myContacts.push(pid);
      if (myContacts.length > MAX_TRACKED) myContacts.shift();
    }
  }
}

function updateContact() {
  if (myContacts.length === 0) return createContact();
  const pid = pick(myContacts);
  const body = { job_title: `Updated ${new Date().toISOString()}` };
  const res = http.put(`${BASE}/api/contacts/${pid}`, JSON.stringify(body), params('/api/contacts/{pid} PUT'));
  check(res, { 'update contact 200': (r) => r.status === 200 });
}

function readContact() {
  if (myContacts.length === 0) return read();
  const res = http.get(`${BASE}/api/contacts/${pick(myContacts)}`, params('/api/contacts/{pid}'));
  check(res, { 'get contact 200': (r) => r.status === 200 });
}

function createActivity() {
  if (myContacts.length === 0) return createContact();
  const body = {
    subject_kind: 'contact',
    subject_pid: pick(myContacts),
    kind: pick(['call', 'email', 'meeting', 'note']),
    summary: 'Synthetic stability-test activity',
  };
  const res = http.post(`${BASE}/api/activities`, JSON.stringify(body), params('/api/activities POST'));
  check(res, { 'create activity 200': (r) => r.status === 200 });
}

// 75% list reads, 10% single reads, 6% creates, 5% updates, 4% activities.
export function workload() {
  const roll = Math.random() * 100;
  if (roll < 75) read();
  else if (roll < 85) readContact();
  else if (roll < 91) createContact();
  else if (roll < 96) updateContact();
  else createActivity();
  sleep(1 + Math.random() * 2);
}

export function health() {
  for (const path of ['/_health/live', '/_health/ready']) {
    const res = http.get(`${BASE}${path}`, { tags: { name: path } });
    if (!check(res, { [`${path} 200`]: (r) => r.status === 200 })) {
      healthFailures.add(1);
    }
  }
}

export function setup() {
  const res = http.get(`${BASE}/_health/ready`);
  if (res.status !== 200) {
    exec.test.abort(`service not ready at ${BASE}/_health/ready (status ${res.status})`);
  }
}
