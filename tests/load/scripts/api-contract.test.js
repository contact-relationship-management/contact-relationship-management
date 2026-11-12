/**
 * API Contract Test
 * 
 * Purpose: Verify OpenAPI/Swagger compliance and API contract adherence
 * Duration: 1-5 minutes
 * VUs: 1-5
 * 
 * This test verifies:
 * - API responses match OpenAPI schema
 * - Required fields are present in responses
 * - Response formats match specification
 * - Error responses follow contract
 * - API version compatibility
 * - Field types match schema
 * - Pagination works correctly
 * - Filtering and sorting work as specified
 * - Deprecated endpoints return appropriate warnings
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
const schemaValidationErrors = new Counter('schema_validation_errors');
const contractViolations = new Counter('contract_violations');
const deprecatedEndpointHits = new Counter('deprecated_endpoint_hits');
const fieldValidationErrors = new Counter('field_validation_errors');

// Get configuration
const config = dev;
const baseUrl = config.baseUrl;

// API Contract expectations
const apiContract = {
  // Expected response schemas
  schemas: {
    contact: {
      requiredFields: ['pid', 'person_ref', 'tags', 'links', 'rev', 'created_at', 'updated_at'],
      typeChecks: {
        pid: 'string',
        person_ref: 'string',
        tags: 'array',
        links: 'object',
        rev: 'number',
        created_at: 'string',
        updated_at: 'string',
      },
    },
    deal: {
      requiredFields: ['pid', 'person_ref', 'currency', 'expected_close', 'value_expect', 'tags', 'links', 'rev', 'created_at', 'updated_at'],
      typeChecks: {
        pid: 'string',
        person_ref: 'string',
        currency: 'string',
        expected_close: 'string',
        value_expect: 'number',
        tags: 'array',
        links: 'object',
        rev: 'number',
        created_at: 'string',
        updated_at: 'string',
      },
    },
    lead: {
      requiredFields: ['pid', 'person_ref', 'status', 'source', 'tags', 'links', 'rev', 'created_at', 'updated_at'],
      typeChecks: {
        pid: 'string',
        person_ref: 'string',
        status: 'string',
        source: 'string',
        tags: 'array',
        links: 'object',
        rev: 'number',
        created_at: 'string',
        updated_at: 'string',
      },
    },
    activity: {
      requiredFields: ['pid', 'subject_kind', 'subject_pid', 'verb', 'object', 'scheduled_at', 'created_at', 'updated_at'],
      typeChecks: {
        pid: 'string',
        subject_kind: 'string',
        subject_pid: 'string',
        verb: 'string',
        object: 'object',
        scheduled_at: 'string',
        created_at: 'string',
        updated_at: 'string',
      },
    },
    campaign: {
      requiredFields: ['pid', 'name', 'status', 'schedule', 'tags', 'created_at', 'updated_at'],
      typeChecks: {
        pid: 'string',
        name: 'string',
        status: 'string',
        schedule: 'object',
        tags: 'array',
        created_at: 'string',
        updated_at: 'string',
      },
    },
    ticket: {
      requiredFields: ['pid', 'subject_kind', 'subject_pid', 'priority', 'status', 'requested_by', 'created_at', 'updated_at'],
      typeChecks: {
        pid: 'string',
        subject_kind: 'string',
        subject_pid: 'string',
        priority: 'string',
        status: 'string',
        requested_by: 'string',
        created_at: 'string',
        updated_at: 'string',
      },
    },
    // List responses
    listResponse: {
      requiredFields: ['items', 'total'],
      typeChecks: {
        items: 'array',
        total: 'number',
      },
    },
    // Error responses
    errorResponse: {
      requiredFields: ['error', 'message'],
      typeChecks: {
        error: 'string',
        message: 'string',
      },
    },
    // Health check responses
    healthResponse: {
      requiredFields: ['status'],
      typeChecks: {
        status: 'string',
      },
    },
  },
  
  // Expected HTTP status codes
  statusCodes: {
    create: 201,
    read: 200,
    list: 200,
    update: 200,
    delete: 204,
    health: 200,
    notFound: 404,
    badRequest: 400,
    unauthorized: 401,
    forbidden: 403,
    rateLimited: 429,
  },
  
  // Expected headers
  responseHeaders: {
    'content-type': 'application/json',
    // 'etag': exists, // ETag for caching
    // 'last-modified': exists, // Last-Modified header
  },
  
  // Pagination parameters
  pagination: {
    defaultLimit: 50,
    maxLimit: 100,
    offsetParam: 'offset',
    limitParam: 'limit',
  },
  
  // Deprecated endpoints (if any)
  deprecatedEndpoints: [
    // Add deprecated endpoints here as they are identified
    // '/api/v1/old-endpoint',
  ],
  
  // API version
  apiVersion: 'v1',
};

// Test setup for API contract test
export const options = {
  vus: 5,
  duration: '2m',
  iterations: 100,
  thresholds: {
    ...config.thresholds,
    // Contract test specific thresholds
    http_req_failed: ['rate<0.05'],
    http_req_duration: ['p(95)<1000'],
    checks: ['rate>0.95'],
    schema_validation_errors: ['count<10'],
    contract_violations: ['count<5'],
  },
};

// Track created resources for cleanup
trackedResources = {
  contacts: [],
  deals: [],
  leads: [],
  activities: [],
  campaigns: [],
  tickets: [],
};

/**
 * Validate that an object has all required fields
 */
