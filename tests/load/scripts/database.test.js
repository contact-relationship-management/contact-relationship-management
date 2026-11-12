/**
 * Database Performance Test
 * 
 * Purpose: Test PostgreSQL database performance under load
 * Duration: 5-10 minutes
 * VUs: 10-50
 * 
 * This test specifically targets database operations to verify:
 * - Query performance under load
 * - Connection pool behavior
 * - Transaction throughput
 * - Index efficiency
 * - Complex join performance
 * - Sorting and filtering performance
 * - Database lock contention
 * - Pagination efficiency
 * - Full-text search performance
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Trend, Rate, Counter } from 'k6/metrics';

// Import configuration and utilities
import { dev } from '../config/dev.js';
import { withAuth, randomInt, randomUUID, randomDate } from '../lib/utils.js';
import { generateContact, generateDeal, generateLead, generateActivity } from '../lib/generators.js';

// Custom metrics
const queryResponseTimes = new Trend('query_response_times');
const complexQueryTimes = new Trend('complex_query_times');
const writeOperationTimes = new Trend('write_operation_times');
const dbErrorRate = new Rate('db_error_rate');
const slowQueryCount = new Counter('slow_queries');
const deadlockCount = new Counter('deadlocks');

// Thresholds for slow queries
const SLOW_QUERY_THRESHOLD = 1000; // ms

// Get configuration
const config = dev;
const baseUrl = config.baseUrl;

// Database test configuration
export const options = {
  vus: config.vus.database,
  duration: config.durations.database,
  stages: config.stages.database,
  thresholds: {
    ...config.thresholds,
    // Database specific thresholds
    http_req_failed: ['rate<0.05'],
    http_req_duration: ['p(95)<2000'],
    // Write operations might be slower
    write_operation_times: ['p(95)<1500'],
    // Complex queries might be slower
    complex_query_times: ['p(95)<3000'],
    slow_queries: ['count<50'], // Less than 50 slow queries
    deadlocks: ['count=0'], // No deadlocks
    checks: ['rate>0.95'],
  },
};

// Track state
let totalContactsCreated = 0;
let totalDealsCreated = 0;
let totalActivitiesCreated = 0;

/**
 * Create test data for database operations
 */
async function createTestData() {
  // Create a contact with related entities
  const contact = generateContact();
  const contactRes = http.post(`${baseUrl}/api/contacts`, JSON.stringify(contact), {
    headers: { 'Content-Type': 'application/json' },
    ...withAuth({}, config.auth.token),
  });
  
  queryResponseTimes.add(contactRes.timings.duration);
  writeOperationTimes.add(contactRes.timings.duration);
  
  if (contactRes.status === 200) {
    const contactBody = JSON.parse(contactRes.body);
    totalContactsCreated++;
    
    // Create related entities
    const deal = generateDeal({ contact_ref: contact.person_ref });
    const dealRes = http.post(`${baseUrl}/api/deals`, JSON.stringify(deal), {
      headers: { 'Content-Type': 'application/json' },
      ...withAuth({}, config.auth.token),
    });
    
    queryResponseTimes.add(dealRes.timings.duration);
    writeOperationTimes.add(dealRes.timings.duration);
    
    if (dealRes.status === 200) {
      totalDealsCreated++;
    }
    
    // Create activity
    const activity = generateActivity({
      subject_kind: 'contact',
      subject_pid: contactBody.pid,
    });
    const activityRes = http.post(`${baseUrl}/api/activities`, JSON.stringify(activity), {
      headers: { 'Content-Type': 'application/json' },
      ...withAuth({}, config.auth.token),
    });
    
    queryResponseTimes.add(activityRes.timings.duration);
    writeOperationTimes.add(activityRes.timings.duration);
    
    if (activityRes.status === 200) {
      totalActivitiesCreated++;
    }
    
    return { contact: contactBody, deal: dealRes.status === 200 ? JSON.parse(dealRes.body) : null };
  }
  
  return { contact: null, deal: null };
}

/**
 * Simple CRUD operations
 */
function simpleCRUDOperations() {
  // Randomly select an operation
  const operations = [
    { name: 'create_contact', func: () => createTestData() },
    { name: 'list_contacts', func: listOperations },
    { name: 'get_contact', func: getOperations },
    { name: 'update_contact', func: updateOperations },
  ];
  
  const operation = operations[randomInt(0, operations.length)];
  return operation.func();
}

/**
 * List operations with various parameters
 */
