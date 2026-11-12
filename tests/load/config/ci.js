// CI environment configuration
// Extends base.js with CI-specific settings

import { base } from './base.js';

/**
 * CI environment configuration
 * Used when running tests in CI/CD pipelines
 * 
 * Optimized for:
 * - Faster execution (shorter durations)
 * - Deterministic results (fixed VU counts)
 * - Clean state (always cleanup)
 */
export const ci = {
  ...base,
  
  // CI-specific base URL (usually localhost in CI)
  baseUrl: process.env.CRM_BASE_URL || 'http://localhost:1241',
  
  // Fixed VU counts for deterministic CI results
  vus: {
    ...base.vus,
    smoke: 1,
    min: 5,
    max: 20,
    stressMax: 50,
    soak: 10,
  },
  
  // Shorter durations for CI (fast feedback)
  durations: {
    smoke: '1m',
    short: '3m',
    medium: '5m',
    long: '10m',
    soak: '5m', // Short soak for CI
    extendedSoak: '30m',
  },
  
  // Authentication: use test token if available
  auth: {
    ...base.auth,
    requireAuth: process.env.CRM_REQUIRE_AUTH === '1',
    token: process.env.CRM_TOKEN || null,
  },
  
  // Database: use test database in CI
  dbUrl: process.env.DB_URL || 'postgres://loco:loco@localhost:5432/contact_relationship_management_test',
  
  // Minimal logging in CI (only warnings and errors)
  logging: {
    ...base.logging,
    level: 'warn',
    logRequests: false,
    logResponses: false,
  },
  
  // Store results in CI directory
  output: {
    ...base.output,
    dir: './results/ci',
  },
  
  // Strict thresholds in CI
  thresholds: {
    ...base.thresholds,
    http_req_failed: ['rate<0.01'], // No errors allowed in CI
    http_req_duration: ['p(95)<500'], // P95 must be < 500ms
    http_req_duration: ['p(99)<1000'], // P99 must be < 1000ms
    checks: ['rate>=1.0'], // All checks must pass
  },
  
  // Always cleanup in CI
  testData: {
    ...base.testData,
    cleanup: true,
  },
  
  // CI-specific stages with faster ramp-up
  stages: {
    ...base.stages,
    normal: [
      { duration: '30s', target: 10 },
      { duration: '1m', target: 20 },
      { duration: '2m', target: 20 },
      { duration: '30s', target: 0 },
    ],
    stress: [
      { duration: '20s', target: 10 },
      { duration: '30s', target: 50 },
      { duration: '1m', target: 50 },
      { duration: '20s', target: 0 },
    ],
    soak: [
      { duration: '1m', target: 10 },
      { duration: '3m', target: 10 },
      { duration: '1m', target: 0 },
    ],
  },
};

export default ci;
