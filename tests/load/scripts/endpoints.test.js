/**
 * Endpoint benchmark (k6): each group of routes on its own, one after another, at a constant
 * arrival rate, so each group's latency is measured without the others' load mixed in. The
 * result is the table it prints and writes to results/endpoints.md: p50, p95, p99 and the
 * failure rate per group, with each group's limit. Seed first (`make seed`), or the lists and
 * rollups run over nearly empty tables.
 *
 * Environment: TEST_URL, CRM_AUTH_TOKEN, and
 *   RATE      requests per second in each group   default 20
 *   DURATION  length of each group                default 30s
 */

import exec from 'k6/execution';
import {
  ANALYTICS,
  COMMON_THRESHOLDS,
  analytics,
  createAccount,
  createActivity,
  createContact,
  createLead,
  createTicket,
  duplicates,
  envInt,
  funnel,
  getContact,
  list,
  moveTicket,
  prepare,
  previewLeadImport,
  previewSegment,
  runReport,
  search,
  updateContact,
  verifyAudits,
} from '../lib/crm.js';

const RATE = envInt('RATE', 20);
const STEP = __ENV.DURATION || '30s';

/** Group name, the function each request runs, and its p95 limit in milliseconds. */
const GROUPS = [
  ['lists', 'lists', 500],
  ['search', 'searches', 500],
  ['contact_reads', 'contactReads', 300],
  ['contact_writes', 'contactWrites', 500],
  ['other_writes', 'otherWrites', 500],
  ['analytics', 'rollups', 1000],
  ['reports', 'reports', 1000],
  ['segments', 'segments', 1000],
  ['duplicates', 'duplicateChecks', 2000],
  ['imports', 'imports', 1000],
  ['audit_verify', 'auditVerify', 2000],
];

function seconds(text) {
  const m = /^(\d+)(s|m|h)$/.exec(text);
  if (!m) throw new Error(`DURATION must look like 30s, 5m or 1h, not ${text}`);
  return Number(m[1]) * { s: 1, m: 60, h: 3600 }[m[2]];
}

const stepSeconds = seconds(STEP);
const scenarios = {};
const thresholds = { ...COMMON_THRESHOLDS };
GROUPS.forEach(([name, fn, p95], i) => {
  scenarios[name] = {
    executor: 'constant-arrival-rate',
    exec: fn,
    rate: RATE,
    timeUnit: '1s',
    duration: STEP,
    preAllocatedVUs: Math.max(5, RATE),
    maxVUs: RATE * 10,
    // A gap of 5 seconds between groups lets one group's slow tail finish before the next.
    startTime: `${i * (stepSeconds + 5)}s`,
    gracefulStop: '5s',
  };
  thresholds[`http_req_duration{scenario:${name}}`] = [`p(95)<${p95}`];
  thresholds[`http_req_failed{scenario:${name}}`] = ['rate<0.01'];
});

export const options = {
  scenarios,
  thresholds,
  summaryTrendStats: ['avg', 'med', 'p(95)', 'p(99)', 'max'],
};

/** Contacts to read, update and check for duplicates; made once in setup, shared by all. */
export function setup() {
  const data = prepare();
  data.contacts = [];
  for (let i = 0; i < 20; i += 1) {
    const pid = createContact();
    if (pid) data.contacts.push(pid);
  }
  if (data.contacts.length === 0) exec.test.abort('could not create the contacts to read');
  return data;
}

function any(listOf) {
  return listOf[exec.scenario.iterationInTest % listOf.length];
}

export function lists() {
  list();
}
export function searches() {
  search(any(['Load', 'Load deal', 'Synthetic', 'account', 'ticket']));
}
export function contactReads(data) {
  getContact(any(data.contacts));
}
export function contactWrites(data) {
  if (exec.scenario.iterationInTest % 2 === 0) createContact();
  else updateContact(any(data.contacts));
}
export function otherWrites(data) {
  const which = exec.scenario.iterationInTest % 4;
  if (which === 0) createActivity(any(data.contacts));
  else if (which === 1) createLead();
  else if (which === 2) createAccount();
  else {
    const ticket = createTicket(any(data.contacts));
    if (ticket) moveTicket(ticket, 'pending');
  }
}
export function rollups(data) {
  const i = exec.scenario.iterationInTest % (ANALYTICS.length + 1);
  if (i === ANALYTICS.length) funnel(data.pipeline);
  else analytics(ANALYTICS[i]);
}
export function reports() {
  runReport();
}
export function segments(data) {
  previewSegment(data.segment);
}
export function duplicateChecks(data) {
  duplicates(any(data.contacts));
}
export function imports() {
  previewLeadImport(200);
}
export function auditVerify() {
  verifyAudits();
}

function ms(value) {
  return value === undefined ? '' : value.toFixed(1);
}

export function handleSummary(summary) {
  const rows = GROUPS.map(([name, , limit]) => {
    const d = summary.metrics[`http_req_duration{scenario:${name}}`];
    const f = summary.metrics[`http_req_failed{scenario:${name}}`];
    const v = (d && d.values) || {};
    const failed = f && f.values ? (f.values.rate * 100).toFixed(2) : '';
    const ok = d && d.thresholds && Object.values(d.thresholds).every((t) => t.ok);
    return `| ${name} | ${ms(v.med)} | ${ms(v['p(95)'])} | ${ms(v['p(99)'])} | ${ms(v.max)} | ${failed}% | < ${limit} | ${ok ? 'pass' : 'FAIL'} |`;
  });
  const reqs = summary.metrics.http_reqs ? summary.metrics.http_reqs.values.count : 0;
  const table = [
    `Endpoint groups at ${RATE} requests a second for ${STEP} each (${reqs} requests).`,
    '',
    '| Group | p50 ms | p95 ms | p99 ms | max ms | failed | p95 limit ms | |',
    '| --- | --- | --- | --- | --- | --- | --- | --- |',
    ...rows,
    '',
  ].join('\n');
  return {
    stdout: `\n${table}\n`,
    'results/endpoints.md': table,
  };
}
