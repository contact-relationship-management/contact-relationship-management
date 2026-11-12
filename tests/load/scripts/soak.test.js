/**
 * Soak Test (Long-Running Test)
 * 
 * Purpose: Test system stability over extended periods
 * Duration: 1-24 hours
 * VUs: 20-100
 * 
 * This test verifies:
 * - Memory leaks
 * - Connection pool exhaustion
 * - Database connection stability
 * - Cache performance over time
 * - Application stability under sustained load
 * - Background job performance (nurture, SLA sweeps)
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Trend, Rate, Counter } from 'k6/metrics';

// Import configuration and utilities
import { dev } from '../config/dev.js';
import { withAuth, randomInt, randomUUID } from '../lib/utils.js';
import { generateContact, generateDeal, generateLead, generateActivity, generateCampaign, generateTicket } from '../lib/generators.js';

// Custom metrics
const responseTimes = new Trend('response_times');
const errorRate = new Rate('error_rate');
const successRate = new Rate('success_rate');
const requests = new Counter('requests');
const contactsCreated = new Counter('contacts_created');
const contactsUpdated = new Counter('contacts_updated');
const dealsCreated = new Counter('deals_created');
const leadsCreated = new Counter('leads_created');
const activitiesCreated = new Counter('activities_created');
const campaignsCreated = new Counter('campaigns_created');
const ticketsCreated = new Counter('tickets_created');

// Get configuration
const config = dev;
const baseUrl = config.baseUrl;

// Test setup for soak test
export const options = {
  vus: config.vus.soak,
  duration: config.durations.soak,
  stages: config.stages.soak,
  thresholds: {
    ...config.thresholds,
    // Soak test thresholds - should be stable over time
    http_req_failed: ['rate<0.01'],
    http_req_duration: ['p(95)<1000'],
    http_req_duration: ['avg<200'],
    requests: ['count>10000'],
    // Response time should not degrade significantly
    http_req_duration: ['p(95)<1500'], // Allow some degradation but not much
  },
};

// Track state across iterations
let iterationCount = 0;
let totalCreated = { contacts: 0, deals: 0, leads: 0, activities: 0, campaigns: 0, tickets: 0 };

/**
 * Create resources with realistic patterns
 */
function createResources() {
  // Create a contact with a deal and activity
  const contact = generateContact();
  const createContactRes = http.post(`${baseUrl}/api/contacts`, JSON.stringify(contact), {
    headers: { 'Content-Type': 'application/json' },
    ...withAuth({}, config.auth.token),
  });
  
  responseTimes.add(createContactRes.timings.duration);
  errorRate.add(createContactRes.status >= 400);
  successRate.add(createContactRes.status === 200);
  requests.add(1);
  
  if (createContactRes.status === 200) {
    const contactBody = JSON.parse(createContactRes.body);
    totalCreated.contacts++;
    contactsCreated.add(1);
    
    // Create a deal for this contact
    const deal = generateDeal({
      contact_ref: contact.person_ref,
    });
    const createDealRes = http.post(`${baseUrl}/api/deals`, JSON.stringify(deal), {
      headers: { 'Content-Type': 'application/json' },
      ...withAuth({}, config.auth.token),
    });
    
    responseTimes.add(createDealRes.timings.duration);
    errorRate.add(createDealRes.status >= 400);
    successRate.add(createDealRes.status === 200);
    requests.add(1);
    
    if (createDealRes.status === 200) {
      totalCreated.deals++;
      dealsCreated.add(1);
    }
    
    // Create an activity for this contact
    const activity = generateActivity({
      subject_kind: 'contact',
      subject_pid: contactBody.pid,
    });
    const createActivityRes = http.post(`${baseUrl}/api/activities`, JSON.stringify(activity), {
      headers: { 'Content-Type': 'application/json' },
      ...withAuth({}, config.auth.token),
    });
    
    responseTimes.add(createActivityRes.timings.duration);
    errorRate.add(createActivityRes.status >= 400);
    successRate.add(createActivityRes.status === 200);
    requests.add(1);
    
    if (createActivityRes.status === 200) {
      totalCreated.activities++;
      activitiesCreated.add(1);
    }
  }
  
  return createContactRes;
}

/**
 * Update existing resources
 */
function updateResources() {
  // List contacts and update a random one
  const listRes = http.get(`${baseUrl}/api/contacts?limit=100`, withAuth({}, config.auth.token));
  
  responseTimes.add(listRes.timings.duration);
  errorRate.add(listRes.status >= 400);
  successRate.add(listRes.status === 200);
  requests.add(1);
  
  if (listRes.status === 200) {
    const body = JSON.parse(listRes.body);
    if (body.items && body.items.length > 0) {
      const contact = body.items[randomInt(0, body.items.length)];
      const updateUrl = `${baseUrl}/api/contacts/${contact.pid}`;
      
      const updateRes = http.patch(updateUrl, JSON.stringify({
        tags: [...(contact.tags || []), `updated-${Date.now()}`],
      }), {
        headers: { 'Content-Type': 'application/json' },
        ...withAuth({}, config.auth.token),
      });
      
      responseTimes.add(updateRes.timings.duration);
      errorRate.add(updateRes.status >= 400);
      successRate.add(updateRes.status === 200);
      requests.add(1);
      contactsUpdated.add(1);
    }
  }
  
  return listRes;
}

/**
 * Search operations
 */
function searchOperations() {
  const searchTypes = ['contacts', 'deals', 'leads'];
  const searchType = searchTypes[randomInt(0, searchTypes.length)];
  
  const url = `${baseUrl}/api/${searchType}?limit=50&status=active`;
  const res = http.get(url, withAuth({}, config.auth.token));
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  
  check(res, {
    [`Search ${searchType} successful`]: (r) => r.status === 200,
  });
  
  return res;
}

