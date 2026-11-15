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

export function createAccount() {
  const res = post('/api/accounts', {
    organization_ref: `organization:${uuid()}`,
    display_name: `Load account ${uuid().slice(0, 8)}`,
    industry: pick(['Logistics', 'Retail', 'Software', 'Health']),
    tier: pick(['prospect', 'customer', 'partner']),
  });
  check(res, { 'create account 200': (r) => r.status === 200 });
  return pidOf(res);
}

export function getAccount(pid) {
  const res = http.get(`${BASE}/api/accounts/${pid}`, params('/api/accounts/{pid}'));
  check(res, { 'get account 200': (r) => r.status === 200 });
  return res;
}

export function getLead(pid) {
  const res = http.get(`${BASE}/api/leads/${pid}`, params('/api/leads/{pid}'));
  check(res, { 'get lead 200': (r) => r.status === 200 });
  return res;
}

export function createTicket(contactPid) {
  const body = {
    title: `Load ticket ${uuid().slice(0, 8)}`,
    priority: pick(['low', 'normal', 'high']),
  };
  if (contactPid) body.contact_pid = contactPid;
  const res = post('/api/tickets', body);
  check(res, { 'create ticket 200': (r) => r.status === 200 });
  return pidOf(res);
}

/** A ticket's status moves along its lifecycle: open, pending, open, resolved. */
export function moveTicket(pid, to) {
  const res = post(`/api/tickets/${pid}/status`, { to }, '/api/tickets/{pid}/status POST');
  check(res, { 'move ticket 200': (r) => r.status === 200 });
  return res;
}

/** Tags are set whole; each name is made on first use. */
export function tagContact(pid) {
  const res = http.put(
    `${BASE}/api/contacts/${pid}/tags`,
    JSON.stringify({ tags: [pick(['load-a', 'load-b', 'load-c']), 'load-test'] }),
    params('/api/contacts/{pid}/tags PUT'),
  );
  check(res, { 'tag contact 200': (r) => r.status === 200 });
  return res;
}

/** The duplicate check compares one contact with up to 5,000 others (CRM-T61). */
export function duplicates(pid) {
  const res = http.get(`${BASE}/api/contacts/${pid}/duplicates`, params('/api/contacts/{pid}/duplicates'));
  check(res, { 'duplicates 200': (r) => r.status === 200 });
  return res;
}

export function createSegment() {
  const res = post('/api/segments', {
    name: `Load segment ${uuid().slice(0, 8)}`,
    filter: { tags: ['load-test'] },
  });
  check(res, { 'create segment 200': (r) => r.status === 200 });
  return pidOf(res);
}

/** The contacts a segment holds now, through the consent gate. */
export function previewSegment(pid) {
  const res = http.get(`${BASE}/api/segments/${pid}/preview`, params('/api/segments/{pid}/preview'));
  check(res, { 'preview segment 200': (r) => r.status === 200 });
  return res;
}

const REPORTS = [
  { kind: 'contact', group_by: 'status' },
  { kind: 'lead', group_by: 'source' },
  { kind: 'deal', group_by: 'state', measure: 'sum_amount' },
  { kind: 'deal', group_by: 'state', measure: 'sum_amount', convert_to: 'GBP' },
  { kind: 'ticket', group_by: 'priority' },
];

/** An ad hoc report (CRM-R48); a sum converted to one currency needs its rates (CRM-R50). */
export function runReport(spec = pick(REPORTS)) {
  const res = post('/api/reports/run', spec, '/api/reports/run POST');
  check(res, { 'run report 200': (r) => r.status === 200 });
  return res;
}

const GIVEN = ['Imogen', 'Robert', 'Aisha', 'Wei', 'Katarzyna', 'Liam', 'Fatima', 'Jose', 'Priya',
  'Oluwaseun', 'Chloe', 'Mateo', 'Sven', 'Yuki', 'Amara', 'Diego'];
const FAMILY = ['Hartley', 'Okafor', 'Nguyen', 'Kowalski', 'Haddad', 'Fernandez', 'Singh', 'Muller',
  'Rossi', 'Tanaka', 'Adeyemi', 'Lindqvist', 'Moreau', 'Novak', 'Costa', 'Byrne'];

/**
 * A lead-import preview of `rows` synthetic rows (at most 500): read, mapped and judged, and each
 * row compared with the contacts, leads and earlier rows whose names share a word with it; nothing
 * is saved. The names are made-up pairs from short lists, so rows resemble one another about as a
 * real file's do; a file whose every name shares a word would be the slow case (CRM-T242).
 */
