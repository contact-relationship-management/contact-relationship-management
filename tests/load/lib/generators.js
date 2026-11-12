// Data generators for load testing
// Generates realistic test data for CRM entities

import {
  randomUUID,
  randomString,
  randomEmail,
  randomPhone,
  randomPersonRef,
  randomOrganizationRef,
  randomDate,
  randomPastDate,
  randomFutureDate,
  randomAmount,
  randomCurrency,
  randomBoolean,
  randomInt,
  randomItem,
  randomTag,
  randomSource,
  randomStatus,
} from './utils.js';

// Common names for realistic test data
const FIRST_NAMES = [
  'James', 'Mary', 'John', 'Patricia', 'Robert', 'Jennifer', 'Michael', 'Linda',
  'William', 'Elizabeth', 'David', 'Barbara', 'Richard', 'Susan', 'Joseph', 'Jessica',
  'Thomas', 'Sarah', 'Charles', 'Karen', 'Christopher', 'Nancy', 'Daniel', 'Lisa',
  'Matthew', 'Margaret', 'Anthony', 'Betty', 'Mark', 'Sandra', 'Donald', 'Ashley',
];

const LAST_NAMES = [
  'Smith', 'Johnson', 'Williams', 'Brown', 'Jones', 'Garcia', 'Miller', 'Davis',
  'Rodriguez', 'Martinez', 'Hernandez', 'Lopez', 'Gonzalez', 'Wilson', 'Anderson', 'Thomas',
  'Taylor', 'Moore', 'Jackson', 'Martin', 'Lee', 'Perez', 'Thompson', 'White',
  'Harris', 'Sanchez', 'Clark', 'Ramirez', 'Lewis', 'Robinson', 'Walker', 'Young',
];

const ORGANIZATIONS = [
  'Acme Corp',
  'Globex',
  'Initech',
  'Wayne Enterprises',
  'Stark Industries',
  'Oscorp',
  'Wonka Industries',
  'Umbrella Corporation',
  'Cyberdyne Systems',
  'Tyrell Corporation',
  'Soylent Corp',
  'MomCorp',
  'Gringotts',
  'Wonka',
  'Dunder Mifflin',
];

const JOB_TITLES = [
  'CEO',
  'CTO',
  'CFO',
  'COO',
  'Sales Director',
  'Marketing Manager',
  'Product Manager',
  'Project Manager',
  'Software Engineer',
  'DevOps Engineer',
  'UX Designer',
  'UI Designer',
  'Data Scientist',
  'Account Manager',
  'Customer Success Manager',
  'Support Engineer',
  'QA Engineer',
  'HR Manager',
  'Financial Analyst',
  'Operations Manager',
];

const COUNTRIES = [
  'United States',
  'United Kingdom',
  'Canada',
  'Australia',
  'Germany',
  'France',
  'Japan',
  'India',
  'Brazil',
  'China',
];

const CITIES = [
  'New York',
  'London',
  'Tokyo',
  'Paris',
  'Berlin',
  'Sydney',
  'Toronto',
  'Mumbai',
  'Sao Paulo',
  'Beijing',
  'Chicago',
  'Los Angeles',
  'Manchester',
  'Birmingham',
  'Melbourne',
];

const ADDRESSES = [
  '123 Main St',
  '456 Oak Ave',
  '789 Pine Rd',
  '1010 Maple Blvd',
  '2020 Cedar Ln',
  '3030 Elm St',
  '4040 Birch Ave',
  '5050 Spruce Rd',
];

const CAMPAIGN_TYPES = ['email', 'social', 'webinar', 'event', 'print', 'tv', 'radio'];
const CAMPAIGN_STATUS = ['draft', 'scheduled', 'running', 'paused', 'completed', 'cancelled'];

const PIPELINE_NAMES = [
  'Sales Pipeline',
  'Enterprise Pipeline',
  'SMB Pipeline',
  'Lead Pipeline',
  'Opportunity Pipeline',
];

