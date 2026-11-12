/**
 * Stress Test
 * 
 * Purpose: Find breaking points and performance limits
 * Duration: 15-30 minutes
 * VUs: 100-500
 * 
 * This test gradually increases load to find:
 * - Maximum throughput
 * - Breaking points (errors, timeouts)
 * - Resource exhaustion (CPU, memory, database connections)
 * - Degradation patterns
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Trend, Rate, Counter } from 'k6/metrics';

// Import configuration and utilities
import { dev } from '../config/dev.js';
import { withAuth, randomInt, randomUUID } from '../lib/utils.js';
import { generateContact, generateDeal, generateLead, generateActivity } from '../lib/generators.js';

// Custom metrics
const responseTimes = new Trend('response_times');
const errorRate = new Rate('error_rate');
const successRate = new Rate('success_rate');
const requests = new Counter('requests');
const contactsCreated = new Counter('contacts_created');
const dealsCreated = new Counter('deals_created');
const leadsCreated = new Counter('leads_created');
const activitiesCreated = new Counter('activities_created');

// Get configuration
const config = dev;
const baseUrl = config.baseUrl;

// Test setup with aggressive ramp-up
export const options = {
  vus: config.vus.stressMax,
  duration: config.durations.medium,
  stages: config.stages.stress,
  thresholds: {
    ...config.thresholds,
    // Stress test thresholds are more lenient
    http_req_failed: ['rate<0.1'], // Allow up to 10% errors under stress
    http_req_duration: ['p(95)<5000'], // P95 < 5s under stress
    requests: ['count>5000'],
    errors: ['count<500'], // Less than 500 errors
  },
};

// Track created resources for cleanup
const createdResources = {
  contacts: [],
  deals: [],
  leads: [],
  activities: [],
};

/**
 * Create a contact with heavy data
 */
function createHeavyContact() {
  const contact = generateContact({
    tags: Array.from({ length: 5 }, () => `tag-${randomUUID()}`),
  });
  
  const url = `${baseUrl}/api/contacts`;
  const res = http.post(url, JSON.stringify(contact), {
    headers: { 'Content-Type': 'application/json' },
    ...withAuth({}, config.auth.token),
  });
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  
  if (res.status === 200) {
    const body = JSON.parse(res.body);
    createdResources.contacts.push(body.pid);
    contactsCreated.add(1);
  }
  
  check(res, {
    'Heavy contact creation successful': (r) => r.status === 200,
  });
  
  return res;
}

/**
 * Create a deal with heavy data
 */
function createHeavyDeal() {
  const deal = generateDeal({
    amount: randomInt(100000, 10000000),
    tags: Array.from({ length: 5 }, () => `tag-${randomUUID()}`),
  });
  
  const url = `${baseUrl}/api/deals`;
  const res = http.post(url, JSON.stringify(deal), {
    headers: { 'Content-Type': 'application/json' },
    ...withAuth({}, config.auth.token),
  });
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  
  if (res.status === 200) {
    const body = JSON.parse(res.body);
    createdResources.deals.push(body.pid);
    dealsCreated.add(1);
  }
  
  check(res, {
    'Heavy deal creation successful': (r) => r.status === 200,
  });
  
  return res;
}

/**
 * Create a lead with heavy data
 */
function createHeavyLead() {
  const lead = generateLead({
    tags: Array.from({ length: 5 }, () => `tag-${randomUUID()}`),
  });
  
  const url = `${baseUrl}/api/leads`;
  const res = http.post(url, JSON.stringify(lead), {
    headers: { 'Content-Type': 'application/json' },
    ...withAuth({}, config.auth.token),
  });
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  
  if (res.status === 200) {
    const body = JSON.parse(res.body);
    createdResources.leads.push(body.pid);
    leadsCreated.add(1);
  }
  
  check(res, {
    'Heavy lead creation successful': (r) => r.status === 200,
  });
  
  return res;
}

/**
 * Create an activity with heavy data
 */
function createHeavyActivity() {
  const activity = generateActivity({
    summary: randomUUID(),
    description: Array.from({ length: 10 }, () => randomUUID()).join(' '),
    tags: Array.from({ length: 5 }, () => `tag-${randomUUID()}`),
  });
  
  const url = `${baseUrl}/api/activities`;
  const res = http.post(url, JSON.stringify(activity), {
    headers: { 'Content-Type': 'application/json' },
    ...withAuth({}, config.auth.token),
  });
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  
  if (res.status === 200) {
    const body = JSON.parse(res.body);
    createdResources.activities.push(body.pid);
    activitiesCreated.add(1);
  }
  
  check(res, {
    'Heavy activity creation successful': (r) => r.status === 200,
  });
  
  return res;
}

/**
 * List all contacts (no pagination limit)
 */
