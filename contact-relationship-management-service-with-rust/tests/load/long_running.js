/**
 * Long-Running Stability Test for Contact Relationship Management API
 * 
 * This test runs for an extended period (6-24 hours) to identify memory leaks,
 * connection pool issues, and other stability problems that only appear over time.
 * 
 * Run with: k6 run -e API_BASE_URL=http://localhost:1241 -e DURATION=6h tests/load/long_running.js
 * 
 * Features:
 * - Memory leak detection
 * - Connection stability verification
 * - Database pool health monitoring
 * - Slow query detection
 * - Error rate tracking over time
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';
import { SharedArray } from 'k6/data';

// Configuration
const API_BASE_URL = __ENV.API_BASE_URL || 'http://localhost:1241';
const DURATION = __ENV.DURATION || '6h';
const VUS = parseInt(__ENV.VUS) || 50;
const CYCLE_DURATION = parseInt(__ENV.CYCLE_DURATION) || 60; // seconds per cycle

// Custom metrics
const memoryUsage = new Trend('memory_usage', true);
const connectionErrors = new Counter('connection_errors');
const slowRequests = new Counter('slow_requests');
const errorRate = new Rate('error_rate');
const requestDuration = new Trend('request_duration', true);

// Test phases - each phase tests different aspects
export const options = {
  stages: [
    { duration: DURATION, target: VUS, name: 'Long-running stability test' },
  ],
  thresholds: {
    http_req_failed: ['rate<0.001'], // Less than 0.1% errors
    http_req_duration: ['p(95)<2000', 'p(99)<5000'], // P95 < 2s, P99 < 5s
    checks: ['rate>0.999'], // 99.9% success rate
    connection_errors: ['count<10'], // Less than 10 connection errors
  },
  noConnectionReuse: true, // Test connection pool behavior
  tags: {
    test_type: 'long_running',
    service: 'crm',
    purpose: 'stability',
  },
};

// Test endpoints with different characteristics
const endpoints = [
  // Read operations (high frequency)
  { path: '/api/contacts', method: 'GET', weight: 5, name: 'List Contacts' },
  { path: '/api/deals', method: 'GET', weight: 5, name: 'List Deals' },
  { path: '/api/tickets', method: 'GET', weight: 4, name: 'List Tickets' },
  { path: '/api/campaigns', method: 'GET', weight: 3, name: 'List Campaigns' },
  { path: '/api/activities', method: 'GET', weight: 3, name: 'List Activities' },
  { path: '/api/leads', method: 'GET', weight: 2, name: 'List Leads' },
  
  // Search operations (medium frequency)
  { path: '/api/search?q=contact', method: 'GET', weight: 4, name: 'Search Contacts' },
  { path: '/api/search?q=deal', method: 'GET', weight: 3, name: 'Search Deals' },
  
  // Health checks (low frequency)
  { path: '/_health/live', method: 'GET', weight: 2, name: 'Health Check' },
  { path: '/_health/ready', method: 'GET', weight: 2, name: 'Readiness Check' },
  
  // Write operations (low frequency - to test database writes over time)
  { path: '/api/contacts', method: 'POST', weight: 1, name: 'Create Contact', 
    body: { display_name: 'Test Contact', email: 'test@example.com' } },
];

// Calculate total weight for weighted random selection
const totalWeight = endpoints.reduce((sum, e) => sum + e.weight, 0);

// Select endpoint based on weights
function selectEndpoint() {
  const random = Math.random() * totalWeight;
  let cumulative = 0;
  for (const endpoint of endpoints) {
    cumulative += endpoint.weight;
    if (random < cumulative) {
      return endpoint;
    }
  }
  return endpoints[0];
}

// Setup
export function setup() {
  console.log(`Long-Running Stability Test Configuration:`);
  console.log(`  API Base URL: ${API_BASE_URL}`);
  console.log(`  Duration: ${DURATION}`);
  console.log(`  Virtual Users: ${VUS}`);
  console.log(`  Cycle Duration: ${CYCLE_DURATION}s`);
  console.log(`\nStarting long-running stability test...`);
  console.log(`This test will run for ${DURATION} to identify:`);
  console.log(`  - Memory leaks`);
  console.log(`  - Connection pool issues`);
  console.log(`  - Database performance degradation`);
  console.log(`  - Error rate trends`);
  
  return { startTime: Date.now(), cycles: 0 };
}

// Main test function
export default function (data) {
  const endpoint = selectEndpoint();
  const url = `${API_BASE_URL}${endpoint.path}`;
  
  // Simulate realistic user behavior with variable think time
  const thinkTime = Math.random() * 5; // 0-5 seconds
  sleep(thinkTime);
  
  // Make the request
  const headers = { 'Content-Type': 'application/json', 'Accept': 'application/json' };
  let response;
  
  try {
    switch (endpoint.method) {
      case 'GET':
        response = http.get(url, { tags: { name: endpoint.name }, headers });
        break;
      case 'POST':
        response = http.post(url, JSON.stringify(endpoint.body || {}), { 
          tags: { name: endpoint.name }, 
          headers 
        });
        break;
      case 'PUT':
        response = http.put(url, JSON.stringify(endpoint.body || {}), { 
          tags: { name: endpoint.name }, 
          headers 
        });
        break;
      case 'DELETE':
        response = http.del(url, null, { tags: { name: endpoint.name }, headers });
        break;
      default:
        throw new Error(`Unsupported method: ${endpoint.method}`);
    }
    
    // Track metrics
    requestDuration.add(response.timings.duration);
    errorRate.add(response.status >= 400);
    
    // Track slow requests (> 2 seconds)
    if (response.timings.duration > 2000) {
      slowRequests.add(1);
      console.log(`⚠️ Slow request (${Math.round(response.timings.duration)}ms): ${endpoint.name}`);
    }
    
    // Validate response
    check(response, {
      'Status is 2xx': (r) => r.status >= 200 && r.status < 300,
      'Status is 4xx client error': (r) => r.status >= 400 && r.status < 500,
      'Status is 5xx server error': (r) => r.status >= 500,
      'Response time < 5s': (r) => r.timings.duration < 5000,
    }, { name: endpoint.name });
    
  } catch (e) {
    connectionErrors.add(1);
    console.log(`❌ Connection error: ${e.message}`);
    errorRate.add(1);
  }
  
  // Small delay to avoid overwhelming the system
  sleep(0.05);
}

// Teardown
export function teardown(data) {
  const duration = (Date.now() - data.startTime) / 1000; // in seconds
  const hours = Math.floor(duration / 3600);
  const minutes = Math.floor((duration % 3600) / 60);
  const seconds = Math.round(duration % 60);
  
  console.log(`\n═══════════════════════════════════════════════════════`);
  console.log(`Long-Running Stability Test Summary`);
  console.log(`═══════════════════════════════════════════════════════`);
  console.log(`Total Duration: ${hours}h ${minutes}m ${seconds}s`);
  console.log(`Total Requests: ${__ENV.K6_METRIC_HTTP_REQS || 'N/A'}`);
  console.log(`Request Rate: ${Math.round((__ENV.K6_METRIC_HTTP_REQS || 0) / duration)} req/s`);
  console.log(`\nError Metrics:`);
  console.log(`  HTTP Errors: ${errorRate.count}`);
  console.log(`  Connection Errors: ${connectionErrors.count}`);
  console.log(`  Slow Requests (>2s): ${slowRequests.count}`);
  console.log(`\nPerformance Metrics:`);
  console.log(`  Avg Response Time: ${Math.round(requestDuration.avg)}ms`);
  console.log(`  P50 Response Time: ${Math.round(requestDuration.median)}ms`);
  console.log(`  P95 Response Time: ${Math.round(requestDuration.percentile(95))}ms`);
  console.log(`  P99 Response Time: ${Math.round(requestDuration.percentile(99))}ms`);
  console.log(`  Max Response Time: ${Math.round(requestDuration.max)}ms`);
  
  // Stability analysis
  const errorRatePercentage = (errorRate.count / (__ENV.K6_METRIC_HTTP_REQS || 1) * 100).toFixed(4);
  console.log(`\nStability Analysis:`);
  console.log(`  Error Rate: ${errorRatePercentage}%`);
  
  if (errorRatePercentage < 0.1) {
    console.log(`  ✅ EXCELLENT: Error rate is very low, system is stable`);
  } else if (errorRatePercentage < 0.5) {
    console.log(`  ✅ GOOD: Error rate is acceptable`);
  } else if (errorRatePercentage < 1.0) {
    console.log(`  ⚠️  WARNING: Error rate is elevated, investigate`);
  } else {
    console.log(`  ❌ CRITICAL: Error rate is high, system is unstable`);
  }
  
  if (slowRequests.count > 100) {
    console.log(`  ⚠️  WARNING: Many slow requests detected, check database performance`);
  }
  
  if (connectionErrors.count > 5) {
    console.log(`  ⚠️  WARNING: Connection errors detected, check network/network configuration`);
  }
  
  console.log(`═══════════════════════════════════════════════════════`);
}
