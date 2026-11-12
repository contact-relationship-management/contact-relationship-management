// Utility functions for load testing

/**
 * Generate a random string of given length
 * @param {number} length - Length of string
 * @returns {string} Random string
 */
export function randomString(length = 10) {
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
  let result = '';
  for (let i = 0; i < length; i++) {
    result += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return result;
}

/**
 * Generate a random UUID v4
 * @returns {string} UUID v4 string
 */
export function randomUUID() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

/**
 * Generate a random email address
 * @param {string} prefix - Optional prefix
 * @returns {string} Random email
 */
export function randomEmail(prefix = '') {
  return `${prefix}${randomString(8)}@example.com`;
}

/**
 * Generate a random phone number
 * @returns {string} Random phone number
 */
export function randomPhone() {
  return `+4477009${Math.floor(Math.random() * 100000).toString().padStart(5, '0')}`;
}

/**
 * Generate a random date within a range
 * @param {Date} start - Start date
 * @param {Date} end - End date
 * @returns {string} ISO date string
 */
export function randomDate(start, end) {
  const startTime = start.getTime();
  const endTime = end.getTime();
  const randomTime = startTime + Math.random() * (endTime - startTime);
  return new Date(randomTime).toISOString().split('T')[0];
}

/**
 * Generate a random date in the past
 * @param {number} maxDays - Maximum days in the past
 * @returns {string} ISO date string
 */
export function randomPastDate(maxDays = 365) {
  const end = new Date();
  const start = new Date();
  start.setDate(start.getDate() - maxDays);
  return randomDate(start, end);
}

/**
 * Generate a random date in the future
 * @param {number} maxDays - Maximum days in the future
 * @returns {string} ISO date string
 */
export function randomFutureDate(maxDays = 365) {
  const start = new Date();
  const end = new Date();
  end.setDate(end.getDate() + maxDays);
  return randomDate(start, end);
}

/**
 * Generate a random amount in minor units (e.g., cents, pence)
 * @param {number} min - Minimum amount
 * @param {number} max - Maximum amount
 * @returns {number} Random amount in minor units
 */
export function randomAmount(min = 100, max = 1000000) {
  return Math.floor(min + Math.random() * (max - min));
}

/**
 * Generate a random ISO 4217 currency code
 * @returns {string} Currency code
 */
export function randomCurrency() {
  const currencies = ['USD', 'GBP', 'EUR', 'JPY', 'CAD', 'AUD', 'CHF', 'CNY'];
  return currencies[Math.floor(Math.random() * currencies.length)];
}

/**
 * Sleep for a specified duration
 * @param {number} ms - Milliseconds to sleep
 * @returns {Promise<void>}
 */
export async function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Retry a function with exponential backoff
 * @param {Function} fn - Function to retry
 * @param {number} maxRetries - Maximum number of retries
 * @param {number} baseDelay - Base delay in ms
 * @returns {Promise<any>} Result of the function
 */
export async function retry(fn, maxRetries = 3, baseDelay = 1000) {
  let lastError;
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error;
      if (i < maxRetries - 1) {
        const delay = baseDelay * Math.pow(2, i);
        console.warn(`Retry ${i + 1}/${maxRetries} after ${delay}ms: ${error.message}`);
        await sleep(delay);
      }
    }
  }
  throw lastError;
}

/**
 * Check if a response is successful
 * @param {import('k6/http').Response} res - k6 HTTP response
 * @param {number} expectedStatus - Expected status code
 * @returns {boolean} True if successful
 */
export function isSuccess(res, expectedStatus = 200) {
  return res.status === expectedStatus;
}

/**
 * Check if a response indicates a client error
 * @param {import('k6/http').Response} res - k6 HTTP response
 * @returns {boolean} True if client error
 */
export function isClientError(res) {
  return res.status >= 400 && res.status < 500;
}

