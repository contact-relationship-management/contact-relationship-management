// Base configuration for all load tests
// Extend this in environment-specific configs

/**
 * Base load test configuration
 * All environments should extend this
 */
export const base = {
  // Base URL for the CRM service
  // Override with CRM_BASE_URL environment variable or in env-specific config
  baseUrl: process.env.CRM_BASE_URL || 'http://localhost:1241',

  // Database URL (for tests that need direct DB access)
  dbUrl: process.env.DB_URL || 'postgres://loco:loco@localhost:5432/contact_relationship_management_test',

  // Test durations (in seconds or time strings like '10m', '1h')
  durations: {
    smoke: '1m',
    short: '5m',
    medium: '15m',
    long: '30m',
    soak: '1h',
    extendedSoak: '24h',
  },

  // Virtual User settings
  vus: {
    smoke: 1,
    min: 1,
    max: 10,
    stressMax: 500,
    soak: 20,
  },

  // Ramp-up/down settings
  stages: {
    // Quick ramp for smoke tests
    smoke: [
      { duration: '10s', target: 1 },
      { duration: '30s', target: 1 },
      { duration: '10s', target: 0 },
    ],
    
    // Gradual ramp for load tests
    normal: [
      { duration: '1m', target: 10 },
      { duration: '2m', target: 50 },
      { duration: '2m', target: 100 },
      { duration: '5m', target: 100 },
      { duration: '1m', target: 0 },
    ],
    
    // Aggressive ramp for stress tests
    stress: [
      { duration: '30s', target: 50 },
      { duration: '1m', target: 200 },
      { duration: '2m', target: 500 },
      { duration: '5m', target: 500 },
      { duration: '1m', target: 0 },
    ],
    
    // Soak test: ramp up, sustain, ramp down
    soak: [
      { duration: '2m', target: 50 },
      { duration: '28m', target: 50 },
      { duration: '2m', target: 0 },
    ],
    
    // Extended soak: longer sustain period
    extendedSoak: [
      { duration: '5m', target: 100 },
      { duration: '23h', target: 100 },
      { duration: '5m', target: 0 },
    ],
    
    // Spike test: sudden surge
    spike: [
      { duration: '1m', target: 10 },
      { duration: '10s', target: 1000 },
      { duration: '1m', target: 10 },
      { duration: '30s', target: 0 },
    ],
  },

  // HTTP timeouts
  timeouts: {
    request: '30s',
    connection: '10s',
    response: '30s',
  },

  // Retry settings
  retries: {
    max: 3,
    backoff: '1s',
  },

  // Thresholds (pass/fail criteria)
  thresholds: {
    // HTTP errors should be < 1%
    http_req_failed: ['rate<0.01'],
    
    // 95% of requests should be below 500ms
    http_req_duration: ['p(95)<500'],
    
    // 99% of requests should be below 1000ms
    http_req_duration: ['p(99)<1000'],
    
    // Average response time should be below 200ms
    http_req_duration: ['avg<200'],
    
    // All checks should pass
    checks: ['rate>=0.99'],
  },

  // Authentication
  auth: {
    // Set to false for testing without auth
    // Set to true and provide token for authenticated tests
    requireAuth: process.env.CRM_REQUIRE_AUTH === '1',
    
    // Token for authenticated requests
    // Can be set via CRM_TOKEN environment variable
    token: process.env.CRM_TOKEN || null,
    
    // Token header name
    tokenHeader: 'Authorization',
    
    // Token prefix
    tokenPrefix: 'Bearer',
  },

  // Test data settings
  testData: {
    // Number of test records to create per test
    contactsPerTest: 10,
    dealsPerTest: 5,
    campaignsPerTest: 2,
    
    // Maximum number of records in database
    maxContacts: 10000,
    maxDeals: 5000,
    maxCampaigns: 100,
    
    // Cleanup after tests
    cleanup: true,
  },

  // Database settings
  database: {
    // Connection pool size
    poolSize: 10,
    
    // Maximum connections
    maxConnections: 20,
    
    // Test database name
    testDbName: 'contact_relationship_management_test',
  },

  // Redis settings
  redis: {
    url: process.env.CRM_REDIS_URL || 'redis://localhost:6379',
    
    // Cache TTL
    ttl: 30,
  },

  // Rate limiting
  rateLimiting: {
    // Max requests per second per VU
    maxRps: 10,
    
    // Enable rate limiting
    enabled: true,
  },

  // Logging
  logging: {
    // Log level: debug, info, warn, error
    level: 'info',
    
    // Log HTTP requests
    logRequests: false,
    
    // Log HTTP responses
    logResponses: false,
  },

  // Output
  output: {
    // Output directory for results
    dir: './results',
    
    // Output formats: json, csv, influxdb, prometheus, cloud
    formats: ['json'],
    
    // Keep old results
    keepOldResults: true,
    
    // Max results to keep
    maxResults: 10,
  },

  // Environment detection
  get environment() {
    if (process.env.CI) return 'ci';
    if (process.env.NODE_ENV === 'production') return 'production';
    if (process.env.NODE_ENV === 'staging') return 'staging';
    return 'development';
  },

  // Helper to get environment-specific config
  getEnvConfig() {
    const env = this.environment;
    try {
      // Dynamic import of environment-specific config
      // eslint-disable-next-line @typescript-eslint/no-var-requires, @typescript-eslint/no-require-imports
      const envConfig = require(`./${env}.js`).default || require(`./${env}.js`);
      return { ...this, ...envConfig };
    } catch {
      // If no environment-specific config, return base
      return this;
    }
  },
};

// Export default for compatibility
export default base;
