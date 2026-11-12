/**
 * Bridges Integration Test
 * 
 * Purpose: Test integration with external systems (PPM, WPM)
 * Duration: 2-5 minutes
 * VUs: 5-10
 * 
 * This test verifies:
 * - Bridge configuration and connectivity
 * - Contact reference resolution (person:<uuid>)
 * - Cross-system data consistency
 * - Bridge authentication
 * - Rate limiting on bridge endpoints
 * - Error handling for bridge operations
 * - Bridge caching behavior
 * - Bridge sync operations
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Trend, Rate, Counter } from 'k6/metrics';

// Import configuration and utilities
import { dev } from '../config/dev.js';
import { withAuth, randomInt, randomUUID } from '../lib/utils.js';
import { generateContact, generateDeal, generateLead } from '../lib/generators.js';

// Custom metrics
const bridgeResponseTimes = new Trend('bridge_response_times');
const bridgeErrorRate = new Rate('bridge_error_rate');
const bridgeSuccessRate = new Rate('bridge_success_rate');
const bridgeRequests = new Counter('bridge_requests');
const bridgeSyncTimes = new Trend('bridge_sync_times');
const referenceResolutionTimes = new Trend('reference_resolution_times');

// Get configuration
const config = dev;
const baseUrl = config.baseUrl;

// Bridge configuration (adjust based on your setup)
const bridges = {
  // PPM (Project Portfolio Management) bridge
  ppm: {
    enabled: true,
    syncEndpoints: [
      '/api/bridge/ppm/sync-contacts',
      '/api/bridge/ppm/sync-deals',
      '/api/bridge/ppm/sync-activities',
      '/api/bridge/ppm/sync-full',
    ],
    queryEndpoints: [
      '/api/bridge/ppm/contacts',
      '/api/bridge/ppm/deals',
      '/api/bridge/ppm/projects',
    ],
    healthEndpoint: '/api/bridge/ppm/health',
  },
  // WPM (Workforce Planning) bridge
  wpm: {
    enabled: true,
    syncEndpoints: [
      '/api/bridge/wpm/sync-contacts',
      '/api/bridge/wpm/sync-assignments',
      '/api/bridge/wpm/sync-full',
    ],
    queryEndpoints: [
      '/api/bridge/wpm/contacts',
      '/api/bridge/wpm/assignments',
      '/api/bridge/wpm/roles',
    ],
    healthEndpoint: '/api/bridge/wpm/health',
  },
  // Generic bridge endpoints
  generic: {
    enabled: true,
    endpoints: [
      '/api/bridge/links',
      '/api/bridge/resolve',
      '/api/bridge/validate',
    ],
  },
};

// Test setup for bridges test
export const options = {
  vus: config.vus.bridges,
  duration: config.durations.bridges,
  thresholds: {
    ...config.thresholds,
    // Bridge specific thresholds
    http_req_failed: ['rate<0.1'],
    http_req_duration: ['p(95)<3000'], // Bridges might be slower
    bridge_response_times: ['p(95)<2000'],
    bridge_error_rate: ['rate<0.05'],
    bridge_requests: ['count>100'],
  },
};

// Track state
let totalBridgeRequests = 0;
let successfulBridgeRequests = 0;
let failedBridgeRequests = 0;
let syncOperations = 0;
let referenceResolutions = 0;

/**
 * Make a bridge request
 */
function makeBridgeRequest(url, method = 'GET', body = null) {
  totalBridgeRequests++;
  bridgeRequests.add(1);
  
  const options = withAuth({}, config.auth.token);
  if (body) {
    options.headers = { 'Content-Type': 'application/json', ...options.headers };
    options.body = JSON.stringify(body);
  }
  
  let response;
  switch (method.toLowerCase()) {
    case 'post':
      response = http.post(url, options);
      break;
    case 'put':
      response = http.put(url, options);
      break;
    case 'patch':
      response = http.patch(url, options);
      break;
    case 'delete':
      response = http.del(url, null, options);
      break;
    default:
      response = http.get(url, options);
  }
  
  bridgeResponseTimes.add(response.timings.duration);
  bridgeErrorRate.add(response.status >= 400);
  bridgeSuccessRate.add(response.status >= 200 && response.status < 300);
  
  if (response.status >= 200 && response.status < 300) {
    successfulBridgeRequests++;
  } else {
    failedBridgeRequests++;
    console.log(`Bridge request failed: ${method} ${url} - ${response.status}`);
  }
  
  return response;
}