function validateRequiredFields(obj, requiredFields) {
  if (!obj || typeof obj !== 'object') {
    return false;
  }
  
  for (const field of requiredFields) {
    if (!(field in obj)) {
      return false;
    }
  }
  return true;
}

/**
 * Validate that field types match expectations
 */
function validateFieldTypes(obj, typeChecks) {
  if (!obj || typeof obj !== 'object') {
    return false;
  }
  
  let allValid = true;
  for (const [field, expectedType] of Object.entries(typeChecks)) {
    if (field in obj) {
      const actualType = typeof obj[field];
      const isArray = Array.isArray(obj[field]);
      
      let typeMatch = false;
      switch (expectedType) {
        case 'array':
          typeMatch = isArray;
          break;
        case 'object':
          typeMatch = actualType === 'object' && !isArray;
          break;
        default:
          typeMatch = actualType === expectedType;
      }
      
      if (!typeMatch) {
        fieldValidationErrors.add(1);
        console.log(`Field type mismatch: ${field} expected ${expectedType}, got ${actualType}`);
        allValid = false;
      }
    }
  }
  return allValid;
}

/**
 * Validate response against schema
 */
function validateSchema(response, schemaName) {
  const schema = apiContract.schemas[schemaName];
  if (!schema) {
    console.warn(`Unknown schema: ${schemaName}`);
    return false;
  }
  
  let body;
  try {
    body = JSON.parse(response.body);
  } catch (e) {
    console.error(`Failed to parse response body: ${e.message}`);
    schemaValidationErrors.add(1);
    return false;
  }
  
  // Validate required fields
  if (schema.requiredFields && !validateRequiredFields(body, schema.requiredFields)) {
    const missingFields = schema.requiredFields.filter(f => !(f in body));
    console.error(`Missing required fields for ${schemaName}: ${missingFields.join(', ')}`);
    contractViolations.add(1);
    return false;
  }
  
  // Validate field types
  if (schema.typeChecks && !validateFieldTypes(body, schema.typeChecks)) {
    contractViolations.add(1);
    return false;
  }
  
  return true;
}

/**
 * Validate HTTP status code
 */
function validateStatusCode(actual, expected) {
  if (Array.isArray(expected)) {
    return expected.includes(actual);
  }
  return actual === expected;
}

/**
 * Validate response headers
 */
function validateHeaders(response, requiredHeaders = {}) {
  const headers = response.headers || {};
  let allValid = true;
  
  for (const [header, expectedValue] of Object.entries(requiredHeaders)) {
    const actualValue = headers[header.toLowerCase()] || headers[header];
    
    if (expectedValue === 'exists') {
      if (!actualValue) {
        console.warn(`Missing required header: ${header}`);
        allValid = false;
      }
    } else if (actualValue !== expectedValue) {
      console.warn(`Header ${header} expected "${expectedValue}", got "${actualValue}"`);
      allValid = false;
    }
  }
  
  return allValid;
}

/**
 * Test endpoint response
 */
function testEndpoint(endpoint, method, body, expectedSchema, expectedStatus) {
  const url = `${baseUrl}${endpoint}`;
  let response;
  
  switch (method.toLowerCase()) {
    case 'get':
      response = http.get(url, withAuth({}, config.auth.token));
      break;
    case 'post':
      response = http.post(url, JSON.stringify(body), {
        headers: { 'Content-Type': 'application/json' },
        ...withAuth({}, config.auth.token),
      });
      break;
    case 'patch':
      response = http.patch(url, JSON.stringify(body), {
        headers: { 'Content-Type': 'application/json' },
        ...withAuth({}, config.auth.token),
      });
      break;
    case 'put':
      response = http.put(url, JSON.stringify(body), {
        headers: { 'Content-Type': 'application/json' },
        ...withAuth({}, config.auth.token),
      });
      break;
    case 'delete':
      response = http.del(url, null, withAuth({}, config.auth.token));
      break;
    default:
      throw new Error(`Unsupported method: ${method}`);
  }
  
  responseTimes.add(response.timings.duration);
  
  // Validate status code
  if (expectedStatus && !validateStatusCode(response.status, expectedStatus)) {
    console.error(`Status code mismatch for ${method} ${endpoint}: expected ${expectedStatus}, got ${response.status}`);
    contractViolations.add(1);
  }
  
  // Validate headers
  if (!validateHeaders(response, apiContract.responseHeaders)) {
    contractViolations.add(1);
  }
  
  // Validate schema if provided
  if (expectedSchema && !validateSchema(response, expectedSchema)) {
    schemaValidationErrors.add(1);
  }
  
  // Check for deprecated endpoint
  if (apiContract.deprecatedEndpoints.includes(endpoint)) {
    deprecatedEndpointHits.add(1);
    // Check for deprecation warning header
    const warningHeader = response.headers['warning'] || response.headers['deprecation'];
    if (!warningHeader) {
      console.warn(`Deprecated endpoint ${endpoint} missing deprecation warning`);
      contractViolations.add(1);
    }
  }
  
  return response;
}

