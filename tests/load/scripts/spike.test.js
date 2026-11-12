/**
 * Spike Test (Sudden Traffic Surge)
 * 
 * Purpose: Test system behavior under sudden, unexpected traffic spikes
 * Duration: 5-10 minutes
 * 
 * This test simulates:
 * - Marketing campaign launch
 * - News coverage driving traffic
 * - DDoS-like sudden surge
 * - Black Friday / sale events
 * 
 * It verifies:
 * - Application can handle sudden traffic increases
 * - Auto-scaling triggers (if configured)
 * - Connection pool sizing is adequate
 * - Rate limiting / circuit breakers work
 * - System recovers after spike
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
const rateLimitedRequests = new Counter('rate_limited_requests');
const timeoutErrors = new Counter('timeout_errors');
const spikeResponseTimes = new Trend('spike_response_times');

// Get configuration
const config = dev;
const baseUrl = config.baseUrl;

// Spike test configuration
const spikeConfig = {
  // Start with normal load
  normal: {
    duration: '30s',
    target: 10,
  },
  // Sudden spike to maximum load
  spike: {
    duration: '2m',
    target: config.vus.max,
  },
  // Ramp down back to normal
  recovery: {
    duration: '1m',
    target: 10,
  },
  // Continue with normal load to verify recovery
  postSpike: {
    duration: '2m',
    target: 10,
  },
};

// Test setup for spike test
export const options = {
  vus: 10, // Starting VUs
  duration: '5m30s',
  stages: [
    { duration: spikeConfig.normal.duration, target: spikeConfig.normal.target },
    { duration: spikeConfig.spike.duration, target: spikeConfig.spike.target },
    { duration: spikeConfig.recovery.duration, target: spikeConfig.recovery.target },
    { duration: spikeConfig.postSpike.duration, target: spikeConfig.postSpike.target },
  ],
  thresholds: {
    ...config.thresholds,
    // Spike test specific thresholds
    http_req_failed: ['rate<0.5'], // Allow higher failure rate during spike
    http_req_duration: ['p(95)<5000'], // Allow slower responses during spike
    http_req_duration: ['p(50)<1000'], // Median should still be reasonable
    // Rate limiting should be respected
    http_req_failed: ['rate<0.3', { metric: 'rate_limited_requests' }],
    // Timeouts should be minimal
    http_req_failed: ['rate<0.1', { metric: 'timeout_errors' }],
    requests: ['count>5000'],
  },
};

// Track state
let currentStage = 'normal';
let inSpike = false;
let spikeStartTime = 0;
let maxConcurrentRequests = 0;
let concurrentRequests = 0;

/**
 * Make a simple GET request
 */
function makeGetRequest(url) {
  const res = http.get(url, withAuth({}, config.auth.token));
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200);
  requests.add(1);
  
  if (res.status === 429) {
    rateLimitedRequests.add(1);
  }
  if (res.error_code === 1050) {
    timeoutErrors.add(1);
  }
  
  // Track spike-specific metrics
  if (inSpike) {
    spikeResponseTimes.add(res.timings.duration);
  }
  
  return res;
}

/**
 * Make a POST request
 */
function makePostRequest(url, body) {
  const res = http.post(url, JSON.stringify(body), {
    headers: { 'Content-Type': 'application/json' },
    ...withAuth({}, config.auth.token),
  });
  
  responseTimes.add(res.timings.duration);
  errorRate.add(res.status >= 400);
  successRate.add(res.status === 200 || res.status === 201);
  requests.add(1);
  
  if (res.status === 429) {
    rateLimitedRequests.add(1);
  }
  if (res.error_code === 1050) {
    timeoutErrors.add(1);
  }
  
  // Track spike-specific metrics
  if (inSpike) {
    spikeResponseTimes.add(res.timings.duration);
  }
  
  return res;
}

/**
 * Quick read operations for spike testing
 */
function quickReadOperations() {
  const endpoints = [
    '/api/contacts?limit=50',
    '/api/deals?limit=50',
    '/api/leads?limit=50',
    '/api/activities?limit=50',
    '/_health/live',
    '/_health/ready',
  ];
  
  const endpoint = endpoints[randomInt(0, endpoints.length)];
  return makeGetRequest(`${baseUrl}${endpoint}`);
}

/**
 * Quick create operations for spike testing
 */
function quickCreateOperations() {
  const types = ['contact', 'lead', 'deal', 'activity'];
  const type = types[randomInt(0, types.length)];
  
  let entity;
  switch (type) {
    case 'contact':
      entity = generateContact();
      return makePostRequest(`${baseUrl}/api/contacts`, entity);
    case 'lead':
      entity = generateLead();
      return makePostRequest(`${baseUrl}/api/leads`, entity);
    case 'deal':
      entity = generateDeal();
      return makePostRequest(`${baseUrl}/api/deals`, entity);
    case 'activity':
      entity = generateActivity();
      return makePostRequest(`${baseUrl}/api/activities`, entity);
    default:
      return quickReadOperations();
  }
}