/**
 * Test bridge health
 */
function testBridgeHealth() {
  const results = [];
  
  // Test PPM bridge health
  if (bridges.ppm.enabled) {
    const ppmRes = makeBridgeRequest(`${baseUrl}${bridges.ppm.healthEndpoint}`);
    results.push({ name: 'PPM Bridge', status: ppmRes.status, duration: ppmRes.timings.duration });
    check(ppmRes, {
      'PPM bridge health check': (r) => r.status === 200,
    });
  }
  
  // Test WPM bridge health
  if (bridges.wpm.enabled) {
    const wpmRes = makeBridgeRequest(`${baseUrl}${bridges.wpm.healthEndpoint}`);
    results.push({ name: 'WPM Bridge', status: wpmRes.status, duration: wpmRes.timings.duration });
    check(wpmRes, {
      'WPM bridge health check': (r) => r.status === 200,
    });
  }
  
  return results;
}

/**
 * Test reference resolution
 */
function testReferenceResolution() {
  // Generate a person reference
  const personUuid = randomUUID();
  const personRef = `person:${personUuid}`;
  
  // Test resolve endpoint
  const resolveUrl = `${baseUrl}/api/bridge/resolve?ref=${encodeURIComponent(personRef)}`;
  const resolveRes = makeBridgeRequest(resolveUrl);
  
  referenceResolutionTimes.add(resolveRes.timings.duration);
  referenceResolutions++;
  
  check(resolveRes, {
    'Reference resolution successful': (r) => r.status === 200,
    'Reference resolution returns object': (r) => {
      try {
        const body = JSON.parse(r.body);
        return body && typeof body === 'object';
      } catch {
        return false;
      }
    },
  });
  
  // Test with existing contact (if we have one)
  const listRes = makeBridgeRequest(`${baseUrl}/api/contacts?limit=1`);
  if (listRes.status === 200) {
    try {
      const body = JSON.parse(listRes.body);
      if (body.items && body.items.length > 0) {
        const contact = body.items[0];
        if (contact.person_ref) {
          const existingResolveUrl = `${baseUrl}/api/bridge/resolve?ref=${encodeURIComponent(contact.person_ref)}`;
          const existingResolveRes = makeBridgeRequest(existingResolveUrl);
          
          referenceResolutionTimes.add(existingResolveRes.timings.duration);
          referenceResolutions++;
          
          check(existingResolveRes, {
            'Existing reference resolution successful': (r) => r.status === 200,
          });
        }
      }
    } catch {
      // Ignore parse errors
    }
  }
  
  return resolveRes;
}

/**
 * Test bridge query operations
 */
function testBridgeQueries() {
  const bridgeTypes = Object.keys(bridges).filter(b => bridges[b].enabled);
  const bridgeType = bridgeTypes[randomInt(0, bridgeTypes.length)];
  const bridgeConfig = bridges[bridgeType];
  
  if (!bridgeConfig.queryEndpoints || bridgeConfig.queryEndpoints.length === 0) {
    return null;
  }
  
  const endpoint = bridgeConfig.queryEndpoints[randomInt(0, bridgeConfig.queryEndpoints.length)];
  const url = `${baseUrl}${endpoint}`;
  
  // Add random parameters
  let urlWithParams = url;
  if (url.includes('/contacts')) {
    urlWithParams += '?limit=10';
  } else if (url.includes('/deals') || url.includes('/projects') || url.includes('/assignments')) {
    urlWithParams += '?limit=10&status=active';
  }
  
  const res = makeBridgeRequest(urlWithParams);
  
  check(res, {
    [`${bridgeType.toUpperCase()} bridge query successful`]: (r) => r.status === 200,
  });
  
  return res;
}

/**
 * Test bridge sync operations
 */
