/**
 * Smoke Test
 * 
 * Purpose: Verify basic functionality of the CRM service
 * Duration: ~1-2 minutes
 * VUs: 1-5
 * 
 * This test verifies that:
 * - The service is running and responsive
 * - Basic endpoints are working
 * - Authentication (if enabled) is working
 * - Core CRUD operations are functional
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Trend, Rate } from 'k6/metrics';

// Import configuration and utilities
import { dev } from '../config/dev.js';
import { withAuth, randomString, randomUUID } from '../lib/utils.js';
import { generateContact, generateDeal, generateLead } from '../lib/generators.js';

// Custom metrics
const responseTimes = new Trend('response_times');
const errorRate = new Rate('error_rate');
const successRate = new Rate('success_rate');

// Get configuration
const config = dev;
const baseUrl = config.baseUrl;

// Test setup
export const options = {
  vus: config.vus.smoke,
  duration: config.durations.smoke,
  thresholds: {
    ...config.thresholds,
    // Smoke test specific thresholds
    http_req_failed: ['rate<0.1'], // Allow up to 10% errors in smoke
    http_req_duration: ['p(95)<2000'], // P95 < 2s
  },
};

/**
 * Check health endpoints
 */
function testHealthEndpoints() {
  const endpoints = [
    '/_health/live',
    '/_health/ready',
    '/_health/startup',
  ];
  
  for (const endpoint of endpoints) {
    const url = `${baseUrl}${endpoint}`;
    const res = http.get(url, withAuth({}, config.auth.token));
    
    responseTimes.add(res.timings.duration);
    errorRate.add(res.status >= 400);
    successRate.add(res.status === 200);
    
    check(res, {
      [`Health endpoint ${endpoint} is reachable`]: (r) => r.status === 200,
    });
    
    sleep(0.5);
  }
}

/**
 * Test API documentation endpoints
 */
function testApiDocs() {
  const endpoints = [
    '/api-docs/openapi.json',
    '/swagger-ui',
  ];
  
  for (const endpoint of endpoints) {
    const url = `${baseUrl}${endpoint}`;
    const res = http.get(url, withAuth({}, config.auth.token));
    
    responseTimes.add(res.timings.duration);
    errorRate.add(res.status >= 400);
    successRate.add(res.status === 200 || res.status === 301 || res.status === 302);
    
    check(res, {
      [`API docs endpoint ${endpoint} is reachable`]: (r) => 
        r.status === 200 || r.status === 301 || r.status === 302,
    });
    
    sleep(0.5);
  }
}

/**
 * Test basic CRUD for contacts
 */
function testContactCrud() {
  // Create a contact
  const contact = generateContact();
  const createUrl = `${baseUrl}/api/contacts`;
  
  const createRes = http.post(createUrl, JSON.stringify(contact), {
    headers: { 'Content-Type': 'application/json' },
    ...withAuth({}, config.auth.token),
  });
  
  responseTimes.add(createRes.timings.duration);
  errorRate.add(createRes.status >= 400);
  successRate.add(createRes.status === 200);
  
  check(createRes, {
    'Contact creation successful': (r) => r.status === 200,
    'Contact creation returns JSON': (r) => r.headers['Content-Type']?.includes('application/json'),
    'Contact creation returns PID': (r) => {
      const body = JSON.parse(r.body);
      return body?.pid && typeof body.pid === 'string';
    },
  });
  
  if (createRes.status === 200) {
    const body = JSON.parse(createRes.body);
    const pid = body.pid;
    const contactUrl = `${baseUrl}/api/contacts/${pid}`;
    
    // Read the contact
    const readRes = http.get(contactUrl, withAuth({}, config.auth.token));
    
    responseTimes.add(readRes.timings.duration);
    errorRate.add(readRes.status >= 400);
    successRate.add(readRes.status === 200);
    
    check(readRes, {
      'Contact read successful': (r) => r.status === 200,
      'Contact read returns correct PID': (r) => {
        const body = JSON.parse(r.body);
        return body?.pid === pid;
      },
    });
    
    // List contacts
    const listRes = http.get(`${baseUrl}/api/contacts`, withAuth({}, config.auth.token));
    
    responseTimes.add(listRes.timings.duration);
    errorRate.add(listRes.status >= 400);
    successRate.add(listRes.status === 200);
    
    check(listRes, {
      'Contact list successful': (r) => r.status === 200,
      'Contact list returns array': (r) => {
        const body = JSON.parse(r.body);
        return Array.isArray(body?.items);
      },
    });
    
    // Delete the contact
    const deleteRes = http.del(contactUrl, withAuth({}, config.auth.token));
    
    responseTimes.add(deleteRes.timings.duration);
    errorRate.add(deleteRes.status >= 400);
    successRate.add(deleteRes.status === 200 || deleteRes.status === 204);
    
    check(deleteRes, {
      'Contact deletion successful': (r) => r.status === 200 || r.status === 204,
    });
  }
  
  sleep(1);
}