/**
 * Mixed operation for spike testing
 */
function mixedOperations() {
  const operation = randomInt(0, 2);
  
  switch (operation) {
    case 0:
      return quickReadOperations();
    case 1:
      return quickCreateOperations();
    default:
      // Simple health check
      return makeGetRequest(`${baseUrl}/_health/live`);
  }
}

/**
 * Update current stage based on test duration
 */
function updateStage() {
  const elapsed = Date.now() - spikeStartTime;
  
  if (elapsed < 30000) {
    currentStage = 'normal';
    inSpike = false;
  } else if (elapsed < 150000) {
    currentStage = 'spike';
    inSpike = true;
  } else if (elapsed < 210000) {
    currentStage = 'recovery';
    inSpike = false;
  } else {
    currentStage = 'postSpike';
    inSpike = false;
  }
}

/**
 * Main test function
 */
export default function () {
  concurrentRequests++;
  maxConcurrentRequests = Math.max(maxConcurrentRequests, concurrentRequests);
  
  // Update current stage
  updateStage();
  
  // During spike, make very fast, simple requests
  if (inSpike) {
    // Very aggressive: make 10 requests per iteration during spike
    for (let i = 0; i < 10; i++) {
      mixedOperations();
    }
    // Minimal sleep during spike to maximize load
    sleep(randomInt(0, 100) / 1000); // 0-100ms sleep
  } else {
    // Normal pace: 1-5 requests per iteration
    const numRequests = randomInt(1, 5);
    for (let i = 0; i < numRequests; i++) {
      mixedOperations();
    }
    // Normal sleep
    sleep(randomInt(1, 3));
  }
  
  concurrentRequests--;
}

/**
 * Setup function
 */
export function setup() {
  spikeStartTime = Date.now();
  console.log(`Starting spike test against ${baseUrl}`);
  console.log(`Spike configuration:`);
  console.log(`  Normal: ${spikeConfig.normal.target} VUs for ${spikeConfig.normal.duration}`);
  console.log(`  Spike: ${spikeConfig.spike.target} VUs for ${spikeConfig.spike.duration}`);
  console.log(`  Recovery: ${spikeConfig.recovery.target} VUs for ${spikeConfig.recovery.duration}`);
  console.log(`  Post-spike: ${spikeConfig.postSpike.target} VUs for ${spikeConfig.postSpike.duration}`);
  return {};
}

/**
 * Teardown function
 */
export function teardown(data) {
  console.log('Spike test completed');
  console.log('=== Results ===');
  console.log(`Total requests: ${requests.count}`);
  console.log(`Success rate: ${successRate.count / (successRate.count + errorRate.count) * 100}%`);
  console.log(`Error rate: ${errorRate.count / (successRate.count + errorRate.count) * 100}%`);
  console.log(`Rate limited requests: ${rateLimitedRequests.count}`);
  console.log(`Timeout errors: ${timeoutErrors.count}`);
  console.log(`Max concurrent requests: ${maxConcurrentRequests}`);
  console.log('');
  console.log('=== Response Time Metrics ===');
  console.log(`Normal operation average: ${responseTimes.avg}ms`);
  console.log(`Spike operation average: ${spikeResponseTimes.avg}ms`);
  console.log(`P50 (all): ${responseTimes.percentile(50)}ms`);
  console.log(`P90 (all): ${responseTimes.percentile(90)}ms`);
  console.log(`P95 (all): ${responseTimes.percentile(95)}ms`);
  console.log(`P95 (during spike): ${spikeResponseTimes.percentile(95)}ms`);
  console.log('');
  
  // Calculate degradation
  const normalAvg = responseTimes.avg;
  const spikeAvg = spikeResponseTimes.avg;
  const degradation = spikeAvg > 0 && normalAvg > 0 
    ? ((spikeAvg - normalAvg) / normalAvg * 100) 
    : 0;
  
  console.log('=== Spike Analysis ===');
  console.log(`Response time degradation during spike: ${degradation.toFixed(2)}%`);
  console.log(`System handled ${requests.count} requests with ${rateLimitedRequests.count} rate limited`);
  
  // Warn if degradation is too high
  if (degradation > 500) {
    console.warn('WARNING: Response time increased by more than 500% during spike!');
    console.warn('This indicates the system may not scale well under sudden load.');
  }
  
  // Warn about rate limiting
  if (rateLimitedRequests.count > requests.count * 0.1) {
    console.warn('WARNING: More than 10% of requests were rate limited!');
    console.warn('Consider adjusting rate limiting thresholds or scaling capacity.');
  }
}
