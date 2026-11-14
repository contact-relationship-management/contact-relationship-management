/**
 * The CRM API calls the load tests make, written against the service's real request bodies
 * (each refuses a field it does not know, so a wrong name is a 422, not a silent default).
 *
 * Every record is synthetic. A contact holds a made-up `person:<uuid>` reference and a
 * display name, never a copied name, email, phone or address (spec/scope.md). The API has
 * no contact delete (erasure is a privacy operation), so run these only against a
 * throwaway database, such as `podman compose up --build` from the repository root.
 *
 * Environment (k6 `-e NAME=value`; k6 has no `process.env`):
 *   TEST_URL        service base URL   default http://localhost:1241
 *   CRM_AUTH_TOKEN  bearer token       default none (the demo has sign-in off)
 */

import http from 'k6/http';
import exec from 'k6/execution';
import { check, sleep } from 'k6';

export const BASE = (__ENV.TEST_URL || 'http://localhost:1241').replace(/\/+$/, '');
const TOKEN = __ENV.CRM_AUTH_TOKEN || '';

/** A whole number from the environment, or the default. */
export function envInt(name, fallback) {
  const n = parseInt(__ENV[name] || '', 10);
  return Number.isFinite(n) && n > 0 ? n : fallback;
}

/** Thresholds every load test shares; a script adds its own latency limits. */
export const COMMON_THRESHOLDS = {
  http_req_failed: ['rate<0.01'],
  checks: ['rate>0.99'],
};

function params(name) {
  const headers = { 'Content-Type': 'application/json' };
  if (TOKEN) headers.Authorization = `Bearer ${TOKEN}`;
  return { headers, tags: { name } };
}

export function uuid() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    return (c === 'x' ? r : (r & 0x3) | 0x8).toString(16);
  });
}

export function pick(list) {
  return list[Math.floor(Math.random() * list.length)];
}

function post(path, body, name) {
  return http.post(`${BASE}${path}`, JSON.stringify(body), params(name || `${path} POST`));
}

function pidOf(res) {
  try {
    return res.json('pid') || null;
  } catch {
    return null;
  }
}

const LISTS = [
  '/api/contacts?limit=50',
  '/api/accounts?limit=50',
  '/api/leads?limit=50',
  '/api/deals?limit=50',
  '/api/activities?limit=50',
  '/api/tickets?limit=50',
];

export function list(path = pick(LISTS)) {
  const res = http.get(`${BASE}${path}`, params(path.split('?')[0]));
  check(res, { 'list 200': (r) => r.status === 200 });
  return res;
}

export function search(term = 'Load') {
  const res = http.get(`${BASE}/api/search?q=${encodeURIComponent(term)}`, params('/api/search'));
  check(res, { 'search 200': (r) => r.status === 200 });
  return res;
}

export function createContact() {
  const n = uuid().slice(0, 8);
  const res = post('/api/contacts', {
    person_ref: `person:${uuid()}`,
    display_name: `Load ${n}`,
    job_title: 'Synthetic load-test contact',
  });
  check(res, { 'create contact 200': (r) => r.status === 200 });
  return pidOf(res);
}

export function getContact(pid) {
  const res = http.get(`${BASE}/api/contacts/${pid}`, params('/api/contacts/{pid}'));
  check(res, { 'get contact 200': (r) => r.status === 200 });
  return res;
}

export function updateContact(pid) {
  const res = http.put(
    `${BASE}/api/contacts/${pid}`,
    JSON.stringify({ job_title: `Updated ${new Date().toISOString()}` }),
    params('/api/contacts/{pid} PUT'),
  );
  check(res, { 'update contact 200': (r) => r.status === 200 });
  return res;
}

export function createActivity(contactPid) {
  const res = post('/api/activities', {
    subject_kind: 'contact',
    subject_pid: contactPid,
    kind: pick(['call', 'email', 'meeting', 'note']),
    summary: 'Synthetic load-test activity',
  });
  check(res, { 'create activity 200': (r) => r.status === 200 });
  return pidOf(res);
}