export function previewLeadImport(rows = 200) {
  const lines = ['Full Name,E-mail,Organisation'];
  for (let i = 0; i < rows; i += 1) {
    const given = pick(GIVEN);
    const family = pick(FAMILY);
    lines.push(`${given} ${family},${given.toLowerCase()}${i}@example.invalid,Org ${i % 20}`);
  }
  const res = post(
    '/api/lead-imports/preview',
    {
      csv: `${lines.join('\n')}\n`,
      mapping: { display_name: 'Full Name', email: 'E-mail', company: 'Organisation' },
    },
    '/api/lead-imports/preview POST',
  );
  check(res, { 'preview lead import 200': (r) => r.status === 200 });
  return res;
}

/** The read-only rollups: dashboards, insights, forecast, attribution and quota attainment. */
export const ANALYTICS = [
  '/api/forecast',
  '/api/forecast?convert_to=GBP',
  '/api/attribution',
  '/api/attribution?model=time_decay',
  '/api/dashboards/sales',
  '/api/dashboards/activity?days=30',
  '/api/dashboards/sla',
  '/api/insights/executive',
  '/api/insights/engagement?days=90',
  '/api/insights/stale-deals',
  '/api/insights/pipeline-hygiene?days=14',
  '/api/insights/forecast-trends',
  '/api/insights/followups',
  '/api/insights/sla',
  '/api/quotas/attainment',
];

/** The stage funnel of one pipeline (CRM-R44). */
export function funnel(pipelinePid) {
  const res = http.get(
    `${BASE}/api/insights/funnel?pipeline=${pipelinePid}`,
    params('/api/insights/funnel'),
  );
  check(res, { 'funnel 200': (r) => r.status === 200 });
  return res;
}

export function analytics(path = pick(ANALYTICS)) {
  const res = http.get(`${BASE}${path}`, params(path.split('?')[0]));
  check(res, { 'analytics 200': (r) => r.status === 200 });
  return res;
}

/** The audit chain checked end to end (CRM-R33); its cost grows with the audit log. */
export function verifyAudits() {
  const res = http.get(`${BASE}/api/audits/verify`, params('/api/audits/verify'));
  check(res, { 'verify audits 200': (r) => r.status === 200 });
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
  const segment = createSegment();
  if (!segment) exec.test.abort('could not create the load-test segment');
  // The two open stages, Qualify and Propose; deals move between them.
  return { pipeline: made.pid, stages: made.stages.slice(0, 2), segment };
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
 * The mixed workload: each entry is a share of iterations (out of 100) and what it does. Mostly
 * reads, as a CRM's use is; the writes touch this VU's own records.
 */
export const WORKLOAD = [
  [40, 'list', () => list()],
  [8, 'search', () => search()],
  [8, 'analytics', (data) => (Math.random() < 0.1 ? funnel(data.pipeline) : analytics())],
  [7, 'read contact', () => getContact(pick(mine))],
  [3, 'read account', (data, vu) => (vu.accounts.length ? getAccount(pick(vu.accounts)) : list())],
  [2, 'report', () => runReport()],
  [2, 'segment preview', (data) => previewSegment(data.segment)],
  [1, 'duplicates', () => duplicates(pick(mine))],
  [6, 'create contact', () => remember(createContact())],
  [4, 'update contact', () => updateContact(pick(mine))],
  [4, 'activity', () => createActivity(pick(mine))],
  [2, 'tag contact', () => tagContact(pick(mine))],
  [3, 'lead', () => createLead()],
  [2, 'account', (data, vu) => keep(vu.accounts, createAccount())],
  [
    3,
    'deal',
    (data) => {
      const deal = createDeal(data.pipeline, pick(mine));
      if (deal) moveDeal(deal, data.stages[1]);
    },
  ],
  [
    4,
    'ticket',
    () => {
      const ticket = createTicket(pick(mine));
      if (ticket) moveTicket(ticket, pick(['pending', 'resolved']));
    },
  ],
  [1, 'lead import preview', () => previewLeadImport(100)],
];

const TOTAL_WEIGHT = WORKLOAD.reduce((sum, [weight]) => sum + weight, 0);
const vuState = { accounts: [] };

function keep(listOf, pid) {
  if (!pid) return;
  listOf.push(pid);
  if (listOf.length > MAX_TRACKED) listOf.shift();
}

/**
 * One iteration of the mixed workload (`WORKLOAD`), then a pause of `thinkSeconds` (up to twice
 * that), so a VU is one person, not a loop. A VU's first iteration makes a contact, so the rest
 * have one to work on.
 */
export function mixed(data, thinkSeconds = 1) {
  if (mine.length === 0) {
    remember(createContact());
  } else {
    let roll = Math.random() * TOTAL_WEIGHT;
    for (const [weight, , run] of WORKLOAD) {
      roll -= weight;
      if (roll < 0) {
        run(data, vuState);
        break;
      }
    }
  }
  if (thinkSeconds > 0) sleep(thinkSeconds * (1 + Math.random()));
}