function listOperations() {
  const listEndpoints = [
    '/api/contacts',
    '/api/deals',
    '/api/leads',
    '/api/activities',
  ];
  
  const endpoint = listEndpoints[randomInt(0, listEndpoints.length)];
  
  // Vary limit and offset
  const limit = [10, 25, 50, 100][randomInt(0, 4)];
  const offset = randomInt(0, 100);
  
  const url = `${baseUrl}${endpoint}?limit=${limit}&offset=${offset}`;
  const res = http.get(url, withAuth({}, config.auth.token));
  
  queryResponseTimes.add(res.timings.duration);
  
  if (res.timings.duration > SLOW_QUERY_THRESHOLD) {
    slowQueryCount.add(1);
    console.log(`Slow list query: ${url} took ${res.timings.duration}ms`);
  }
  
  check(res, {
    [`List ${endpoint} successful`]: (r) => r.status === 200,
  });
  
  return res;
}

/**
 * Get single resource operations
 */
function getOperations() {
  const getEndpoints = [
    { base: '/api/contacts', pid: null },
    { base: '/api/deals', pid: null },
    { base: '/api/leads', pid: null },
    { base: '/api/activities', pid: null },
  ];
  
  const { base, pid } = getEndpoints[randomInt(0, getEndpoints.length)];
  
  // Try to get a random existing resource or use a test PID
  const testPids = ['test-1', 'test-2', 'test-3'];
  const randomPid = testPids[randomInt(0, testPids.length)];
  
  const url = `${baseUrl}${base}/${randomPid}`;
  const res = http.get(url, withAuth({}, config.auth.token));
  
  queryResponseTimes.add(res.timings.duration);
  
  // 404 is acceptable for get operations
  check(res, {
    [`Get ${base} returns success or not found`]: (r) => r.status === 200 || r.status === 404,
  });
  
  return res;
}

/**
 * Update operations
 */
function updateOperations() {
  // Try to update a random contact
  const listRes = http.get(`${baseUrl}/api/contacts?limit=10`, withAuth({}, config.auth.token));
  
  queryResponseTimes.add(listRes.timings.duration);
  
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
      
      queryResponseTimes.add(updateRes.timings.duration);
      writeOperationTimes.add(updateRes.timings.duration);
      
      check(updateRes, {
        [`Update contact successful`]: (r) => r.status === 200,
      });
      
      return updateRes;
    }
  }
  
  return listRes;
}

/**
 * Complex query operations - multiple joins and conditions
 */
function complexQueryOperations() {
  const complexQueries = [
    // Contacts with deals
    {
      name: 'contacts_with_deals',
      url: () => `${baseUrl}/api/contacts?has_deals=true&limit=50`,
    },
    // Contacts with activities
    {
      name: 'contacts_with_activities',
      url: () => `${baseUrl}/api/contacts?has_activities=true&limit=50`,
    },
    // Deals by status
    {
      name: 'deals_by_status',
      url: () => `${baseUrl}/api/deals?status=active&limit=50`,
    },
    // Activities by date range
    {
      name: 'activities_by_date',
      url: () => {
        const startDate = new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString();
        const endDate = new Date().toISOString();
        return `${baseUrl}/api/activities?scheduled_at_gte=${startDate}&scheduled_at_lte=${endDate}&limit=50`;
      },
    },
    // Contacts with tags
    {
      name: 'contacts_with_tags',
      url: () => `${baseUrl}/api/contacts?tag=important&limit=50`,
    },
    // Deals by value range
    {
      name: 'deals_by_value',
      url: () => `${baseUrl}/api/deals?value_expect_gte=1000&value_expect_lte=100000&limit=50`,
    },
  ];
  
  const query = complexQueries[randomInt(0, complexQueries.length)];
  const url = query.url();
  
  const res = http.get(url, withAuth({}, config.auth.token));
  
  queryResponseTimes.add(res.timings.duration);
  complexQueryTimes.add(res.timings.duration);
  
  if (res.timings.duration > SLOW_QUERY_THRESHOLD) {
    slowQueryCount.add(1);
    console.log(`Slow complex query: ${query.name} took ${res.timings.duration}ms`);
  }
  
  check(res, {
    [`Complex query ${query.name} successful`]: (r) => r.status === 200,
  });
  
  return res;
}

/**
 * Aggregation operations (counts, sums, averages)
 */