export function createLead() {
  const res = post('/api/leads', {
    display_name: `Load lead ${uuid().slice(0, 8)}`,
    source: pick(['web', 'referral', 'event']),
  });
  check(res, { 'create lead 200': (r) => r.status === 200 });
  return pidOf(res);
}

/** One pipeline for the whole run; `setup()` makes it and passes its pid to the VUs. */
export function createPipeline() {
  const res = post('/api/pipelines', {
    name: `Load pipeline ${uuid().slice(0, 8)}`,
    stages: [
      { name: 'Qualify', probability_percent: 20 },
      { name: 'Propose', probability_percent: 60 },
      { name: 'Won', probability_percent: 100, is_won: true },
      { name: 'Lost', probability_percent: 0, is_lost: true },
    ],
  });
  check(res, { 'create pipeline 200': (r) => r.status === 200 });
  const pid = pidOf(res);
  let stages = [];
  try {
    stages = res.json('stage_pids') || [];
  } catch {
    stages = [];
  }
  return pid ? { pid, stages } : null;
}

export function createDeal(pipelinePid, contactPid) {
  const body = {
    name: `Load deal ${uuid().slice(0, 8)}`,
    pipeline_pid: pipelinePid,
    amount_minor: 1000 + Math.floor(Math.random() * 10_000_000),
    currency: pick(['USD', 'EUR', 'GBP']),
  };
  if (contactPid) body.primary_contact_pid = contactPid;
  const res = post('/api/deals', body);
  check(res, { 'create deal 200': (r) => r.status === 200 });
  return pidOf(res);
}

/** Moves a deal to another stage of its pipeline (there is no read of one deal by pid). */
export function moveDeal(pid, stagePid) {
  const res = post(`/api/deals/${pid}/stage`, { stage_pid: stagePid }, '/api/deals/{pid}/stage POST');
  check(res, { 'move deal 200': (r) => r.status === 200 });
  return res;
}

/** Stops the run before any load if the service is not ready, and makes the shared pipeline. */
export function prepare() {
  const res = http.get(`${BASE}/_health/ready`);
  if (res.status !== 200) {
    exec.test.abort(`service not ready at ${BASE}/_health/ready (status ${res.status})`);
  }
  const made = createPipeline();
  if (!made || made.stages.length < 2) exec.test.abort('could not create the load-test pipeline');
  // The two open stages, Qualify and Propose; deals move between them.
  return { pipeline: made.pid, stages: made.stages.slice(0, 2) };
}

// Contacts this VU made, so reads, updates, activities and deals target real rows.
const mine = [];
const MAX_TRACKED = 200;

function remember(pid) {
  if (!pid) return;
  mine.push(pid);
  if (mine.length > MAX_TRACKED) mine.shift();
}

/**
 * One iteration of the mixed workload: 60% list reads, 10% search, 10% contact reads,
 * 6% contact creates, 4% updates, 4% activities, 3% leads, 3% deals (each made and then
 * moved to the next stage). Then a pause of
 * `thinkSeconds` (up to twice that), so a VU is one person, not a loop. A VU's first
 * iteration makes a contact, so the rest have one to work on.
 */
export function mixed(data, thinkSeconds = 1) {
  const roll = Math.random() * 100;
  if (mine.length === 0) remember(createContact());
  else if (roll < 60) list();
  else if (roll < 70) search();
  else if (roll < 80) getContact(pick(mine));
  else if (roll < 86) remember(createContact());
  else if (roll < 90) updateContact(pick(mine));
  else if (roll < 94) createActivity(pick(mine));
  else if (roll < 97) createLead();
  else {
    const deal = createDeal(data.pipeline, pick(mine));
    if (deal) moveDeal(deal, data.stages[1]);
  }
  if (thinkSeconds > 0) sleep(thinkSeconds * (1 + Math.random()));
}