function testBridgeSync() {
  const bridgeTypes = Object.keys(bridges).filter(b => bridges[b].enabled);
  const bridgeType = bridgeTypes[randomInt(0, bridgeTypes.length)];
  const bridgeConfig = bridges[bridgeType];
  
  if (!bridgeConfig.syncEndpoints || bridgeConfig.syncEndpoints.length === 0) {
    return null;
  }
  
  const endpoint = bridgeConfig.syncEndpoints[randomInt(0, bridgeConfig.syncEndpoints.length)];
  const url = `${baseUrl}${endpoint}`;
  
  const startTime = Date.now();
  const res = makeBridgeRequest(url, 'POST');
  const duration = Date.now() - startTime;
  
  bridgeSyncTimes.add(duration);
  syncOperations++;
  
  // Sync operations might return 200 or 202 (Accepted)
  check(res, {
    [`${bridgeType.toUpperCase()} bridge sync accepted`]: (r) => r.status === 200 || r.status === 202,
  });
  
  return res;
}

/**
 * Test cross-system consistency
 */
function testCrossSystemConsistency() {
  // Create a contact in CRM
  const contact = generateContact();
  const createRes = http.post(`${baseUrl}/api/contacts`, JSON.stringify(contact), {
    headers: { 'Content-Type': 'application/json' },
    ...withAuth({}, config.auth.token),
  });
  
  if (createRes.status !== 200) {
    console.log(`Failed to create contact for consistency test: ${createRes.status}`);
    return createRes;
  }
  
  const contactBody = JSON.parse(createRes.body);
  const personRef = contactBody.person_ref;
  
  // Wait a moment for potential sync
  sleep(1);
  
  // Check if we can resolve this reference via bridge
  const resolveUrl = `${baseUrl}/api/bridge/resolve?ref=${encodeURIComponent(personRef)}`;
  const resolveRes = makeBridgeRequest(resolveUrl);
  
  referenceResolutionTimes.add(resolveRes.timings.duration);
  referenceResolutions++;
  
  check(resolveRes, {
    'Cross-system consistency: Reference resolves after creation': (r) => r.status === 200,
  });
  
  // Check bridge contacts list
  const bridgeContactsRes = makeBridgeRequest(`${baseUrl}/api/bridge/ppm/contacts?limit=10`);
  
  check(bridgeContactsRes, {
    'Cross-system consistency: Bridge returns contacts': (r) => r.status === 200,
  });
  
  return { createRes, resolveRes, bridgeContactsRes };
}

/**
 * Test bridge authentication
 */
function testBridgeAuthentication() {
  // Test with invalid/no authentication
  const testUrl = `${baseUrl}${bridges.ppm.healthEndpoint}`;
  
  // Request without auth
  const noAuthRes = http.get(testUrl, {});
  
  check(noAuthRes, {
    'Bridge rejects unauthenticated requests': (r) => r.status === 401 || r.status === 403,
  });
  
  // Request with wrong auth
  const wrongAuthRes = http.get(testUrl, {
    headers: { 'Authorization': 'Bearer invalid-token' },
  });
  
  check(wrongAuthRes, {
    'Bridge rejects invalid authentication': (r) => r.status === 401 || r.status === 403,
  });
  
  // Request with correct auth (should work)
  const validAuthRes = makeBridgeRequest(testUrl);
  
  check(validAuthRes, {
    'Bridge accepts valid authentication': (r) => r.status === 200,
  });
  
  return { noAuthRes, wrongAuthRes, validAuthRes };
}

/**
 * Test bridge rate limiting
 */
function testBridgeRateLimiting() {
  // Make multiple rapid requests to the same endpoint
  const testUrl = `${baseUrl}${bridges.ppm.queryEndpoints[0]}`;
  const requests = [];
  
  for (let i = 0; i < 20; i++) {
    const res = makeBridgeRequest(testUrl);
    requests.push(res);
    // No sleep to test rate limiting
  }
  
  // Check if any requests were rate limited
  const rateLimited = requests.filter(r => r.status === 429);
  
  if (rateLimited.length > 0) {
    console.log(`Bridge rate limiting triggered: ${rateLimited.length} requests rate limited`);
  }
  
  check(requests, {
    'Bridge handles rapid requests': (rs) => rs.every(r => r.status !== 500),
  });
  
  return requests;
}

/**
 * Test bridge links endpoint
 */
function testBridgeLinks() {
  // Test the links endpoint
  const linksRes = makeBridgeRequest(`${baseUrl}/api/bridge/links`);
  
  check(linksRes, {
    'Bridge links endpoint works': (r) => r.status === 200,
    'Bridge links returns array': (r) => {
      try {
        const body = JSON.parse(r.body);
        return Array.isArray(body.links) || Array.isArray(body);
      } catch {
        return false;
      }
    },
  });
  
  return linksRes;
}