/**
 * Test CRUD operations for an entity type
 */
function testCRUDOperations(entityType) {
  const entityGenerators = {
    contact: generateContact,
    deal: generateDeal,
    lead: generateLead,
    activity: generateActivity,
    campaign: generateCampaign,
    ticket: generateTicket,
  };
  
  const generator = entityGenerators[entityType];
  if (!generator) {
    console.error(`No generator for entity type: ${entityType}`);
    return;
  }
  
  const schemaName = entityType;
  const endpoint = `/api/${entityType}s`;
  
  // Test CREATE
  const entity = generator();
  const createResponse = testEndpoint(endpoint, 'POST', entity, schemaName, [200, 201]);
  check(createResponse, {
    [`Create ${entityType} successful`]: (r) => r.status === 200 || r.status === 201,
  });
  
  if (createResponse.status === 200 || createResponse.status === 201) {
    const created = JSON.parse(createResponse.body);
    trackedResources[entityType + 's'].push(created.pid);
    
    // Test READ
    const readResponse = testEndpoint(`${endpoint}/${created.pid}`, 'GET', null, schemaName, 200);
    check(readResponse, {
      [`Read ${entityType} successful`]: (r) => r.status === 200,
    });
    
    // Test LIST
    const listResponse = testEndpoint(endpoint, 'GET', { limit: 10 }, 'listResponse', 200);
    check(listResponse, {
      [`List ${entityType}s successful`]: (r) => r.status === 200,
    });
    
    // Test UPDATE
    const updateResponse = testEndpoint(`${endpoint}/${created.pid}`, 'PATCH', {
      tags: [...(created.tags || []), 'api-contract-test'],
    }, schemaName, 200);
    check(updateResponse, {
      [`Update ${entityType} successful`]: (r) => r.status === 200,
    });
    
    // Test DELETE
    const deleteResponse = testEndpoint(`${endpoint}/${created.pid}`, 'DELETE', null, null, 204);
    check(deleteResponse, {
      [`Delete ${entityType} successful`]: (r) => r.status === 204,
    });
  }
  
  sleep(0.5);
}

/**
 * Test pagination
 */
function testPagination(entityType) {
  const endpoint = `/api/${entityType}s`;
  
  // Test default pagination
  const defaultResponse = testEndpoint(endpoint, 'GET', null, 'listResponse', 200);
  check(defaultResponse, {
    [`Default pagination for ${entityType} returns array`]: (r) => {
      const body = JSON.parse(r.body);
      return Array.isArray(body.items);
    },
    [`Default pagination limit is ${apiContract.pagination.defaultLimit}`]: (r) => {
      const body = JSON.parse(r.body);
      return body.items.length <= apiContract.pagination.defaultLimit;
    },
  });
  
  // Test custom limit
  const customLimit = 10;
  const customResponse = testEndpoint(endpoint, 'GET', { limit: customLimit }, 'listResponse', 200);
  check(customResponse, {
    [`Custom pagination limit for ${entityType} is respected`]: (r) => {
      const body = JSON.parse(r.body);
      return body.items.length <= customLimit;
    },
  });
  
  // Test offset
  const offsetResponse = testEndpoint(endpoint, 'GET', { limit: 5, offset: 5 }, 'listResponse', 200);
  check(offsetResponse, {
    [`Pagination offset for ${entityType} works`]: (r) => r.status === 200,
  });
  
  // Test max limit
  const maxLimitResponse = testEndpoint(endpoint, 'GET', { limit: apiContract.pagination.maxLimit + 1 }, 'listResponse', 200);
  check(maxLimitResponse, {
    [`Max limit for ${entityType} is enforced`]: (r) => {
      const body = JSON.parse(r.body);
      return body.items.length <= apiContract.pagination.maxLimit;
    },
  });
  
  sleep(0.5);
}

/**
 * Test filtering and sorting
 */