function listAllContacts() {
  const url = `${baseUrl}/api/contacts?limit=1000`;
  const res = http.get(url, withAuth({}, config.auth.token));
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  
  check(res, {
    'List all contacts successful': (r) => r.status === 200,
  });
  
  return res;
}

/**
 * List all deals (no pagination limit)
 */
function listAllDeals() {
  const url = `${baseUrl}/api/deals?limit=1000`;
  const res = http.get(url, withAuth({}, config.auth.token));
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  
  check(res, {
    'List all deals successful': (r) => r.status === 200,
  });
  
  return res;
}

/**
 * Complex search with multiple filters
 */
function complexSearch() {
  const url = `${baseUrl}/api/contacts?limit=100&status=active&tag=${randomUUID()}`;
  const res = http.get(url, withAuth({}, config.auth.token));
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  
  check(res, {
    'Complex search successful': (r) => r.status === 200,
  });
  
  return res;
}

/**
 * Get dashboard/insights
 */
function getInsights() {
  const endpoints = [
    '/api/insights/win-rate',
    '/api/insights/pipeline',
    '/api/insights/forecast',
    '/api/insights/sla',
  ];
  
  const endpoint = endpoints[randomInt(0, endpoints.length)];
  const url = `${baseUrl}${endpoint}`;
  const res = http.get(url, withAuth({}, config.auth.token));
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  
  check(res, {
    [`Insight endpoint ${endpoint} successful`]: (r) => r.status === 200,
  });
  
  return res;
}

/**
 * Get timeline for a resource
 */
function getTimeline() {
  // Use a random resource type
  const resourceTypes = ['contacts', 'deals', 'leads'];
  const resourceType = resourceTypes[randomInt(0, resourceTypes.length)];
  const pid = randomUUID();
  
  const url = `${baseUrl}/api/${resourceType}/${pid}/timeline`;
  const res = http.get(url, withAuth({}, config.auth.token));
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200 || res.status === 404);
  requests.add(1);
  
  check(res, {
    `Timeline for ${resourceType} successful`: (r) => r.status === 200 || r.status === 404,
  });
  
  return res;
}

/**
 * Main test function
 */
export default function () {
  // Weighted random selection of operations
  // Heavier weight on read operations
  const operation = randomInt(0, 20);
  
  switch (operation) {
    case 0:
    case 1:
      // 10% chance: Create heavy contact
      createHeavyContact();
      break;
    case 2:
    case 3:
      // 10% chance: Create heavy deal
      createHeavyDeal();
      break;
    case 4:
    case 5:
      // 10% chance: Create heavy lead
      createHeavyLead();
      break;
    case 6:
    case 7:
      // 10% chance: Create heavy activity
      createHeavyActivity();
      break;
    case 8:
    case 9:
    case 10:
      // 15% chance: List all contacts
      listAllContacts();
      break;
    case 11:
    case 12:
      // 10% chance: List all deals
      listAllDeals();
      break;
    case 13:
    case 14:
      // 10% chance: Complex search
      complexSearch();
      break;
    case 15:
    case 16:
      // 10% chance: Get insights
      getInsights();
      break;
    case 17:
    case 18:
    case 19:
      // 15% chance: Get timeline
      getTimeline();
      break;
  }
  
  // Very short sleep under stress
  sleep(randomInt(0, 2));
}

/**
 * Setup function - runs once before all VUs start
 */
export function setup() {
  console.log(`Starting stress test against ${baseUrl}`);
  console.log(`VUs: Up to ${config.vus.stressMax}, Duration: ${config.durations.medium}`);
  console.log(`Authentication required: ${config.auth.requireAuth}`);
  return {};
}

/**
 * Teardown function - runs once after all VUs finish
 */
export function teardown(data) {
  console.log('Stress test completed');
  console.log('=== Results ===');
  console.log(`Total requests: ${requests.count}`);
  console.log(`Success rate: ${successRate.count / (successRate.count + errorRate.count) * 100}%`);
  console.log(`Error rate: ${errorRate.count / (successRate.count + errorRate.count) * 100}%`);
  console.log(`Average response time: ${responseTimes.avg}ms`);
  console.log(`P50 response time: ${responseTimes.percentile(50)}ms`);
  console.log(`P90 response time: ${responseTimes.percentile(90)}ms`);
  console.log(`P95 response time: ${responseTimes.percentile(95)}ms`);
  console.log(`P99 response time: ${responseTimes.percentile(99)}ms`);
  console.log('');
  console.log('=== Created Resources ===');
  console.log(`Contacts: ${contactsCreated.count}`);
  console.log(`Deals: ${dealsCreated.count}`);
  console.log(`Leads: ${leadsCreated.count}`);
  console.log(`Activities: ${activitiesCreated.count}`);
  
  // Note: In a real stress test, you might want to clean up created resources
  // For now, we just log them
  if (config.testData.cleanup) {
    console.log('Cleanup enabled - resources would be deleted');
  } else {
    console.log('Cleanup disabled - resources remain in database');
  }
}
