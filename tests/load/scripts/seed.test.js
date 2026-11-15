/**
 * Seed (k6): fills a throwaway database with a synthetic book of business before the other
 * tests, so lists, search, reports and rollups run against a volume of rows, not an empty
 * table. Each iteration makes one contact with an activity and tags, and, in proportion, an
 * account, a lead, a deal and a ticket. It is a setup step with loose limits, not a test of speed.
 *
 * Environment: TEST_URL, CRM_AUTH_TOKEN, and
 *   SEED  contacts to make            default 1000
 *   VUS   users making them at once   default 5
 */

import {
  createAccount,
  createActivity,
  createContact,
  createDeal,
  createLead,
  createTicket,
  envInt,
  moveDeal,
  moveTicket,
  prepare,
  tagContact,
} from '../lib/crm.js';

export const options = {
  scenarios: {
    seed: {
      executor: 'shared-iterations',
      vus: envInt('VUS', 5),
      iterations: envInt('SEED', 1000),
      maxDuration: '2h',
    },
  },
  thresholds: { http_req_failed: ['rate<0.01'], checks: ['rate>0.99'] },
};

export function setup() {
  return prepare();
}

export default function (data) {
  const contact = createContact();
  if (!contact) return;
  createActivity(contact);
  tagContact(contact);
  const roll = Math.random();
  if (roll < 0.3) createAccount();
  if (roll < 0.5) createLead();
  if (roll < 0.4) {
    const deal = createDeal(data.pipeline, contact);
    if (deal && Math.random() < 0.5) moveDeal(deal, data.stages[1]);
  }
  if (roll < 0.25) {
    const ticket = createTicket(contact);
    if (ticket && Math.random() < 0.5) moveTicket(ticket, 'resolved');
  }
}