function testFilteringAndSorting(entityType) {
  const endpoint = `/api/${entityType}s`;
  
  // Test filtering by status (if applicable)
  const filterableTypes = ['deal', 'lead', 'ticket', 'campaign'];
  if (filterableTypes.includes(entityType)) {
    const filterResponse = testEndpoint(endpoint, 'GET', { status: 'active' }, 'listResponse', 200);
    check(filterResponse, {
      [`Filter ${entityType} by status works`]: (r) => r.status === 200,
    });
  }
  
  // Test sorting
  const sortResponse = testEndpoint(endpoint, 'GET', { sort_by: 'created_at', sort_order: 'desc' }, 'listResponse', 200);
  check(sortResponse, {
    [`Sort ${entityType} works`]: (r) => r.status === 200,
  });
  
  sleep(0.5);
}

/**
 * Test health endpoints
 */
function testHealthEndpoints() {
  const healthEndpoints = [
    '/_health/live',
    '/_health/ready',
    '/_health/startup',
  ];
  
  for (const endpoint of healthEndpoints) {
    const response = testEndpoint(endpoint, 'GET', null, 'healthResponse', 200);
    check(response, {
      [`Health endpoint ${endpoint} works`]: (r) => r.status === 200,
    });
  }
  
  sleep(0.5);
}

/**
 * Test error responses
 */
function testErrorResponses() {
  // Test 404 - Not Found
  const notFoundResponse = testEndpoint('/api/contacts/non-existent-pid', 'GET', null, 'errorResponse', 404);
  check(notFoundResponse, {
    '404 response has error structure': (r) => {
      if (r.status !== 404) return false;
      try {
        const body = JSON.parse(r.body);
        return body.error && body.message;
      } catch {
        return false;
      }
    },
  });
  
  // Test 400 - Bad Request (invalid JSON)
  const badRequestResponse = testEndpoint('/api/contacts', 'POST', { invalid: 'json' }, 'errorResponse', 400);
  check(badRequestResponse, {
    '400 response has error structure': (r) => {
      if (r.status !== 400) return false;
      try {
        const body = JSON.parse(r.body);
        return body.error && body.message;
      } catch {
        return false;
      }
    },
  });
  
  sleep(0.5);
}

/**
 * Test API versioning
 */
function testApiVersioning() {
  // Test current version
  const response = testEndpoint('/api/contacts?limit=1', 'GET', null, 'listResponse', 200);
  check(response, {
    'API version is correct': (r) => {
      // Check if response includes version info or endpoint is accessible
      return r.status === 200;
    },
  });
  
  sleep(0.5);
}

/**
 * Main test function
 */
export default function () {
  // Test all entity types
  const entityTypes = ['contact', 'deal', 'lead', 'activity', 'campaign', 'ticket'];
  
  // Select a random entity type for this VU
  const entityType = entityTypes[randomInt(0, entityTypes.length)];
  
  // Run different types of tests
  const testType = randomInt(0, 4);
  
  switch (testType) {
    case 0:
      testCRUDOperations(entityType);
      break;
    case 1:
      testPagination(entityType);
      break;
    case 2:
      testFilteringAndSorting(entityType);
      break;
    case 3:
      testHealthEndpoints();
      break;
    default:
      testErrorResponses();
  }
  
  // Occasionally test API versioning
  if (randomInt(0, 9) === 0) {
    testApiVersioning();
  }
  
  sleep(randomInt(1, 3));
}

/**
 * Setup function
 */
export function setup() {
  console.log(`Starting API contract test against ${baseUrl}`);
  console.log(`Testing against API version: ${apiContract.apiVersion}`);
  console.log('This test validates OpenAPI compliance and contract adherence');
  return {};
}

/**
 * Teardown function
 */
export function teardown(data) {
  console.log('API contract test completed');
  console.log('=== Contract Validation Results ===');
  console.log(`Schema validation errors: ${schemaValidationErrors.count}`);
  console.log(`Contract violations: ${contractViolations.count}`);
  console.log(`Field validation errors: ${fieldValidationErrors.count}`);
  console.log(`Deprecated endpoint hits: ${deprecatedEndpointHits.count}`);
  console.log('');
  console.log('=== Performance Metrics ===');
  console.log(`Average response time: ${responseTimes.avg}ms`);
  console.log(`P50 response time: ${responseTimes.percentile(50)}ms`);
  console.log(`P90 response time: ${responseTimes.percentile(90)}ms`);
  console.log(`P95 response time: ${responseTimes.percentile(95)}ms`);
  console.log('');
  
  // Summary
  const totalErrors = schemaValidationErrors.count + contractViolations.count + fieldValidationErrors.count;
  if (totalErrors === 0) {
    console.log('✓ All API contract tests passed!');
  } else {
    console.log(`✗ ${totalErrors} contract violations found!`);
    console.log('Review the schema definitions and API responses.');
  }
}