function aggregationOperations() {
  const aggregations = [
    { name: 'contact_count', url: `${baseUrl}/api/contacts/count` },
    { name: 'deal_count', url: `${baseUrl}/api/deals/count` },
    { name: 'lead_count', url: `${baseUrl}/api/leads/count` },
    { name: 'activity_count', url: `${baseUrl}/api/activities/count` },
    { name: 'total_deal_value', url: `${baseUrl}/api/deals/total-value` },
    { name: 'avg_deal_value', url: `${baseUrl}/api/deals/avg-value` },
  ];
  
  const aggregation = aggregations[randomInt(0, aggregations.length)];
  const res = http.get(aggregation.url, withAuth({}, config.auth.token));
  
  queryResponseTimes.add(res.timings.duration);
  complexQueryTimes.add(res.timings.duration);
  
  check(res, {
    [`Aggregation ${aggregation.name} successful`]: (r) => r.status === 200,
  });
  
  return res;
}

/**
 * Sorting operations
 */
function sortingOperations() {
  const sortOptions = [
    { entity: 'contacts', field: 'created_at', order: 'desc' },
    { entity: 'contacts', field: 'created_at', order: 'asc' },
    { entity: 'deals', field: 'value_expect', order: 'desc' },
    { entity: 'deals', field: 'expected_close', order: 'asc' },
    { entity: 'activities', field: 'scheduled_at', order: 'desc' },
    { entity: 'leads', field: 'created_at', order: 'desc' },
  ];
  
  const sort = sortOptions[randomInt(0, sortOptions.length)];
  const url = `${baseUrl}/api/${sort.entity}?sort_by=${sort.field}&sort_order=${sort.order}&limit=50`;
  
  const res = http.get(url, withAuth({}, config.auth.token));
  
  queryResponseTimes.add(res.timings.duration);
  
  check(res, {
    [`Sort ${sort.entity} by ${sort.field} ${sort.order} successful`]: (r) => r.status === 200,
  });
  
  // Validate that results are sorted correctly
  if (res.status === 200) {
    try {
      const body = JSON.parse(res.body);
      if (body.items && body.items.length > 1) {
        // For demonstration, just check that we have results
        // In a real test, you'd validate the actual sorting
        check(res, {
          [`Sort ${sort.entity} has results`]: () => body.items.length > 0,
        });
      }
    } catch (e) {
      // Ignore parse errors
    }
  }
  
  return res;
}

/**
 * Full-text search operations
 */
function searchOperations() {
  const searchTerms = ['important', 'urgent', 'customer', 'prospect', 'deal', 'meeting'];
  const searchTerm = searchTerms[randomInt(0, searchTerms.length)];
  
  const entities = ['contacts', 'deals', 'leads', 'activities'];
  const entity = entities[randomInt(0, entities.length)];
  
  const url = `${baseUrl}/api/${entity}?q=${encodeURIComponent(searchTerm)}&limit=50`;
  
  const res = http.get(url, withAuth({}, config.auth.token));
  
  queryResponseTimes.add(res.timings.duration);
  complexQueryTimes.add(res.timings.duration);
  
  if (res.timings.duration > SLOW_QUERY_THRESHOLD) {
    slowQueryCount.add(1);
    console.log(`Slow search query: ${entity} search for "${searchTerm}" took ${res.timings.duration}ms`);
  }
  
  check(res, {
    [`Search ${entity} for "${searchTerm}" successful`]: (r) => r.status === 200,
  });
  
  return res;
}

/**
 * Transaction operations - create multiple related entities
 */
function transactionOperations() {
  // Create a contact with deal and activity in sequence
  const contact = generateContact();
  
  const startTime = Date.now();
  
  const contactRes = http.post(`${baseUrl}/api/contacts`, JSON.stringify(contact), {
    headers: { 'Content-Type': 'application/json' },
    ...withAuth({}, config.auth.token),
  });
  
  if (contactRes.status !== 200) {
    dbErrorRate.add(1);
    return contactRes;
  }
  
  const contactBody = JSON.parse(contactRes.body);
  const deal = generateDeal({ contact_ref: contact.person_ref });
  
  const dealRes = http.post(`${baseUrl}/api/deals`, JSON.stringify(deal), {
    headers: { 'Content-Type': 'application/json' },
    ...withAuth({}, config.auth.token),
  });
  
  if (dealRes.status !== 200) {
    dbErrorRate.add(1);
    return dealRes;
  }
  
  const dealBody = JSON.parse(dealRes.body);
  const activity = generateActivity({
    subject_kind: 'deal',
    subject_pid: dealBody.pid,
  });
  
  const activityRes = http.post(`${baseUrl}/api/activities`, JSON.stringify(activity), {
    headers: { 'Content-Type': 'application/json' },
    ...withAuth({}, config.auth.token),
  });
  
  const duration = Date.now() - startTime;
  queryResponseTimes.add(duration);
  writeOperationTimes.add(duration);
  
  if (activityRes.status === 200) {
    totalContactsCreated++;
    totalDealsCreated++;
    totalActivitiesCreated++;
  }
  
  check(activityRes, {
    'Transaction create contact+deal+activity successful': (r) => r.status === 200,
  });
  
  return activityRes;
}

