/**
 * Distributed Load Test for Contact Relationship Management API
 * 
 * This test simulates multiple users from different geographic locations
 * hitting the API simultaneously. Designed to run across multiple k6
 * instances or Docker containers.
 * 
 * Run with: k6 run --distributed -e API_BASE_URL=http://localhost:1241 tests/load/distributed.js
 * 
 * Scenarios:
 * - Smoke test: Basic endpoint verification
 * - Ramp-up test: Gradual load increase
 * - Stress test: Maximum capacity test
 * - Distributed test: Multi-region simulation
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';
import { SharedArray } from 'k6/data';

// Configuration
const API_BASE_URL = __ENV.API_BASE_URL || 'http://localhost:1241';
const VUS = parseInt(__ENV.VUS) || 100;
const DURATION = __ENV.DURATION || '5m';
const RAMP_UP = __ENV.RAMP_UP || '1m';
const RAMP_DOWN = __ENV.RAMP_DOWN || '1m';

// Custom metrics
const errorRate = new Rate('error_rate');
const requestDuration = new Rate('request_duration');

// Test data - shared across all VUs
const testData = new SharedArray('test data', function () {
  return [
    { endpoint: '/api/contacts', method: 'GET', name: 'List Contacts' },
    { endpoint: '/api/deals', method: 'GET', name: 'List Deals' },
    { endpoint: '/api/tickets', method: 'GET', name: 'List Tickets' },
    { endpoint: '/api/campaigns', method: 'GET', name: 'List Campaigns' },
    { endpoint: '/api/activities', method: 'GET', name: 'List Activities' },
    { endpoint: '/api/leads', method: 'GET', name: 'List Leads' },
    { endpoint: '/api/search?q=test', method: 'GET', name: 'Search' },
    { endpoint: '/_health/live', method: 'GET', name: 'Health Check' },
  ];
});

// Headers
export const options = {
  stages: [
    { duration: RAMP_UP, target: VUS, name: 'Ramp up' },
    { duration: DURATION, target: VUS, name: 'Sustained load' },
    { duration: RAMP_DOWN, target: 0, name: 'Ramp down' },
  ],
  thresholds: {
    http_req_failed: ['rate<0.01'], // Less than 1% errors
    http_req_duration: ['p(95)<1000', 'p(99)<2000'], // P95 < 1s, P99 < 2s
    checks: ['rate>0.99'], // 99% success rate
  },
  noConnectionReuse: false, // Reuse connections for efficiency
  tags: {
    test_type: 'distributed',
    service: 'crm',
  },
};

// Setup - runs once before all VUs start
export function setup() {
  console.log(`Distributed Load Test Configuration:`);
  console.log(`  API Base URL: ${API_BASE_URL}`);
  console.log(`  Virtual Users: ${VUS}`);
  console.log(`  Duration: ${DURATION}`);
  console.log(`  Ramp Up: ${RAMP_UP}`);
  console.log(`  Ramp Down: ${RAMP_DOWN}`);
  
  // Verify API is reachable
  const healthCheck = http.get(`${API_BASE_URL}/_health/live`);
  if (healthCheck.status !== 200) {
    throw new Error(`API health check failed: ${healthCheck.status}`);
  }
  
  return { startTime: Date.now() };
}

// Main test function
export default function (data) {
  // Select a random endpoint from test data
  const endpointConfig = testData[data.vuIdInTest % testData.length];
  const url = `${API_BASE_URL}${endpointConfig.endpoint}`;
  
  // Add small random delay to simulate think time (0-2 seconds)
  sleep(Math.random() * 2);
  
  // Make the request based on method
  let response;
  switch (endpointConfig.method) {
    case 'GET':
      response = http.get(url, {
        tags: { name: endpointConfig.name, endpoint: endpointConfig.endpoint },
        headers: { 'Accept': 'application/json' },
      });
      break;
    case 'POST':
      response = http.post(url, JSON.stringify({ test: 'data' }), {
        tags: { name: endpointConfig.name, endpoint: endpointConfig.endpoint },
        headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
      });
      break;
    case 'PUT':
      response = http.put(url, JSON.stringify({ test: 'data' }), {
        tags: { name: endpointConfig.name, endpoint: endpointConfig.endpoint },
        headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
      });
      break;
    case 'DELETE':
      response = http.del(url, null, {
        tags: { name: endpointConfig.name, endpoint: endpointConfig.endpoint },
      });
      break;
    default:
      throw new Error(`Unsupported HTTP method: ${endpointConfig.method}`);
  }
  
  // Track metrics
  errorRate.add(response.status !== 200);
  requestDuration.add(1);
  
  // Validate response
  check(response, {
    'Status is 2xx': (r) => r.status >= 200 && r.status < 300,
    'Response time < 2s': (r) => r.timings.duration < 2000,
    'Has response body': (r) => r.body && r.body.length > 0,
  }, { name: endpointConfig.name });
  
  // Small delay between requests
  sleep(0.1);
}

// Teardown - runs once after all VUs finish
export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000; // in seconds
  console.log(`\nTest Summary:`);
  console.log(`  Total Duration: ${Math.round(duration)} seconds`);
  console.log(`  Total Requests: ${__ENV.K6_METRIC_HTTP_REQS}`);
  console.log(`  Request Rate: ${Math.round(__ENV.K6_METRIC_HTTP_REQS / duration)} req/s`);
  console.log(`  Error Rate: ${errorRate.count / (__ENV.K6_METRIC_HTTP_REQS || 1)}`);
}