const STAGE_NAMES = [
  'New',
  'Qualified',
  'Proposal',
  'Negotiation',
  'Closed Won',
  'Closed Lost',
  'On Hold',
  'Follow Up',
];

const TICKET_PRIORITIES = ['low', 'medium', 'high', 'critical'];
const TICKET_TYPES = ['bug', 'feature', 'question', 'enhancement', 'documentation'];

/**
 * Generate a realistic contact
 * @param {Object} overrides - Override properties
 * @returns {Object} Contact object
 */
export function generateContact(overrides = {}) {
  const personRef = overrides.personRef || randomPersonRef();
  const firstName = overrides.firstName || randomItem(FIRST_NAMES);
  const lastName = overrides.lastName || randomItem(LAST_NAMES);
  const email = overrides.email || `${firstName.toLowerCase()}.${lastName.toLowerCase()}@example.com`;
  
  return {
    person_ref: personRef,
    first_name: firstName,
    last_name: lastName,
    display_name: `${firstName} ${lastName}`,
    email,
    phone: overrides.phone || randomPhone(),
    organization_ref: overrides.organizationRef || randomOrganizationRef(),
    job_title: overrides.jobTitle || randomItem(JOB_TITLES),
    tags: overrides.tags || [randomTag(), randomTag()],
    source: overrides.source || randomSource(),
    status: overrides.status || 'active',
    owner_ref: overrides.ownerRef || randomPersonRef(),
    address: overrides.address || `${randomItem(ADDRESSES)}, ${randomItem(CITIES)}, ${randomItem(COUNTRIES)}`,
    ...overrides,
  };
}

/**
 * Generate a realistic account
 * @param {Object} overrides - Override properties
 * @returns {Object} Account object
 */
export function generateAccount(overrides = {}) {
  return {
    organization_ref: overrides.organizationRef || randomOrganizationRef(),
    name: overrides.name || randomItem(ORGANIZATIONS),
    domain: overrides.domain || `${randomString(8)}.com`,
    industry: overrides.industry || randomItem(['Technology', 'Finance', 'Healthcare', 'Education', 'Retail', 'Manufacturing']),
    size: overrides.size || randomItem(['SMB', 'Mid-Market', 'Enterprise']),
    phone: overrides.phone || randomPhone(),
    address: overrides.address || `${randomItem(ADDRESSES)}, ${randomItem(CITIES)}, ${randomItem(COUNTRIES)}`,
    owner_ref: overrides.ownerRef || randomPersonRef(),
    tags: overrides.tags || [randomTag()],
    ...overrides,
  };
}

/**
 * Generate a realistic lead
 * @param {Object} overrides - Override properties
 * @returns {Object} Lead object
 */
export function generateLead(overrides = {}) {
  const contact = generateContact();
  
  return {
    person_ref: overrides.personRef || contact.person_ref,
    display_name: overrides.displayName || contact.display_name,
    email: overrides.email || contact.email,
    phone: overrides.phone || contact.phone,
    organization_ref: overrides.organizationRef || contact.organization_ref,
    source: overrides.source || randomSource(),
    score: overrides.score || randomInt(0, 100),
    status: overrides.status || randomStatus('lead'),
    owner_ref: overrides.ownerRef || randomPersonRef(),
    campaign_ref: overrides.campaignRef || null,
    tags: overrides.tags || [randomTag()],
    ...overrides,
  };
}

/**
 * Generate a realistic deal
 * @param {Object} overrides - Override properties
 * @returns {Object} Deal object
 */
export function generateDeal(overrides = {}) {
  return {
    name: overrides.name || `Deal ${randomString(6)}`,
    amount: overrides.amount || randomAmount(1000, 1000000),
    currency: overrides.currency || randomCurrency(),
    pipeline_ref: overrides.pipelineRef || randomUUID(),
    stage_ref: overrides.stageRef || randomUUID(),
    contact_ref: overrides.contactRef || randomPersonRef(),
    account_ref: overrides.accountRef || randomOrganizationRef(),
    owner_ref: overrides.ownerRef || randomPersonRef(),
    status: overrides.status || randomStatus('deal'),
    probability: overrides.probability || randomInt(0, 100),
    expected_close_date: overrides.expectedCloseDate || randomFutureDate(90),
    tags: overrides.tags || [randomTag()],
    ...overrides,
  };
}