/**
 * Dashboard and insights operations
 */
function dashboardOperations() {
  const dashboards = [
    '/api/insights/win-rate',
    '/api/insights/pipeline',
    '/api/insights/forecast',
    '/api/insights/sla',
    '/api/insights/activity-feed',
  ];
  
  const dashboard = dashboards[randomInt(0, dashboards.length)];
  const url = `${baseUrl}${dashboard}`;
  const res = http.get(url, withAuth({}, config.auth.token));
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  
  check(res, {
    [`Dashboard ${dashboard} successful`]: (r) => r.status === 200,
  });
  
  return res;
}

/**
 * Background job operations
 */
function backgroundJobOperations() {
  // Trigger nurture advance
  const nurtureRes = http.post(`${baseUrl}/api/nurture/advance`, {}, withAuth({}, config.auth.token));
  
  responseTimes.add(nurtureRes.timings.duration);
  errorRate.add(nurtureRes.status >= 400);
  successRate.add(nurtureRes.status === 200 || nurtureRes.status === 202);
  requests.add(1);
  
  check(nurtureRes, {
    'Nurture advance successful': (r) => r.status === 200 || r.status === 202,
  });
  
  // Trigger SLA breach sweep
  const slaRes = http.post(`${baseUrl}/api/sla/breaches/sweep`, {}, withAuth({}, config.auth.token));
  
  responseTimes.add(slaRes.timings.duration);
  errorRate.add(slaRes.status >= 400);
  successRate.add(slaRes.status === 200 || slaRes.status === 202);
  requests.add(1);
  
  check(slaRes, {
    'SLA breach sweep successful': (r) => r.status === 200 || r.status === 202,
  });
  
  return { nurtureRes, slaRes };
}

/**
 * Health check operations
 */
function healthChecks() {
  const endpoints = [
    '/_health/live',
    '/_health/ready',
    '/_health/startup',
    '/metrics.prom',
  ];
  
  const endpoint = endpoints[randomInt(0, endpoints.length)];
  const url = `${baseUrl}${endpoint}`;
  const res = http.get(url, withAuth({}, config.auth.token));
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  
  check(res, {
    [`Health check ${endpoint} successful`]: (r) => r.status === 200,
  });
  
  return res;
}

/**
 * Main test function
 */
export default function () {
  iterationCount++;
  
  // Log progress every 100 iterations
  if (iterationCount % 100 === 0) {
    console.log(`Iteration ${iterationCount}: Created ${JSON.stringify(totalCreated)} resources`);
  }
  
  // Weighted random selection of operations
  // Mix of create, read, update, delete operations
  const operation = randomInt(0, 30);
  
  switch (operation) {
    case 0:
    case 1:
    case 2:
      // 10% chance: Create resources
      createResources();
      break;
    case 3:
    case 4:
      // 6.7% chance: Update resources
      updateResources();
      break;
    case 5:
    case 6:
    case 7:
    case 8:
      // 13.3% chance: Search operations
      searchOperations();
      break;
    case 9:
    case 10:
    case 11:
      // 10% chance: Dashboard operations
      dashboardOperations();
      break;
    case 12:
    case 13:
      // 6.7% chance: Background job operations
      backgroundJobOperations();
      break;
    case 14:
    case 15:
    case 16:
    case 17:
    case 18:
    case 19:
      // 20% chance: Health checks
      healthChecks();
      break;
    default:
      // Remaining 33.3%: List operations with random limits
      const listTypes = ['contacts', 'deals', 'leads', 'activities'];
      const listType = listTypes[randomInt(0, listTypes.length)];
      const limit = randomInt(10, 100);
      
      const listRes = http.get(`${baseUrl}/api/${listType}?limit=${limit}`, withAuth({}, config.auth.token));
      
      responseTimes.add(listRes.timings.duration);
      errorRate.add(listRes.status >= 400);
      successRate.add(listRes.status === 200);
      requests.add(1);
      
      check(listRes, {
        [`List ${listType} successful`]: (r) => r.status === 200,
      });
  }
  
  // Variable sleep to simulate realistic user behavior
  // Longer sleep for soak tests to reduce load but maintain activity
  sleep(randomInt(1, 5));
}

/**
 * Setup function - runs once before all VUs start
 */
export function setup() {
  console.log(`Starting soak test against ${baseUrl}`);
  console.log(`VUs: ${config.vus.soak}, Duration: ${config.durations.soak}`);
  console.log(`Authentication required: ${config.auth.requireAuth}`);
  console.log('This is a long-running test - expect it to run for the full duration');
  return {};
}

/**
 * Teardown function - runs once after all VUs finish
 */
export function teardown(data) {
  console.log('Soak test completed');
  console.log('=== Final Results ===');
  console.log(`Total iterations: ${iterationCount}`);
  console.log(`Total requests: ${requests.count}`);
  console.log(`Success rate: ${successRate.count / (successRate.count + errorRate.count) * 100}%`);
  console.log(`Error rate: ${errorRate.count / (successRate.count + errorRate.count) * 100}%`);
  console.log('');
  console.log('=== Performance Metrics ===');
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
  console.log(`Campaigns: ${campaignsCreated.count}`);
  console.log(`Tickets: ${ticketsCreated.count}`);
  console.log('');
  console.log('=== Updated Resources ===');
  console.log(`Contacts updated: ${contactsUpdated.count}`);
  
  // Analyze degradation over time
  // This would require tracking metrics per iteration in a real implementation
  console.log('');
  console.log('=== Stability Analysis ===');
  console.log('Note: For degradation analysis, use time-series output (InfluxDB, Prometheus)');
  console.log('and visualize response times, error rates, and throughput over the test duration.');
}
