/**
 * Smoke test (k6): one virtual user makes each call once per iteration and every call must
 * succeed. Run it first; the other load tests assume what it checks.
 *
 * Environment: TEST_URL, CRM_AUTH_TOKEN (lib/crm.js), and
 *   ITERATIONS  iterations of the round   default 3
 */

import {
  COMMON_THRESHOLDS,
  createActivity,
  createContact,
  createDeal,
  createLead,
  envInt,
  getContact,
  list,
  moveDeal,
  prepare,
  search,
  updateContact,
} from '../lib/crm.js';

export const options = {
  scenarios: {
    smoke: { executor: 'per-vu-iterations', vus: 1, iterations: envInt('ITERATIONS', 3) },
  },
  thresholds: {
    ...COMMON_THRESHOLDS,
    http_req_failed: ['rate==0'],
    checks: ['rate==1'],
    http_req_duration: ['p(95)<1000'],
  },
};

export function setup() {
  return prepare();
}

export default function (data) {
  const contact = createContact();
  if (contact) {
    getContact(contact);
    updateContact(contact);
    createActivity(contact);
  }
  createLead();
  const deal = createDeal(data.pipeline, contact);
  if (deal) moveDeal(deal, data.stages[1]);
  for (const path of ['/api/contacts?limit=10', '/api/leads?limit=10', '/api/deals?limit=10']) {
    list(path);
  }
  search('Load');
}