/**
 * Generate a realistic activity
 * @param {Object} overrides - Override properties
 * @returns {Object} Activity object
 */
export function generateActivity(overrides = {}) {
  const types = ['call', 'email', 'meeting', 'note', 'task'];
  const type = overrides.type || randomItem(types);
  
  return {
    subject_kind: overrides.subjectKind || randomItem(['contact', 'account', 'lead', 'deal']),
    subject_pid: overrides.subjectPid || randomUUID(),
    kind: type,
    summary: overrides.summary || `Activity: ${randomString(20)}`,
    description: overrides.description || `Detailed description of the ${type} activity.`,
    occurred_at: overrides.occurredAt || randomPastDate(30),
    due_at: type === 'task' ? randomFutureDate(7) : null,
    actor_ref: overrides.actorRef || randomPersonRef(),
    assignee_ref: overrides.assigneeRef || randomPersonRef(),
    status: overrides.status || randomStatus('activity'),
    tags: overrides.tags || [],
    ...overrides,
  };
}

/**
 * Generate a realistic campaign
 * @param {Object} overrides - Override properties
 * @returns {Object} Campaign object
 */
export function generateCampaign(overrides = {}) {
  return {
    name: overrides.name || `Campaign ${randomString(8)}`,
    description: overrides.description || `Marketing campaign for ${randomItem(['Q1', 'Q2', 'Q3', 'Q4'])}`,
    type: overrides.type || randomItem(CAMPAIGN_TYPES),
    status: overrides.status || randomStatus('campaign'),
    budget: overrides.budget || randomAmount(1000, 100000),
    currency: overrides.currency || randomCurrency(),
    start_date: overrides.startDate || randomPastDate(30),
    end_date: overrides.endDate || randomFutureDate(60),
    owner_ref: overrides.ownerRef || randomPersonRef(),
    tags: overrides.tags || [randomTag()],
    ...overrides,
  };
}

/**
 * Generate a realistic ticket
 * @param {Object} overrides - Override properties
 * @returns {Object} Ticket object
 */
export function generateTicket(overrides = {}) {
  return {
    title: overrides.title || `Support: ${randomString(20)}`,
    description: overrides.description || `Detailed description of the support issue.`,
    priority: overrides.priority || randomItem(TICKET_PRIORITIES),
    type: overrides.type || randomItem(TICKET_TYPES),
    status: overrides.status || 'open',
    contact_ref: overrides.contactRef || randomPersonRef(),
    account_ref: overrides.accountRef || randomOrganizationRef(),
    assignee_ref: overrides.assigneeRef || randomPersonRef(),
    owner_ref: overrides.ownerRef || randomPersonRef(),
    tags: overrides.tags || [randomTag()],
    ...overrides,
  };
}

/**
 * Generate a realistic pipeline
 * @param {Object} overrides - Override properties
 * @returns {Object} Pipeline object
 */
export function generatePipeline(overrides = {}) {
  const stages = [];
  const numStages = overrides.numStages || randomInt(3, 8);
  
  for (let i = 0; i < numStages; i++) {
    stages.push({
      name: overrides.stageNames?.[i] || STAGE_NAMES[i] || `Stage ${i + 1}`,
      position: i,
      probability: overrides.stageProbabilities?.[i] || (i === numStages - 1 ? 100 : i * 15),
      is_terminal: overrides.isTerminal?.[i] || (i === numStages - 1),
      won: overrides.won?.[i] || (i === numStages - 1 && randomBoolean()),
    });
  }
  
  return {
    name: overrides.name || randomItem(PIPELINE_NAMES),
    description: overrides.description || `Sales pipeline for ${randomItem(['Enterprise', 'SMB', 'All'])} customers`,
    stages,
    owner_ref: overrides.ownerRef || randomPersonRef(),
    tags: overrides.tags || [],
    ...overrides,
  };
}