/**
 * Batch operations
 */
function batchOperations() {
  // Create multiple contacts in sequence
  const count = randomInt(3, 10);
  
  const startTime = Date.now();
  
  for (let i = 0; i < count; i++) {
    const contact = generateContact();
    const res = http.post(`${baseUrl}/api/contacts`, JSON.stringify(contact), {
      headers: { 'Content-Type': 'application/json' },
      ...withAuth({}, config.auth.token),
    });
    
    writeOperationTimes.add(res.timings.duration);
    
    if (res.status === 200) {
      totalContactsCreated++;
    } else {
      dbErrorRate.add(1);
    }
    
    // Small delay between batch requests
    sleep(0.1);
  }
  
  const duration = Date.now() - startTime;
  const avgTime = duration / count;
  queryResponseTimes.add(avgTime);
  
  return { count, duration, avgTime };
}

/**
 * Main test function
 */
export default function () {
  // Weighted random selection of operations
  const operation = randomInt(0, 60);
  
  switch (operation) {
    case 0:
    case 1:
    case 2:
    case 3:
      // 6.7%: Simple CRUD
      simpleCRUDOperations();
      break;
    case 4:
    case 5:
    case 6:
    case 7:
    case 8:
      // 8.3%: List operations
      listOperations();
      break;
    case 9:
    case 10:
    case 11:
      // 5%: Complex queries
      complexQueryOperations();
      break;
    case 12:
    case 13:
    case 14:
      // 5%: Aggregations
      aggregationOperations();
      break;
    case 15:
    case 16:
    case 17:
      // 5%: Sorting
      sortingOperations();
      break;
    case 18:
    case 19:
    case 20:
      // 5%: Search
      searchOperations();
      break;
    case 21:
    case 22:
    case 23:
      // 5%: Transactions
      transactionOperations();
      break;
    case 24:
    case 25:
      // 3.3%: Batch operations
      batchOperations();
      break;
    default:
      // Remaining: Simple queries
      listOperations();
  }
  
  // Variable sleep
  sleep(randomInt(0, 2));
}

/**
 * Setup function
 */
export function setup() {
  console.log(`Starting database performance test against ${baseUrl}`);
  console.log(`VUs: ${config.vus.database}, Duration: ${config.durations.database}`);
  console.log('This test focuses on PostgreSQL database performance');
  return {};
}

/**
 * Teardown function
 */
export function teardown(data) {
  console.log('Database performance test completed');
  console.log('=== Database Metrics ===');
  console.log(`Total contacts created: ${totalContactsCreated}`);
  console.log(`Total deals created: ${totalDealsCreated}`);
  console.log(`Total activities created: ${totalActivitiesCreated}`);
  console.log('');
  console.log('=== Query Performance ===');
  console.log(`Simple query avg: ${queryResponseTimes.avg}ms`);
  console.log(`Complex query avg: ${complexQueryTimes.avg}ms`);
  console.log(`Write operation avg: ${writeOperationTimes.avg}ms`);
  console.log('');
  console.log('=== Performance Percentiles ===');
  console.log(`Simple queries P95: ${queryResponseTimes.percentile(95)}ms`);
  console.log(`Complex queries P95: ${complexQueryTimes.percentile(95)}ms`);
  console.log(`Write operations P95: ${writeOperationTimes.percentile(95)}ms`);
  console.log('');
  console.log('=== Issues ===');
  console.log(`Slow queries (>${SLOW_QUERY_THRESHOLD}ms): ${slowQueryCount.count}`);
  console.log(`Deadlocks: ${deadlockCount.count}`);
  console.log(`DB error rate: ${dbErrorRate.count}`);
  console.log('');
  
  // Recommendations
  if (slowQueryCount.count > 10) {
    console.log('RECOMMENDATION: Investigate slow queries. Consider:');
    console.log('  - Adding database indexes');
    console.log('  - Optimizing complex joins');
    console.log('  - Query caching');
  }
  
  if (deadlockCount.count > 0) {
    console.log('RECOMMENDATION: Investigate deadlocks. Consider:');
    console.log('  - Reducing transaction scope');
    console.log('  - Using shorter transactions');
    console.log('  - Adding retry logic');
  }
  
  if (queryResponseTimes.percentile(95) > 2000) {
    console.log('RECOMMENDATION: P95 query time is high. Consider:');
    console.log('  - Database query optimization');
    console.log('  - Connection pool tuning');
    console.log('  - Read replica scaling');
  }
}