/**
 * Check if a response indicates a server error
 * @param {import('k6/http').Response} res - k6 HTTP response
 * @returns {boolean} True if server error
 */
export function isServerError(res) {
  return res.status >= 500;
}

/**
 * Generate a random person reference (URN)
 * @returns {string} person: UUID
 */
export function randomPersonRef() {
  return `person:${randomUUID()}`;
}

/**
 * Generate a random organization reference (URN)
 * @returns {string} organization: UUID
 */
export function randomOrganizationRef() {
  return `organization:${randomUUID()}`;
}

/**
 * Generate a random item from an array
 * @param {Array} array - Array to select from
 * @returns {*} Random item
 */
export function randomItem(array) {
  return array[Math.floor(Math.random() * array.length)];
}

/**
 * Shuffle an array (Fisher-Yates algorithm)
 * @param {Array} array - Array to shuffle
 * @returns {Array} Shuffled array
 */
export function shuffle(array) {
  const result = [...array];
  for (let i = result.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [result[i], result[j]] = [result[j], result[i]];
  }
  return result;
}

/**
 * Generate a random boolean
 * @param {number} probability - Probability of true (0-1)
 * @returns {boolean} Random boolean
 */
export function randomBoolean(probability = 0.5) {
  return Math.random() < probability;
}

/**
 * Generate a random integer within a range
 * @param {number} min - Minimum value (inclusive)
 * @param {number} max - Maximum value (exclusive)
 * @returns {number} Random integer
 */
export function randomInt(min, max) {
  return Math.floor(min + Math.random() * (max - min));
}

/**
 * Format a number as currency
 * @param {number} amount - Amount in minor units
 * @param {string} currency - Currency code
 * @returns {string} Formatted currency
 */
export function formatCurrency(amount, currency = 'USD') {
  const minorToMajor = { USD: 100, GBP: 100, EUR: 100, JPY: 1, CAD: 100, AUD: 100, CHF: 100, CNY: 100 };
  const divisor = minorToMajor[currency] || 100;
  const majorAmount = amount / divisor;
  return new Intl.NumberFormat('en-US', { style: 'currency', currency }).format(majorAmount);
}

/**
 * Add authentication headers to a request
 * @param {Object} params - Request params
 * @param {string} token - Auth token
 * @returns {Object} Params with auth headers
 */
export function withAuth(params, token) {
  if (!token) return params;
  return {
    ...params,
    headers: {
      ...params.headers,
      'Authorization': `Bearer ${token}`,
    },
  };
}

/**
 * Generate a random tag
 * @returns {string} Random tag
 */
export function randomTag() {
  const adjectives = ['important', 'urgent', 'high-priority', 'low-priority', 'customer', 'prospect', 'vip', 'follow-up', 'cold', 'warm', 'hot'];
  const nouns = ['lead', 'deal', 'contact', 'account', 'opportunity', 'ticket', 'campaign', 'meeting', 'call', 'email'];
  return `${randomItem(adjectives)}-${randomItem(nouns)}`;
}

/**
 * Generate a random source
 * @returns {string} Random source
 */
export function randomSource() {
  const sources = ['web', 'phone', 'email', 'referral', 'social', 'advertisement', 'event', 'import'];
  return randomItem(sources);
}

/**
 * Generate a random status
 * @param {string} entityType - Entity type (lead, deal, ticket, etc.)
 * @returns {string} Random status
 */
export function randomStatus(entityType) {
  const statuses = {
    lead: ['new', 'contacted', 'qualified', 'converted', 'disqualified'],
    deal: ['new', 'qualified', 'proposal', 'negotiation', 'won', 'lost'],
    ticket: ['open', 'in_progress', 'waiting', 'resolved', 'closed'],
    campaign: ['draft', 'scheduled', 'running', 'paused', 'completed', 'cancelled'],
    activity: ['planned', 'completed', 'cancelled'],
  };
  return randomItem(statuses[entityType] || ['active', 'inactive']);
}