/**
 * Generate a realistic SLA policy
 * @param {Object} overrides - Override properties
 * @returns {Object} SLA Policy object
 */
export function generateSlaPolicy(overrides = {}) {
  return {
    name: overrides.name || `SLA ${randomString(6)}`,
    description: overrides.description || `SLA policy for ${randomItem(['Standard', 'Premium', 'Enterprise'])} support`,
    first_response_minutes: overrides.firstResponseMinutes || randomInt(15, 240),
    resolution_hours: overrides.resolutionHours || randomInt(2, 48),
    business_hours: overrides.businessHours || {
      monday: ['09:00-17:00'],
      tuesday: ['09:00-17:00'],
      wednesday: ['09:00-17:00'],
      thursday: ['09:00-17:00'],
      friday: ['09:00-17:00'],
    },
    timezone: overrides.timezone || 'UTC',
    escalate_to_ref: overrides.escalateToRef || randomPersonRef(),
    tags: overrides.tags || [],
    ...overrides,
  };
}

/**
 * Generate a realistic knowledge base article
 * @param {Object} overrides - Override properties
 * @returns {Object} KB Article object
 */
export function generateArticle(overrides = {}) {
  return {
    title: overrides.title || `How to ${randomString(20)}`,
    body: overrides.body || `This is the body of the knowledge base article.\n\nIt contains detailed information about ${randomString(30)}.`,
    status: overrides.status || randomItem(['draft', 'published', 'archived']),
    version: overrides.version || 1,
    author_ref: overrides.authorRef || randomPersonRef(),
    tags: overrides.tags || [randomTag(), randomTag()],
    ...overrides,
  };
}

/**
 * Generate multiple entities of the same type
 * @param {Function} generator - Generator function
 * @param {number} count - Number of entities to generate
 * @param {Object} overrides - Override properties (applied to all)
 * @returns {Array} Array of generated entities
 */
export function generateMany(generator, count, overrides = {}) {
  return Array.from({ length: count }, () => generator(overrides));
}

/**
 * Generate a complete test dataset
 * @param {Object} options - Generation options
 * @returns {Object} Complete dataset
 */
export function generateDataset(options = {}) {
  const {
    contacts = 100,
    accounts = 50,
    leads = 100,
    deals = 100,
    activities = 200,
    campaigns = 10,
    tickets = 50,
    pipelines = 5,
    slaPolicies = 5,
    articles = 20,
  } = options;
  
  return {
    contacts: generateMany(generateContact, contacts),
    accounts: generateMany(generateAccount, accounts),
    leads: generateMany(generateLead, leads),
    deals: generateMany(generateDeal, deals),
    activities: generateMany(generateActivity, activities),
    campaigns: generateMany(generateCampaign, campaigns),
    tickets: generateMany(generateTicket, tickets),
    pipelines: generateMany(generatePipeline, pipelines),
    slaPolicies: generateMany(generateSlaPolicy, slaPolicies),
    articles: generateMany(generateArticle, articles),
  };
}

/**
 * Generate a random entity reference based on type
 * @param {string} type - Entity type
 * @returns {string} Entity reference
 */
export function generateEntityRef(type) {
  const prefixes = {
    contact: 'person',
    account: 'organization',
    lead: 'person',
    deal: null,
    activity: null,
    campaign: null,
    ticket: null,
    pipeline: null,
    slaPolicy: null,
    article: null,
  };
  
  if (prefixes[type]) {
    return `${prefixes[type]}:${randomUUID()}`;
  }
  return randomUUID();
}

export {
  FIRST_NAMES,
  LAST_NAMES,
  ORGANIZATIONS,
  JOB_TITLES,
  COUNTRIES,
  CITIES,
  ADDRESSES,
  CAMPAIGN_TYPES,
  CAMPAIGN_STATUS,
  PIPELINE_NAMES,
  STAGE_NAMES,
  TICKET_PRIORITIES,
  TICKET_TYPES,
};
