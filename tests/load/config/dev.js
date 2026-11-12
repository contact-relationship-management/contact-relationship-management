// Development environment configuration
// Extends base.js with dev-specific settings

import { base } from './base.js';

/**
 * Development environment configuration
 * Used when running tests locally
 */
export const dev = {
  ...base,
  
  // Development-specific base URL
  baseUrl: process.env.CRM_BASE_URL || 'http://localhost:1241',
  
  // Lower VU counts for local testing
  vus: {
    ...base.vus,
    smoke: 1,
    min: 1,
    max: 10,
    stressMax: 50,
    soak: 5,
  },
  
  // Shorter durations for development
  durations: {
    smoke: '30s',
    short: '2m',
    medium: '5m',
    long: '10m',
    soak: '10m',
    extendedSoak: '1h',
  },
  
  // Authentication: typically off in development
  auth: {
    ...base.auth,
    requireAuth: false, // Development usually has auth off
  },
  
  // Database: use local test database
  dbUrl: process.env.DB_URL || 'postgres://loco:loco@localhost:5432/contact_relationship_management_development',
  
  // More verbose logging in development
  logging: {
    ...base.logging,
    level: 'debug',
    logRequests: true,
    logResponses: false,
  },
  
  // Store results in dev directory
  output: {
    ...base.output,
    dir: './results/dev',
  },
  
  // Relax thresholds in development
  thresholds: {
    ...base.thresholds,
    http_req_failed: ['rate<0.05'], // Allow up to 5% errors in dev
    http_req_duration: ['p(95)<1000'], // Allow up to 1s P95 in dev
  },
};

export default dev;