/**
 * Test basic CRUD for leads
 */
function testLeadCrud() {
  const lead = generateLead();
  const createUrl = `${baseUrl}/api/leads`;
  
  const createRes = http.post(createUrl, JSON.stringify(lead), {
    headers: { 'Content-Type': 'application/json' },
    ...withAuth({}, config.auth.token),
  });
  
  responseTimes.add(createRes.timings.duration);
  errorRate.add(createRes.status >= 400);
  successRate.add(createRes.status === 200);
  
  check(createRes, {
    'Lead creation successful': (r) => r.status === 200,
  });
  
  if (createRes.status === 200) {
    const body = JSON.parse(createRes.body);
    const pid = body.pid;
    const leadUrl = `${baseUrl}/api/leads/${pid}`;
    
    const readRes = http.get(leadUrl, withAuth({}, config.auth.token));
    
    responseTimes.add(readRes.timings.duration);
    errorRate.add(readRes.status >= 400);
    successRate.add(readRes.status === 200);
    
    check(readRes, {
      'Lead read successful': (r) => r.status === 200,
    });
  }
  
  sleep(1);
}

/**
 * Test basic CRUD for deals
 */
function testDealCrud() {
  const deal = generateDeal();
  const createUrl = `${baseUrl}/api/deals`;
  
  const createRes = http.post(createUrl, JSON.stringify(deal), {
    headers: { 'Content-Type': 'application/json' },
    ...withAuth({}, config.auth.token),
  });
  
  responseTimes.add(createRes.timings.duration);
  errorRate.add(createRes.status >= 400);
  successRate.add(createRes.status === 200);
  
  check(createRes, {
    'Deal creation successful': (r) => r.status === 200,
  });
  
  if (createRes.status === 200) {
    const body = JSON.parse(createRes.body);
    const pid = body.pid;
    const dealUrl = `${baseUrl}/api/deals/${pid}`;
    
    const readRes = http.get(dealUrl, withAuth({}, config.auth.token));
    
    responseTimes.add(readRes.timings.duration);
    errorRate.add(readRes.status >= 400);
    successRate.add(readRes.status === 200);
    
    check(readRes, {
      'Deal read successful': (r) => r.status === 200,
    });
  }
  
  sleep(1);
}

/**
 * Test search functionality
 */
function testSearch() {
  const searchUrl = `${baseUrl}/api/contacts?limit=10`;
  const res = http.get(searchUrl, withAuth({}, config.auth.token));
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  
  check(res, {
    'Search successful': (r) => r.status === 200,
  });
  
  sleep(0.5);
}

/**
 * Main test function
 */
export default function () {
  // Test health endpoints
  testHealthEndpoints();
  
  // Test API documentation
  testApiDocs();
  
  // Test CRUD operations
  testContactCrud();
  testLeadCrud();
  testDealCrud();
  
  // Test search
  testSearch();
  
  // Small delay between iterations
  sleep(1);
}

/**
 * Setup function - runs once before all VUs start
 */
export function setup() {
  console.log(`Starting smoke test against ${baseUrl}`);
  console.log(`Authentication required: ${config.auth.requireAuth}`);
  return {};
}

/**
 * Teardown function - runs once after all VUs finish
 */
export function teardown(data) {
  console.log('Smoke test completed');
  console.log(`Success rate: ${successRate.count / (successRate.count + errorRate.count) * 100}%`);
  console.log(`Error rate: ${errorRate.count / (successRate.count + errorRate.count) * 100}%`);
}
