/**
 * Basic Load Test
 * 
 * Purpose: Establish baseline performance metrics
 * Duration: 5-10 minutes
 * VUs: 10-50
 * 
 * This test establishes baseline performance for:
 * - Read operations (contacts, deals, leads)
 * - List operations with pagination
 * - Search operations
 * - Basic CRUD operations
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Trend, Rate, Counter } from 'k6/metrics';

// Import configuration and utilities
import { dev } from '../config/dev.js';
import { withAuth, randomInt } from '../lib/utils.js';

// Custom metrics
const responseTimes = new Trend('response_times');
const errorRate = new Rate('error_rate');
const successRate = new Rate('success_rate');
const requests = new Counter('requests');
const contactsCreated = new Counter('contacts_created');
const contactsRead = new Counter('contacts_read');
const contactsListed = new Counter('contacts_listed');

// Get configuration
const config = dev;
const baseUrl = config.baseUrl;

// Test setup
export const options = {
  vus: config.vus.max,
  duration: config.durations.short,
  stages: config.stages.normal,
  thresholds: {
    ...config.thresholds,
    // Load test specific thresholds
    http_req_failed: ['rate<0.01'],
    http_req_duration: ['p(95)<1000'],
    requests: ['count>1000'],
  },
};

/**
 * List contacts with pagination
 */
function listContacts(page = 1, limit = 20) {
  const url = `${baseUrl}/api/contacts?page=${page}&limit=${limit}`;
  const res = http.get(url, withAuth({}, config.auth.token));
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  contactsListed.add(1);
  
  check(res, {
    'List contacts successful': (r) => r.status === 200,
    'List contacts returns array': (r) => {
      const body = JSON.parse(r.body);
      return Array.isArray(body?.items);
    },
  });
  
  return res;
}

/**
 * Get a single contact by PID
 */
function getContact(pid) {
  const url = `${baseUrl}/api/contacts/${pid}`;
  const res = http.get(url, withAuth({}, config.auth.token));
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  contactsRead.add(1);
  
  check(res, {
    'Get contact successful': (r) => r.status === 200 || r.status === 404,
  });
  
  return res;
}

/**
 * Search contacts
 */
function searchContacts(query = '') {
  const url = `${baseUrl}/api/contacts?query=${query}&limit=10`;
  const res = http.get(url, withAuth({}, config.auth.token));
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  
  check(res, {
    'Search contacts successful': (r) => r.status === 200,
  });
  
  return res;
}

/**
 * List deals
 */
function listDeals() {
  const url = `${baseUrl}/api/deals?limit=20`;
  const res = http.get(url, withAuth({}, config.auth.token));
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  
  check(res, {
    'List deals successful': (r) => r.status === 200,
  });
  
  return res;
}

/**
 * List leads
 */
function listLeads() {
  const url = `${baseUrl}/api/leads?limit=20`;
  const res = http.get(url, withAuth({}, config.auth.token));
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  
  check(res, {
    'List leads successful': (r) => r.status === 200,
  });
  
  return res;
}

/**
 * Get forecast
 */
function getForecast() {
  const url = `${baseUrl}/api/forecast`;
  const res = http.get(url, withAuth({}, config.auth.token));
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  
  check(res, {
    'Get forecast successful': (r) => r.status === 200,
  });
  
  return res;
}

/**
 * Main test function
 */
export default function () {
  // Weighted random selection of operations
  const operation = randomInt(0, 10);
  
  switch (operation) {
    case 0:
    case 1:
    case 2:
    case 3:
      // 40% chance: List contacts
      listContacts(randomInt(1, 5), randomInt(10, 50));
      break;
    case 4:
    case 5:
      // 20% chance: Get single contact (use a known PID or random)
      getContact('00000000-0000-0000-0000-000000000000');
      break;
    case 6:
      // 10% chance: Search contacts
      searchContacts(`test${randomInt(0, 100)}`);
      break;
    case 7:
    case 8:
      // 20% chance: List deals
      listDeals();
      break;
    case 9:
      // 10% chance: List leads
      listLeads();
      break;
    case 10:
      // 10% chance: Get forecast
      getForecast();
      break;
  }
  
  // Random sleep to simulate think time
  sleep(randomInt(1, 5));
}

/**
 * Setup function - runs once before all VUs start
 */
export function setup() {
  console.log(`Starting basic load test against ${baseUrl}`);
  console.log(`VUs: ${config.vus.max}, Duration: ${config.durations.short}`);
  console.log(`Authentication required: ${config.auth.requireAuth}`);
  return {};
}

/**
 * Teardown function - runs once after all VUs finish
 */
export function teardown(data) {
  console.log('Basic load test completed');
  console.log(`Total requests: ${requests.count}`);
  console.log(`Success rate: ${successRate.count / (successRate.count + errorRate.count) * 100}%`);
  console.log(`Error rate: ${errorRate.count / (successRate.count + errorRate.count) * 100}%`);
  console.log(`Contacts listed: ${contactsListed.count}`);
  console.log(`Contacts read: ${contactsRead.count}`);
  console.log(`Average response time: ${responseTimes.avg}ms`);
  console.log(`P95 response time: ${responseTimes.percentile(95)}ms`);
  console.log(`P99 response time: ${responseTimes.percentile(99)}ms`);
}