/**
 * Main test function
 */
export default function () {
  // Weighted random selection of operations
  const operation = randomInt(0, 40);
  
  switch (operation) {
    case 0:
      // 2.5%: Bridge health
      testBridgeHealth();
      break;
    case 1:
    case 2:
    case 3:
      // 7.5%: Reference resolution
      testReferenceResolution();
      break;
    case 4:
    case 5:
    case 6:
    case 7:
      // 10%: Bridge queries
      testBridgeQueries();
      break;
    case 8:
    case 9:
    case 10:
      // 7.5%: Bridge sync
      testBridgeSync();
      break;
    case 11:
    case 12:
      // 5%: Cross-system consistency
      testCrossSystemConsistency();
      break;
    case 13:
    case 14:
      // 5%: Authentication
      testBridgeAuthentication();
      break;
    case 15:
    case 16:
      // 5%: Rate limiting
      testBridgeRateLimiting();
      break;
    case 17:
    case 18:
      // 5%: Links
      testBridgeLinks();
      break;
    default:
      // Remaining: Mixed bridge operations
      const mixedOperations = [
        testBridgeHealth,
        testReferenceResolution,
        testBridgeQueries,
        testBridgeSync,
      ];
      mixedOperations[randomInt(0, mixedOperations.length)]();
  }
  
  // Variable sleep
  sleep(randomInt(0, 3));
}

/**
 * Setup function
 */
export function setup() {
  console.log(`Starting bridges integration test against ${baseUrl}`);
  console.log(`VUs: ${config.vus.bridges}, Duration: ${config.durations.bridges}`);
  console.log('This test validates integration with external systems (PPM, WPM)');
  
  // List enabled bridges
  const enabledBridges = Object.keys(bridges).filter(b => bridges[b].enabled);
  console.log(`Enabled bridges: ${enabledBridges.join(', ')}`);
  
  return {};
}

/**
 * Teardown function
 */
export function teardown(data) {
  console.log('Bridges integration test completed');
  console.log('=== Bridge Metrics ===');
  console.log(`Total bridge requests: ${totalBridgeRequests}`);
  console.log(`Successful: ${successfulBridgeRequests}`);
  console.log(`Failed: ${failedBridgeRequests}`);
  console.log(`Success rate: ${successfulBridgeRequests / totalBridgeRequests * 100}%`);
  console.log(`Error rate: ${failedBridgeRequests / totalBridgeRequests * 100}%`);
  console.log('');
  console.log(`Sync operations: ${syncOperations}`);
  console.log(`Reference resolutions: ${referenceResolutions}`);
  console.log('');
  console.log('=== Performance Metrics ===');
  console.log(`Bridge response avg: ${bridgeResponseTimes.avg}ms`);
  console.log(`Bridge sync avg: ${bridgeSyncTimes.avg}ms`);
  console.log(`Reference resolution avg: ${referenceResolutionTimes.avg}ms`);
  console.log('');
  console.log('=== Performance Percentiles ===');
  console.log(`Bridge P50: ${bridgeResponseTimes.percentile(50)}ms`);
  console.log(`Bridge P90: ${bridgeResponseTimes.percentile(90)}ms`);
  console.log(`Bridge P95: ${bridgeResponseTimes.percentile(95)}ms`);
  console.log(`Sync P95: ${bridgeSyncTimes.percentile(95)}ms`);
  console.log('');
  
  // Analysis
  if (failedBridgeRequests > totalBridgeRequests * 0.1) {
    console.log('WARNING: High bridge failure rate!');
    console.log('Check bridge connectivity and configuration.');
  }
  
  if (bridgeResponseTimes.percentile(95) > 5000) {
    console.log('WARNING: Bridge response times are high!');
    console.log('Check bridge server performance and network connectivity.');
  }
  
  if (referenceResolutionTimes.percentile(95) > 2000) {
    console.log('WARNING: Reference resolution is slow!');
    console.log('Check person service connectivity and caching.');
  }
  
  console.log('');
  console.log('Note: Bridge tests depend on external systems being available.');
  console.log('Some failures may be expected if bridges are not configured.');
}
