// The guided tour's fictitious back end (server-side only).
//
// Signing in as `example@example.com` gives a demo session. While that
// session is active the BFF proxy answers every API call from this module
// instead of forwarding to the CRM service, so the whole app can be explored
// with no service, no real data and no real account. Everything here is
// invented: the people, organisations and figures do not exist.
//
// Each sign-in to the tour gets its own session id and its own copy of the
// data, held in memory, so one visitor never sees what another typed. Writes
// (moving a deal, resolving a ticket, recording consent, …) change that copy,
// so the screens behave as they would against the real service. At most
// `MAX_DEMO_SESSIONS` copies are kept; the least recently used goes first, and
// a visitor whose copy went starts again from the original data.

import { matches } from "#lib/search.js";
import type {
  Account,
  Activity,
  Article,
  Campaign,
  Contact,
  Deal,
  Lead,
  Notification,
  SavedView,
  Segment,
  EmailTemplate,
  Workflow,
  WorkflowRun,
  CustomField,
  CustomFieldValue,
  Attachment,
  Tag,
  Report,
  ReportResult,
  Attribution,
  AttributionAmount,
  KbDeflection,
  CommissionPlan,
  Quota,
  QuotaAttainment,
  CommissionBand,
  CommissionTier,
  FxRate,
  ReportRateUsed,
  ReportUnconverted,
  ReportSpec,
  MergeToken,
  TemplatePreview,
  Stage,
  Ticket,
} from "#lib/api/crm.js";

/** The only address the tour accepts. */
export const DEMO_EMAIL = "example@example.com";

/** Every demo session id starts with this (never sent to a real service). */
export const DEMO_SESSION_PREFIX = "demo-tour-";

/** How many visitors' copies of the tour data are kept in memory. */
export const MAX_DEMO_SESSIONS = 200;

/** Whether `sid` is a demo session. */
export function isDemoSession(sid: string | null | undefined): boolean {
  return typeof sid === "string" && sid.startsWith(DEMO_SESSION_PREFIX);
}

/** Whether `email` is the tour's address (case and spacing ignored). */
export function isDemoEmail(email: string): boolean {
  return email.trim().toLowerCase() === DEMO_EMAIL;
}

/** A reply the proxy turns into an HTTP response. */
export interface DemoReply {
  status: number;
  body: unknown;
}

const DAY = 86_400_000;
const iso = (offsetDays: number, hour = 9): string => {
  const d = new Date(Date.now() + offsetDays * DAY);
  d.setUTCHours(hour, 0, 0, 0);
  return d.toISOString();
};
const day = (offsetDays: number): string => iso(offsetDays).slice(0, 10);
/** Whole days from today to an ISO day (negative when it has passed). */
const daysFromToday = (isoDay: string): number =>
  Math.round(
    (Date.parse(`${isoDay}T00:00:00Z`) - Date.parse(`${day(0)}T00:00:00Z`)) /
      86_400_000,
  );

interface Link {
  deals: string[];
  tickets: string[];
}

interface Grant {
  pid: string;
  subject: string;
  reason: string;
  granted_at: string;
  expires_at: string;
  reviewed_at: string | null;
  reviewed_by: string | null;
  review_note: string | null;
}

interface FollowupSeed {
  pid: string;
  kind: string;
  summary: string;
  who: string;
  recorder: string;
  due_on: string;
}

interface MergedRow {
  contact: Contact;
  survivor_pid: string;
  links: Link;
  external: ExternalLinkRow[];
  at: string;
}

interface ExternalLinkRow {
  pid: string;
  subject_kind: "contact" | "account" | "deal" | "lead";
  subject_pid: string;
  system: string;
  url: string;
  external_id: string | null;
  label: string | null;
}

interface State {
  segments: Segment[];
  /** The demo email templates (CRM-R36). */
  templates: EmailTemplate[];
  /** The demo workflow rules and their run log (CRM-R40). */
  workflows: Workflow[];
  workflowRuns: WorkflowRun[];
  /** The demo's custom fields and the values set on records, by field key and record (CRM-R41). */
  customFields: CustomField[];
  customValues: Record<string, string | number>;
  /** Files attached in the tour (name and size only), by record pid (CRM-R42). */
  attachments: Record<string, Attachment[]>;
  /** Tag names by record pid, and the tags that exist (CRM-R47). */
  tagged: Record<string, string[]>;
  tags: Tag[];
  /** The tour's saved reports (CRM-R48). */
  reports: Report[];
  /** The tour's exchange rates (CRM-R50). */
  fxRates: FxRate[];
  /** The demo user's saved views (CRM-R35). */
  views: SavedView[];
  /** The inbox of the one demo user (CRM-R32). */
  notifications: Notification[];
  followups: FollowupSeed[];
  accounts: Account[];
  contacts: Contact[];
  links: Record<string, Link>;
  /** Links to the same contact in other systems (CRM-R27), by contact pid. */
  external: Record<string, ExternalLinkRow[]>;
  /** Merges that can be undone (CRM-R26). */
  merged: MergedRow[];
  activities: Activity[];
  leads: Lead[];
  stages: Stage[];
  deals: Deal[];
  campaigns: Campaign[];
  tickets: Ticket[];
  articles: Article[];
  /** The tour's commission plans and quotas (CRM-R59). */
  plans: CommissionPlan[];
  quotas: Quota[];
  /** Deals credited to several workers, by deal (CRM-R65). */
  splits: Record<string, { owner_ref: string; share_bps: number }[]>;
  /** Teams of workers for the quota roll-up (CRM-R67). */
  teams: { pid: string; name: string; members: string[]; set_by: string }[];
  /** What the public help centre has counted (CRM-R57). */
  kb: {
    views: Record<string, number>;
    yes: Record<string, number>;
    no: Record<string, number>;
    searches: number;
    empty: number;
  };
  grants: Grant[];
}

const PIPELINE = { pid: "pipe-1", name: "New business" };

function build(): State {
  const accounts: Account[] = [
    ["acc-1", "Brightwater Energy", "strategic", "Energy"],
    ["acc-2", "Harbourlight Logistics", "growth", "Logistics"],
    ["acc-3", "Foxglove Health Trust", "strategic", "Healthcare"],
    ["acc-4", "Kestrel Software", "growth", "Software"],
    ["acc-5", "Marlow & Finch Legal", "standard", "Legal services"],
    ["acc-6", "Tidewell Borough Council", "standard", "Public sector"],
  ].map(([pid, display_name, tier, industry], i) => ({
    pid: pid!,
    organization_ref: `org-demo-${i + 1}`,
    display_name: display_name!,
    tier: tier!,
    industry: industry!,
  }));

  const people: Array<[string, string, string, string, string]> = [
    // pid, name, account, title, consent
    ["con-1", "Imogen Hartley", "acc-1", "Head of Procurement", "granted"],
    ["con-2", "Rafael Okonkwo", "acc-1", "Operations Director", "granted"],
    ["con-3", "Saoirse Lindqvist", "acc-2", "Fleet Manager", "granted"],
    ["con-4", "Tobias Marchetti", "acc-2", "Finance Lead", "withdrawn"],
    [
      "con-5",
      "Priya Venkataraman",
      "acc-3",
      "Digital Programme Lead",
      "granted",
    ],
    [
      "con-6",
      "Elias Bergstrom",
      "acc-3",
      "Chief Clinical Information Officer",
      "granted",
    ],
    ["con-7", "Wren Callaghan", "acc-4", "Product Director", "granted"],
    ["con-8", "Dara Whitlock", "acc-4", "Engineering Manager", "granted"],
    ["con-9", "Lucien Ashdown", "acc-5", "Managing Partner", "unknown"],
    ["con-10", "Noor Haddad", "acc-5", "Practice Manager", "granted"],
    [
      "con-11",
      "Gethin Prosser",
      "acc-6",
      "Head of Customer Services",
      "granted",
    ],
    [
      "con-12",
      "Odette Fairweather",
      "acc-6",
      "Transformation Lead",
      "withdrawn",
    ],
    // A near-duplicate of con-1 (a misspelt surname), so the duplicates
    // section has something to show. Fictitious like the rest.
    ["con-13", "Imogen Hartly", "acc-1", "Procurement Head", "unknown"],
  ];
  const contacts: Contact[] = people.map(
    ([pid, name, account, title, consent], i) => ({
      pid,
      person_ref: `person-demo-${i + 1}`,
      account_pid: account,
      owner_ref: i % 2 === 0 ? "owner-ayo" : "owner-bea",
      display_name: name,
      given_name: null,
      family_name: null,
      status: "active",
      job_title: title,
      preferred_channel: i % 3 === 0 ? "phone" : "email",
      marketing_consent: consent,
    }),
  );

  const stages: Stage[] = [
    ["stg-1", "Qualification", 10, false, false],
    ["stg-2", "Discovery", 25, false, false],
    ["stg-3", "Proposal", 50, false, false],
    ["stg-4", "Negotiation", 75, false, false],
    ["stg-5", "Won", 100, true, false],
    ["stg-6", "Lost", 0, false, true],
  ].map(([pid, name, probability_percent, is_won, is_lost], position) => ({
    pid: pid as string,
    name: name as string,
    position,
    probability_percent: probability_percent as number,
    is_won: is_won as boolean,
    is_lost: is_lost as boolean,
  }));

  const deal = (
    pid: string,
    name: string,
    stage: string,
    pounds: number,
    closedDays: number | null = null,
  ): Deal => {
    const s = stages.find((x) => x.pid === stage)!;
    return {
      pid,
      name,
      pipeline_pid: PIPELINE.pid,
      stage_pid: stage,
      amount_minor: pounds * 100,
      currency: "GBP",
      won: s.is_won,
      closed_at: closedDays === null ? null : iso(closedDays),
      owner_ref:
        pid.endsWith("1") || pid.endsWith("3") ? "owner-ayo" : "owner-bea",
      expected_close_on: day(20 + (pid.charCodeAt(pid.length - 1) % 7) * 9),
    };
  };
  const deals: Deal[] = [
    deal("deal-1", "Smart-meter analytics rollout", "stg-4", 184_000),
    deal("deal-2", "Depot routing platform", "stg-3", 96_500),
    deal("deal-3", "Clinical records integration", "stg-3", 242_000),
    deal("deal-4", "Customer portal refresh", "stg-2", 58_000),
    deal("deal-5", "Developer tooling licences", "stg-2", 31_200),
    deal("deal-6", "Case-management upgrade", "stg-1", 74_000),
    deal("deal-7", "Field-service mobile app", "stg-1", 45_500),
    deal("deal-8", "Grid-edge sensors pilot", "stg-4", 128_000),
    deal("deal-9", "Document automation", "stg-5", 52_000, -12),
    deal("deal-10", "Fleet telematics renewal", "stg-5", 88_000, -30),
    deal("deal-11", "Appointment reminders", "stg-5", 27_500, -45),
    deal("deal-12", "Procurement data warehouse", "stg-6", 66_000, -20),
  ];

  const links: Record<string, Link> = {
    "con-1": { deals: ["deal-1", "deal-8"], tickets: [] },
    "con-2": { deals: ["deal-8"], tickets: ["tkt-1"] },
    "con-3": { deals: ["deal-2", "deal-10"], tickets: ["tkt-2"] },
    "con-5": { deals: ["deal-3", "deal-11"], tickets: [] },
    "con-6": { deals: ["deal-3"], tickets: ["tkt-3"] },
    "con-7": { deals: ["deal-5"], tickets: ["tkt-4"] },
    "con-10": { deals: ["deal-9"], tickets: ["tkt-5"] },
    "con-11": { deals: ["deal-4", "deal-6"], tickets: ["tkt-6"] },
    "con-12": { deals: ["deal-6"], tickets: [] },
  };

  const act = (
    pid: string,
    who: string,
    kind: string,
    daysAgo: number,
    summary: string,
  ): Activity => ({
    pid,
    subject_kind: "contact",
    subject_pid: who,
    kind,
    occurred_at: iso(-daysAgo, 14),
    summary,
  });
  const activities: Activity[] = [
    act(
      "act-1",
      "con-1",
      "call",
      3,
      "Walked through the analytics pilot results",
    ),
    act("act-2", "con-1", "email", 9, "Sent the revised commercial terms"),
    act(
      "act-3",
      "con-1",
      "meeting",
      21,
      "On-site workshop with the operations team",
    ),
    act("act-4", "con-3", "email", 6, "Shared routing demo recording"),
    act("act-5", "con-3", "call", 18, "Discovery call: depot constraints"),
    act(
      "act-6",
      "con-5",
      "meeting",
      4,
      "Steering group: integration scope agreed",
    ),
    act(
      "act-7",
      "con-5",
      "note",
      15,
      "Prefers a phased go-live across three clinics",
    ),
    act(
      "act-8",
      "con-7",
      "email",
      11,
      "Trial licences issued to the platform team",
    ),
    act(
      "act-9",
      "con-11",
      "call",
      2,
      "Council budget cycle: decision due next quarter",
    ),
    act("act-10", "con-2", "call", 5, "Escalated sensor firmware question"),
  ];

  const lead = (
    pid: string,
    source: string,
    name: string,
    email: string | null,
    score: number,
    status: string,
  ): Lead => ({
    pid,
    source,
    display_name: name,
    email,
    score,
    status,
    owner_ref: null,
  });
  const leads: Lead[] = [
    lead(
      "lead-1",
      "referral",
      "Hollis Greenway",
      "hollis@example.org",
      78,
      "new",
    ),
    lead(
      "lead-2",
      "web form",
      "Mireille Davenport",
      "mireille@example.org",
      64,
      "contacted",
    ),
    lead(
      "lead-3",
      "event",
      "Anselm Kowalczyk",
      "anselm@example.org",
      52,
      "new",
    ),
    lead("lead-4", "web form", "Tamsin Reyes", null, 41, "contacted"),
    lead(
      "lead-5",
      "webinar",
      "Jericho Abara",
      "jericho@example.org",
      33,
      "new",
    ),
    lead(
      "lead-6",
      "cold outreach",
      "Philippa Strand",
      "philippa@example.org",
      19,
      "disqualified",
    ),
    lead(
      "lead-7",
      "referral",
      "Cormac Idowu",
      "cormac@example.org",
      88,
      "qualified",
    ),
  ].sort((a, b) => b.score - a.score);

  // Dates are relative to today so the timeline always looks current.
  const ago = (days: number) =>
    new Date(Date.now() - days * 86_400_000).toISOString();
  const campaign = (
    pid: string,
    name: string,
    status: string,
    pounds: number,
    recipients: number,
    delivered: number,
    opened: number,
    clicked: number,
    span: { created_at: string; updated_at: string },
  ): Campaign => ({
    pid,
    name,
    status,
    cost_minor: pounds * 100,
    currency: "GBP",
    recipients,
    delivered,
    opened,
    clicked,
    ...span,
  });
  const campaigns: Campaign[] = [
    campaign(
      "camp-1",
      "Spring product update",
      "completed",
      2400,
      1200,
      1168,
      612,
      148,
      { created_at: ago(75), updated_at: ago(40) },
    ),
    campaign(
      "camp-2",
      "Customer webinar series",
      "running",
      5200,
      860,
      842,
      405,
      96,
      { created_at: ago(30), updated_at: ago(2) },
    ),
    campaign("camp-3", "Renewal reminders", "scheduled", 800, 340, 0, 0, 0, {
      created_at: ago(10),
      updated_at: ago(6),
    }),
    campaign("camp-4", "Autumn roadmap preview", "draft", 1500, 0, 0, 0, 0, {
      created_at: ago(3),
      updated_at: ago(3),
    }),
  ];

  const ticket = (
    pid: string,
    title: string,
    priority: string,
    status: string,
    dueDays: number,
    first: boolean,
    resolution: boolean,
  ): Ticket => ({
    pid,
    title,
    priority,
    status,
    first_response_due_at: iso(dueDays, 12),
    paused_at: status === "pending" ? iso(dueDays, 12) : null,
    wait_limit_exceeded_at:
      status === "pending" && dueDays <= -2 ? iso(dueDays, 12) : null,
    live_first_response_breached: first,
    live_resolution_breached: resolution,
  });
  const tickets: Ticket[] = [
    ticket(
      "tkt-1",
      "Sensor firmware update fails on site",
      "urgent",
      "open",
      -1,
      true,
      false,
    ),
    ticket(
      "tkt-2",
      "Route export missing depot codes",
      "high",
      "open",
      0,
      false,
      false,
    ),
    ticket(
      "tkt-3",
      "Clinical feed delayed overnight",
      "urgent",
      "pending",
      -2,
      true,
      true,
    ),
    ticket(
      "tkt-4",
      "Licence seats not showing in console",
      "normal",
      "open",
      1,
      false,
      false,
    ),
    ticket(
      "tkt-5",
      "Template merge drops line breaks",
      "low",
      "open",
      2,
      false,
      false,
    ),
    ticket(
      "tkt-6",
      "Cannot reset portal password",
      "high",
      "open",
      0,
      false,
      false,
    ),
    ticket(
      "tkt-7",
      "Dashboard totals differ from export",
      "normal",
      "resolved",
      -6,
      false,
      false,
    ),
    ticket(
      "tkt-8",
      "Request: add SSO for council staff",
      "low",
      "resolved",
      -9,
      false,
      false,
    ),
  ];

  const article = (
    pid: string,
    title: string,
    body: string,
    status: string,
    version: number,
  ): Article => ({ pid, title, body, status, version });
  const articles: Article[] = [
    article(
      "kb-1",
      "Getting started with lead scoring",
      "Scores add up points from the rules your team enables: a referral source, a recent reply, a matching industry. Open a lead to see which rules fired and how many points each added.",
      "published",
      3,
    ),
    article(
      "kb-2",
      "How the forecast is calculated",
      "The forecast is each open deal's amount multiplied by its stage probability, totalled per currency. It is derived on request and never typed in by hand.",
      "published",
      2,
    ),
    article(
      "kb-3",
      "Resetting a customer's portal password",
      "Confirm the person's identity, then send a reset link from the portal admin page. Links expire after one hour.",
      "published",
      5,
    ),
    article(
      "kb-4",
      "Handling a consent withdrawal",
      "Record the withdrawal on the contact straight away. Campaigns skip withdrawn contacts at send time, so no further mail goes out.",
      "published",
      1,
    ),
    article(
      "kb-6",
      "Licence seats missing from the console",
      "Seats appear in the console once the licence is synced. Ask the customer to sign out and in, then check the sync status on the licence page.",
      "published",
      1,
    ),
    article(
      "kb-5",
      "Draft: SLA policy for strategic accounts",
      "Strategic accounts get a one-hour first response on urgent tickets. This draft is awaiting review.",
      "draft",
      1,
    ),
  ];

  const grants: Grant[] = [
    {
      pid: "bg-1",
      subject: "owner-bea",
      reason:
        "Customer on the phone disputing an invoice; needed to see unmasked deal values to answer.",
      granted_at: iso(-3, 10),
      expires_at: iso(-3, 10).replace("T10:00", "T10:15"),
      reviewed_at: iso(-2, 9),
      reviewed_by: "owner-ayo",
      review_note: "Reasonable; the invoice dispute is on file.",
    },
    {
      pid: "bg-2",
      subject: "owner-ayo",
      reason:
        "Urgent security question from a strategic account about which contacts hold marketing consent.",
      granted_at: iso(-1, 15),
      expires_at: iso(-1, 15).replace("T15:00", "T15:30"),
      reviewed_at: null,
      reviewed_by: null,
      review_note: null,
    },
  ];

  const fu = (
    pid: string,
    kind: string,
    summary: string,
    who: string,
    recorder: string,
    dueDays: number,
  ): FollowupSeed => ({
    pid,
    kind,
    summary,
    who,
    recorder,
    due_on: day(dueDays),
  });
  const followups: FollowupSeed[] = [
    fu(
      "fu-1",
      "task",
      "Send revised proposal to Foxglove",
      "con-5",
      "owner-ayo",
      -4,
    ),
    fu(
      "fu-2",
      "call",
      "Check in after the council budget meeting",
      "con-11",
      "owner-bea",
      -1,
    ),
    fu(
      "fu-3",
      "task",
      "Renewal paperwork for Harbourlight",
      "con-3",
      "owner-bea",
      3,
    ),
    fu(
      "fu-4",
      "meeting",
      "Quarterly review with Brightwater",
      "con-1",
      "owner-ayo",
      8,
    ),
    fu(
      "fu-5",
      "task",
      "Book onboarding for Kestrel licences",
      "con-7",
      "owner-ayo",
      14,
    ),
  ];
  const notifications: Notification[] = [
    {
      pid: "note-1",
      kind: "task_reminder",
      subject_kind: "contact",
      subject_pid: "con-5",
      created_at: iso(0, 8),
      read_at: null,
    },
    {
      pid: "note-2",
      kind: "sla_breach",
      subject_kind: "ticket",
      subject_pid: tickets[0]!.pid,
      created_at: iso(-1, 16),
      read_at: null,
    },
  ];
  return {
    segments: [{ pid: "seg-1", name: "Consented customers" }],
    workflows: [
      {
        pid: "wf-1",
        name: "Greet new emailers",
        enabled: true,
        trigger: "contact_created",
        every_minutes: null,
        conditions: {
          statuses: [],
          channels: ["email"],
          account_tiers: [],
          sources: [],
        },
        actions: [
          { type: "create_task", summary: "Say hello", due_in_days: 2 },
          { type: "enrol_nurture", sequence_pid: "seq-1" },
        ],
        updated_at: "2026-10-25T09:00:00Z",
      },
    ],
    customFields: [
      {
        pid: "cf-1",
        entity: "contact",
        key: "renewal_date",
        label: "Renewal date",
        kind: "date",
        choices: [],
        sensitivity: "business",
        position: 0,
      },
      {
        pid: "cf-2",
        entity: "contact",
        key: "hobby",
        label: "Hobby",
        kind: "text",
        choices: [],
        sensitivity: "personal",
        position: 1,
      },
    ],
    tagged: { "con-1": ["Key account"], "acc-1": ["Key account"] },
    tags: [{ pid: "tag-1", name: "Key account", uses: 2 }],
    fxRates: [
      fxRate("fx-1", "GBP", "USD", "1.27", -300),
      fxRate("fx-2", "GBP", "EUR", "1.17", -300),
      fxRate("fx-3", "GBP", "USD", "1.3", -20),
    ],
    reports: [
      {
        pid: "rep-1",
        owner_ref: "worker:demo",
        mine: true,
        name: "Leads by source",
        spec: {
          kind: "lead",
          group_by: "source",
          measure: "count",
          filter: {},
        },
        shared: false,
        updated_at: "2026-10-25T09:00:00Z",
      },
    ],
    attachments: {
      "con-1": [
        {
          pid: "att-1",
          name: "proposal.pdf",
          content_type: "application/pdf",
          size_bytes: 48213,
          sha256: "0".repeat(64),
          uploaded_by: "worker:demo",
          created_at: "2026-10-25T09:00:00Z",
        },
      ],
    },
    customValues: { "cf-1:con-1": "2027-03-01", "cf-2:con-1": "sailing" },
    workflowRuns: [
      {
        pid: "run-1",
        workflow_pid: "wf-1",
        workflow_name: "Greet new emailers",
        trigger: "contact_created",
        subject_kind: "contact",
        subject_pid: "con-2",
        depth: 0,
        outcome: "done",
        reason:
          "contact created on a contact; conditions met (channel is email)",
        steps: [
          {
            action: "create a task: Say hello",
            result: "done",
            detail: "task created",
          },
          {
            action: "enrol in nurture sequence seq-1",
            result: "skipped",
            detail: "no marketing consent at this moment",
          },
        ],
        created_at: "2026-10-25T09:05:00Z",
      },
    ],
    templates: [
      {
        pid: "template-1",
        name: "Autumn news",
        subject: "News for {{given_name}}",
        body: "Hi {{given_name}} at {{account_name}},\n\nFrom {{rep_name}}.\n\nStop: {{unsubscribe_url}}",
        fallbacks: { account_name: "your team" },
        in_use: 0,
        updated_at: "2026-10-25T09:00:00Z",
      },
    ],
    views: [
      {
        pid: "view-1",
        owner_ref: "worker:demo",
        mine: false,
        kind: "ticket",
        name: "Urgent and open",
        filter: { priority: ["urgent", "high"], status: ["open"] },
        sort: { field: "priority", direction: "desc" },
        columns: null,
        shared: true,
      },
    ],
    notifications,
    followups,
    accounts,
    contacts,
    links,
    merged: [],
    external: {
      [contacts[0]!.pid]: [
        {
          pid: "xl-1",
          subject_kind: "contact",
          subject_pid: contacts[0]!.pid,
          system: "hubspot",
          url: "https://app.hubspot.com/contacts/1234/record/0-1/5001",
          external_id: null,
          label: "HubSpot contact",
        },
      ],
    },
    activities,
    leads,
    stages,
    deals,
    campaigns,
    tickets,
    articles,
    plans: [
      {
        pid: "plan-1",
        name: "Standard",
        tiers: [
          { from_percent: 0, rate_bps: 500 },
          { from_percent: 100, rate_bps: 800 },
        ],
        quotas: 2,
        set_by: "worker:demo",
      },
    ],
    splits: {},
    teams: [],
    quotas: [
      {
        pid: "quota-1",
        owner_ref: "owner-ayo",
        period_from: day(-90),
        period_to: day(30),
        currency: "GBP",
        amount_minor: 5_000_000,
        plan_pid: "plan-1",
        set_by: "worker:demo",
      },
      {
        pid: "quota-2",
        owner_ref: "owner-bea",
        period_from: day(-90),
        period_to: day(30),
        currency: "GBP",
        amount_minor: 10_000_000,
        plan_pid: "plan-1",
        set_by: "worker:demo",
      },
    ],
    kb: {
      views: { "kb-1": 41, "kb-3": 27, "kb-2": 12 },
      yes: { "kb-1": 22, "kb-3": 15, "kb-2": 5 },
      no: { "kb-1": 3, "kb-3": 6, "kb-2": 4 },
      searches: 63,
      empty: 9,
    },
    grants,
  };
}

/** The copy `demoHandle` works on; `demoHandleFor` points it at a visitor's. */
let state: State = build();

/** Start the default copy (the one `demoHandle` uses alone) again. */
export function resetDemo(): void {
  state = build();
}

/** Each visitor's copy, least recently used first. */
const sessions = new Map<string, State>();

function keep(sid: string, copy: State): void {
  sessions.delete(sid);
  sessions.set(sid, copy);
  while (sessions.size > MAX_DEMO_SESSIONS) {
    sessions.delete(sessions.keys().next().value!);
  }
}

/** A new tour session with its own fresh copy of the data. */
export function startDemoSession(): string {
  const sid = `${DEMO_SESSION_PREFIX}${crypto.randomUUID()}`;
  keep(sid, build());
  return sid;
}

/** Forget a tour session's copy (sign-out). */
export function endDemoSession(sid: string): void {
  sessions.delete(sid);
}

/** How many tour copies are held (for tests). */
export function demoSessionCount(): number {
  return sessions.size;
}

/** `demoHandle` against the copy that belongs to `sid`. */
export function demoHandleFor(
  sid: string,
  method: string,
  path: string,
  query: URLSearchParams,
  body: unknown,
): DemoReply {
  const copy = sessions.get(sid) ?? build();
  keep(sid, copy);
  const saved = state;
  state = copy;
  try {
    // Synchronous, so no other request can run while `state` is swapped.
    return demoHandle(method, path, query, body);
  } finally {
    state = saved;
  }
}

const ok = (body: unknown): DemoReply => ({ status: 200, body });
const problem = (status: number, description: string): DemoReply => ({
  status,
  body: { error: status === 404 ? "not_found" : "unprocessable", description },
});

const open = (status: string) => !["resolved", "closed"].includes(status);

function ratio(numerator: number, denominator: number) {
  return {
    numerator,
    denominator,
    value: denominator === 0 ? null : numerator / denominator,
  };
}

function nameOf(pid: string): string | null {
  return state.accounts.find((a) => a.pid === pid)?.display_name ?? null;
}

function openDeals(): Deal[] {
  const final = new Set(
    state.stages.filter((s) => s.is_won || s.is_lost).map((s) => s.pid),
  );
  return state.deals.filter((d) => !final.has(d.stage_pid));
}

function forecastNow(query?: URLSearchParams): DemoReply {
  const now = forecastTotals();
  const convertTo = query?.get("convert_to") ?? null;
  const rateDate = query?.get("rate_date") ?? null;
  if (!convertTo && rateDate) return problem(422, "rate_date needs convert_to");
  let converted: Exclude<ReturnType<typeof convertSums>, string> | null = null;
  if (convertTo) {
    const done = convertSums(
      convertTo,
      rateDate,
      Object.entries(now.totals_minor).map(([currency, value]) => ({
        group: "forecast",
        currency,
        value,
      })),
    );
    if (typeof done === "string") return problem(422, done);
    converted = done;
  }
  return ok({
    ...now,
    convert_to: convertTo,
    converted_minor: converted ? (converted.rows[0]?.value ?? 0) : null,
    rate_date: converted?.rateDate ?? null,
    rates: converted?.rates ?? [],
    unconverted: converted?.unconverted ?? [],
  });
}

function forecastTotals() {
  const prob = new Map(state.stages.map((s) => [s.pid, s.probability_percent]));
  const open_ = openDeals();
  const total = open_.reduce(
    (sum, d) =>
      sum +
      Math.round(((d.amount_minor ?? 0) * (prob.get(d.stage_pid) ?? 0)) / 100),
    0,
  );
  return {
    as_of: new Date().toISOString(),
    open_deals: open_.length,
    totals_minor: { GBP: total },
  };
}

function salesDashboard() {
  const won = state.deals.filter((d) => d.won).length;
  const lost = state.deals.filter(
    (d) => state.stages.find((s) => s.pid === d.stage_pid)?.is_lost,
  ).length;
  const byStage: Record<string, { deals: number; value_minor: number }> = {};
  for (const stage of state.stages) {
    const inStage = state.deals.filter((d) => d.stage_pid === stage.pid);
    byStage[stage.name] = {
      deals: inStage.length,
      value_minor: inStage.reduce((s, d) => s + (d.amount_minor ?? 0), 0),
    };
  }
  return {
    win_rate: ratio(won, won + lost),
    open_deals: openDeals().length,
    pipeline_by_stage: byStage,
  };
}

function slaDashboard() {
  const openTickets = state.tickets.filter((t) => open(t.status));
  const priorities = ["urgent", "high", "normal", "low"];
  return {
    open_tickets: openTickets.length,
    by_priority: priorities.map((priority) => {
      const mine = openTickets.filter((t) => t.priority === priority);
      return {
        priority,
        open: mine.length,
        breached: mine.filter(
          (t) => t.live_first_response_breached || t.live_resolution_breached,
        ).length,
      };
    }),
  };
}

function followupRow(
  pid: string,
  kind: string,
  summary: string,
  who: string,
  recorder: string,
  dueDays: number,
) {
  return {
    pid,
    kind,
    summary,
    subject_kind: "contact",
    subject_pid: who,
    actor_ref: recorder,
    due_on: day(dueDays),
    overdue_days: dueDays < 0 ? -dueDays : null,
  };
}

function followups(kind: string | null) {
  const rows = state.followups
    .map((f) =>
      followupRow(
        f.pid,
        f.kind,
        f.summary,
        f.who,
        f.recorder,
        daysFromToday(f.due_on),
      ),
    )
    .filter((r) => !kind || r.kind === kind);
  return {
    as_of: new Date().toISOString(),
    note: "Open activities with a due date. Overdue ones are listed first.",
    overdue: rows.filter((r) => r.overdue_days !== null),
    upcoming_30d: rows.filter((r) => r.overdue_days === null),
    open_by_recorder: { "owner-ayo": 3, "owner-bea": 2 },
  };
}

function staleDeals(days: number) {
  const ages = new Map<string, number>([
    ["deal-1", 9],
    ["deal-2", 34],
    ["deal-3", 12],
    ["deal-4", 41],
    ["deal-5", 6],
    ["deal-6", 52],
    ["deal-7", 3],
    ["deal-8", 27],
  ]);
  const deals = openDeals().map((d) => ({
    pid: d.pid,
    name: d.name,
    stage: state.stages.find((s) => s.pid === d.stage_pid)?.name ?? null,
    owner_ref:
      d.pid.endsWith("1") || d.pid.endsWith("3") ? "owner-ayo" : "owner-bea",
    amount_minor: d.amount_minor ?? 0,
    currency: d.currency,
    days_in_stage: ages.get(d.pid) ?? 5,
    stale: (ages.get(d.pid) ?? 5) >= days,
  }));
  return {
    as_of: new Date().toISOString(),
    derivation: `A deal is stale when it has sat in its stage for ${days} days or more.`,
    threshold_days: days,
    open_deals: deals.length,
    stale_deals: deals.filter((d) => d.stale).length,
    deals,
  };
}

const NOTE = "Fictitious data for the guided tour.";

function insight(name: string, q: URLSearchParams): DemoReply | null {
  const as_of = new Date().toISOString();
  const days = (fallback: number) => Number(q.get("days")) || fallback;
  switch (name) {
    case "kb-deflection":
      return ok(kbDeflectionAnswer(days(30)));
    case "stale-deals":
      return ok(staleDeals(days(30)));
    case "followups":
      return ok(followups(q.get("kind")));
    case "pipeline-hygiene":
      return ok({
        as_of,
        threshold_days: days(30),
        findings: [
          {
            rule: "no_next_step",
            detail: "Case-management upgrade has no scheduled next step.",
            deal: "deal-6",
          },
          {
            rule: "stale_in_stage",
            detail:
              "Customer portal refresh has been in Discovery for 41 days.",
            deal: "deal-4",
          },
          {
            rule: "no_amount_confidence",
            detail: "Field-service mobile app has no decision date recorded.",
            deal: "deal-7",
          },
        ],
      });
    case "executive":
      return ok({
        as_of,
        window: { from: day(-30), to: day(0) },
        deals_won: 3,
        deals_lost: 1,
        won_value_by_currency_minor: { GBP: 16_750_000 },
        lost_reasons: { "Budget frozen": 1 },
        new_leads: 7,
        tickets_opened: 8,
        tickets_resolved: 2,
        campaigns_started: 2,
        activities_logged: 41,
        consent_withdrawals: 2,
        note: NOTE,
      });
    case "forecast-trends": {
      const base = forecastTotals().totals_minor.GBP ?? 0;
      return ok({
        as_of,
        note: NOTE,
        series: [-35, -28, -21, -14, -7, 0].map((offset, i) => ({
          taken_on: day(offset),
          totals: { GBP: Math.round(base * (0.62 + i * 0.075)) },
        })),
      });
    }
    case "sla": {
      const breached = state.tickets.filter(
        (t) =>
          open(t.status) &&
          (t.live_first_response_breached || t.live_resolution_breached),
      );
      return ok({
        as_of,
        derivation:
          "A ticket is breached when its first-response or resolution deadline has passed.",
        breaches: breached.map((t, i) => ({
          pid: t.pid,
          title: t.title,
          priority: t.priority,
          status: t.status,
          assignee_ref: i % 2 === 0 ? "owner-ayo" : "owner-bea",
          breached: t.live_resolution_breached
            ? "resolution"
            : "first_response",
          overdue_hours: 6 + i * 11,
        })),
        workload: [
          { assignee_ref: "owner-ayo", open: 3, breached: 1, at_risk_4h: 1 },
          { assignee_ref: "owner-bea", open: 3, breached: 1, at_risk_4h: 0 },
        ],
      });
    }
    case "dpo": {
      const coverage: Record<string, number> = {};
      for (const c of state.contacts)
        coverage[c.marketing_consent] =
          (coverage[c.marketing_consent] ?? 0) + 1;
      return ok({
        as_of,
        note: NOTE,
        contacts: state.contacts.length,
        consent_coverage: coverage,
        window_days: days(30),
        withdrawals_in_window: 2,
        consent_events_by_source: {
          "operator ui": 3,
          "unsubscribe-link": 2,
          import: 1,
        },
        duplicate_person_refs: [
          {
            person_ref: "person-demo-dup",
            contacts: [
              { pid: "con-4", display_name: "Tobias Marchetti" },
              { pid: "con-9", display_name: "Lucien Ashdown" },
            ],
          },
        ],
      });
    }
    case "cadence":
      return ok({
        as_of,
        derivation: `Contacts and accounts with no activity for ${days(30)} days.`,
        threshold_days: days(30),
        untouched_contacts: [
          {
            pid: "con-9",
            display_name: "Lucien Ashdown",
            stakeholder_role: "decision maker",
            days_since_touch: 64,
            has_next_touch: false,
          },
          {
            pid: "con-4",
            display_name: "Tobias Marchetti",
            stakeholder_role: "influencer",
            days_since_touch: 47,
            has_next_touch: false,
          },
          {
            pid: "con-8",
            display_name: "Dara Whitlock",
            stakeholder_role: null,
            days_since_touch: 38,
            has_next_touch: true,
          },
        ],
        untouched_accounts: [
          {
            pid: "acc-5",
            display_name: "Marlow & Finch Legal",
            stakeholder_role: "customer",
            days_since_touch: 58,
          },
        ],
        contacts_without_next_touch: 5,
      });
    case "engagement":
      return ok({
        as_of,
        window_days: days(30),
        touches: 41,
        per_recorder_month: { "owner-ayo": 24, "owner-bea": 17 },
        per_kind: { email: 18, call: 12, meeting: 7, note: 4 },
        sentiment: { positive: 14, neutral: 22, negative: 5 },
        note: NOTE,
      });
    case "funnel": {
      const entered = [24, 17, 11, 7, 4, 1];
      return ok({
        as_of,
        pipeline: PIPELINE,
        derivation:
          "Counts the deals that ever entered each stage, from the audit trail.",
        stages: state.stages.map((s, i) => ({
          stage: s.name,
          position: s.position,
          is_won: s.is_won,
          is_lost: s.is_lost,
          entered: entered[i] ?? 0,
          conversion_from_previous:
            i === 0 ? null : ratio(entered[i] ?? 0, entered[i - 1] ?? 0),
        })),
      });
    }
    case "members":
      return ok({
        as_of,
        derivation: `An account is silent after ${days(30)} days with no touch.`,
        threshold_days: days(30),
        silent_accounts: 1,
        accounts: state.accounts.map((a, i) => ({
          pid: a.pid,
          display_name: a.display_name,
          tier: a.tier,
          stakeholder_role: i % 2 === 0 ? "customer" : "partner",
          membership: {
            status: i === 4 ? "lapsed" : "active",
            joined_on: day(-400 + i * 30),
            renewal_on: i === 4 ? null : day(20 + i * 25),
          },
          contacts: state.contacts.filter((c) => c.account_pid === a.pid)
            .length,
          days_since_touch: i === 4 ? 58 : 4 + i * 5,
          silent: i === 4,
          open_followups: i % 3,
          open_tickets: i % 2,
        })),
      });
    case "consent-by-account":
      return ok({
        as_of,
        window_days: days(30),
        note: NOTE,
        accounts: state.accounts.map((a) => {
          const cov: Record<string, number> = {};
          for (const c of state.contacts.filter(
            (x) => x.account_pid === a.pid,
          )) {
            cov[c.marketing_consent] = (cov[c.marketing_consent] ?? 0) + 1;
          }
          return {
            pid: a.pid,
            display_name: a.display_name,
            consent_coverage: cov,
            withdrawals_in_window:
              a.pid === "acc-2" || a.pid === "acc-6" ? 1 : 0,
          };
        }),
      });
    case "stakeholders": {
      const row = (
        pid: string,
        influence: number,
        interest: number,
        since: number,
      ) => {
        const c = state.contacts.find((x) => x.pid === pid)!;
        return {
          pid,
          display_name: c.display_name,
          marketing_consent: c.marketing_consent,
          influence,
          interest,
          days_since_touch: since,
        };
      };
      return ok({
        as_of,
        note: NOTE,
        by_role: {
          "decision maker": [row("con-1", 9, 8, 3), row("con-9", 8, 3, 64)],
          influencer: [
            row("con-5", 7, 9, 4),
            row("con-3", 6, 7, 6),
            row("con-4", 5, 4, 47),
          ],
          user: [row("con-7", 4, 8, 11), row("con-10", 3, 6, 15)],
        },
        grid: {
          "manage closely": 3,
          "keep satisfied": 1,
          "keep informed": 3,
          monitor: 1,
        },
        stakeholders_without_grid_scores: 2,
        undeclared_contacts: 4,
        account_roles: [
          {
            pid: "acc-1",
            display_name: "Brightwater Energy",
            role: "customer",
          },
          {
            pid: "acc-3",
            display_name: "Foxglove Health Trust",
            role: "customer",
          },
          { pid: "acc-4", display_name: "Kestrel Software", role: "partner" },
        ],
      });
    }
    case "partnerships": {
      const register = [
        [
          "pt-1",
          "acc-4",
          "technology",
          "pilot",
          "Joint integration pilot with Kestrel's platform team",
          -90,
        ],
        [
          "pt-2",
          "acc-3",
          "research",
          "discovery",
          "Co-designing a clinical-data dashboard",
          -45,
        ],
        [
          "pt-3",
          "acc-1",
          "innovation",
          "scaling",
          "Grid-edge sensors rolled out to a second region",
          -200,
        ],
      ].map(([pid, account, kind, stage, summary, start]) => ({
        pid: pid as string,
        account_pid: account as string,
        account: nameOf(account as string),
        kind: kind as string,
        stage: stage as string,
        summary: summary as string,
        started_on: day(start as number),
      }));
      return ok({
        as_of,
        by_kind: { technology: 1, research: 1, innovation: 1 },
        by_stage: { discovery: 1, pilot: 1, scaling: 1 },
        register,
      });
    }
    case "memberships":
      return ok({
        as_of,
        window_days: days(60),
        memberships: 6,
        renewals_due: [
          {
            pid: "mem-1",
            account: "Harbourlight Logistics",
            status: "active",
            joined_on: day(-340),
            renewal_on: day(25),
          },
          {
            pid: "mem-2",
            account: "Kestrel Software",
            status: "active",
            joined_on: day(-310),
            renewal_on: day(41),
          },
        ],
        lapsed: [
          { pid: "mem-3", account: "Marlow & Finch Legal", status: "lapsed" },
        ],
      });
    default:
      return null;
  }
}

function editContact(pid: string, body: unknown): DemoReply {
  const contact = state.contacts.find((c) => c.pid === pid);
  if (!contact) return problem(404, "contact not found");
  const input = bodyOf(body);
  const names = [
    "display_name",
    "given_name",
    "family_name",
    "job_title",
  ] as const;
  if (!names.some((n) => input[n] !== undefined))
    return problem(422, "nothing to change");
  if (input.display_name !== undefined) {
    if (typeof input.display_name !== "string" || !input.display_name.trim())
      return problem(422, "display_name is required");
    contact.display_name = input.display_name.trim();
  }
  for (const n of ["given_name", "family_name", "job_title"] as const) {
    const v = input[n];
    if (v === null) contact[n] = null;
    else if (typeof v === "string") contact[n] = v.trim() || null;
  }
  return ok(contact);
}

function contactDetail(pid: string): DemoReply {
  const contact = state.contacts.find((c) => c.pid === pid);
  if (!contact) return problem(404, "contact not found");
  const link = state.links[pid] ?? { deals: [], tickets: [] };
  return ok({
    contact,
    activities: state.activities
      .filter((a) => a.subject_pid === pid)
      .sort((a, b) => b.occurred_at.localeCompare(a.occurred_at)),
    deals: state.deals.filter((d) => link.deals.includes(d.pid)),
    tickets: state.tickets.filter((t) => link.tickets.includes(t.pid)),
  });
}

/**
 * Likely duplicates of a contact. The tour has one pair (con-1 and con-13);
 * the figures are illustrative, in the shape the service returns.
 */
function contactDuplicates(pid: string): DemoReply {
  if (!state.contacts.some((c) => c.pid === pid))
    return problem(404, "contact not found");
  const other = { "con-1": "con-13", "con-13": "con-1" }[pid];
  const found = state.contacts.find((c) => c.pid === other);
  const field = (
    name: string,
    outcome: string,
    similarity: number,
    weight: number,
  ) => ({ field: name, outcome, similarity, weight });
  return ok({
    contact_pid: pid,
    compared: state.contacts.length - 1,
    truncated: false,
    duplicates: found
      ? [
          {
            contact: {
              pid: found.pid,
              display_name: found.display_name,
              account_pid: found.account_pid,
              job_title: found.job_title,
              status: found.status,
            },
            probability: 0.9985,
            verdict: "match",
            deterministic: false,
            email_checked: false,
            fields: [
              field("given_name", "agree", 1, 3.46),
              field("family_name", "agree", 0.96, 4.55),
              field("email", "missing", 0, 0),
              field("phone", "missing", 0, 0),
              field("organization", "agree", 1, 3.75),
              field("job_title", "partial", 0.8, 0.9),
            ],
          },
        ]
      : [],
  });
}

/**
 * Merge a contact into another (CRM-R24), in the shape the service answers.
 * The tour moves the deals and tickets it holds; every other table is empty
 * here. Consent follows the stricter side: withdrawn, then never, then granted.
 */
const LINK_DOMAINS: Record<string, string[]> = {
  hubspot: ["hubspot.com"],
  salesforce: ["salesforce.com", "force.com", "salesforce-setup.com"],
  attio: ["attio.com"],
  pipedrive: ["pipedrive.com"],
  dynamics: ["dynamics.com"],
  other: [],
};

/** The same rules as the service's `rules::external_links::check_url`. */
function checkLinkUrl(system: string, raw: string): string {
  if (!(system in LINK_DOMAINS))
    return `system must be one of ${Object.keys(LINK_DOMAINS).join(", ")}`;
  let url: URL;
  try {
    url = new URL(raw.trim());
  } catch {
    return "url is not a valid web address";
  }
  if (url.protocol !== "https:") return "url must start with https://";
  if (url.username || url.password)
    return "url must not contain a user name or password";
  const domains = LINK_DOMAINS[system]!;
  const host = url.hostname.toLowerCase();
  if (
    domains.length &&
    !domains.some((d) => host === d || host.endsWith(`.${d}`))
  )
    return `a ${system} link must be on one of: ${domains.join(", ")}`;
  return "";
}

function recordLinks(
  kind: "contact" | "account" | "deal" | "lead",
  pid: string,
  method: string,
  body: unknown,
): DemoReply {
  const pool: { pid: string }[] = {
    contact: state.contacts,
    account: state.accounts,
    deal: state.deals,
    lead: state.leads,
  }[kind];
  if (!pool.some((r) => r.pid === pid))
    return problem(404, `${kind} not found`);
  const list = (state.external[pid] ??= []);
  if (method === "GET")
    return ok({ links: list, systems: Object.keys(LINK_DOMAINS) });
  const { system, url, label } = (body ?? {}) as {
    system?: string;
    url?: string;
    label?: string | null;
  };
  const why = checkLinkUrl(String(system ?? ""), String(url ?? ""));
  if (why) return problem(422, why);
  const address = new URL(String(url).trim()).toString();
  if (list.some((l) => l.url === address))
    return problem(409, "this record already has that link");
  const row: ExternalLinkRow = {
    pid: `xl-${Date.now()}-${list.length}`,
    subject_kind: kind,
    subject_pid: pid,
    system: String(system),
    url: address,
    external_id: null,
    label: label?.trim() || null,
  };
  list.push(row);
  return ok(row);
}

function contactMerges(pid: string): DemoReply {
  return ok({
    merges: state.merged
      .filter((m) => m.survivor_pid === pid)
      .map((m) => ({
        merged_pid: m.contact.pid,
        merged_name: m.contact.display_name,
        merged_at: m.at,
        undoable: true,
      }))
      .reverse(),
  });
}

function unmergeContact(pid: string, body: unknown): DemoReply {
  const { reason, dry_run } = (body ?? {}) as {
    reason?: string;
    dry_run?: boolean;
  };
  if (!reason?.trim()) return problem(422, "reason is required");
  const entry = state.merged.find((m) => m.contact.pid === pid);
  if (!entry)
    return problem(
      409,
      "this contact has not been merged into another, or was already unmerged",
    );
  const survivor = state.contacts.find((c) => c.pid === entry.survivor_pid);
  if (!survivor) return problem(409, "the survivor has been deleted");
  const rank = (s: string) => (s === "withdrawn" ? 2 : s === "granted" ? 0 : 1);
  const result =
    rank(survivor.marketing_consent) > rank(entry.contact.marketing_consent)
      ? survivor.marketing_consent
      : entry.contact.marketing_consent;
  const reply = {
    survivor_pid: survivor.pid,
    merged_pid: pid,
    dry_run: dry_run === true,
    restored: {
      activities: 0,
      leads: 0,
      deals: entry.links.deals.length,
      tickets: entry.links.tickets.length,
      email_deliveries: 0,
      consent_events: 0,
      nurture_enrollments: 0,
      working_group_members: 0,
      external_links: entry.external.length,
    },
    left_behind: {},
    dropped_restored: { nurture_enrollments: 0, working_group_members: 0 },
    fields_cleared: [] as string[],
    fields_kept: [] as string[],
    consent: {
      survivor: survivor.marketing_consent,
      merged_before: entry.contact.marketing_consent,
      merged_result: result,
    },
  };
  if (dry_run === true) return ok(reply);
  const keep = state.links[survivor.pid] ?? { deals: [], tickets: [] };
  state.links[survivor.pid] = {
    deals: keep.deals.filter((d) => !entry.links.deals.includes(d)),
    tickets: keep.tickets.filter((t) => !entry.links.tickets.includes(t)),
  };
  state.links[pid] = entry.links;
  state.external[survivor.pid] = (state.external[survivor.pid] ?? []).filter(
    (l) => !entry.external.some((e) => e.pid === l.pid),
  );
  state.external[pid] = entry.external.map((l) => ({ ...l, subject_pid: pid }));
  entry.contact.marketing_consent = result;
  state.contacts.push(entry.contact);
  state.merged = state.merged.filter((m) => m !== entry);
  return ok(reply);
}

function removeLink(linkPid: string): DemoReply {
  for (const [contact, rows] of Object.entries(state.external)) {
    const keep = rows.filter((l) => l.pid !== linkPid);
    if (keep.length !== rows.length) {
      state.external[contact] = keep;
      return ok({ deleted: linkPid });
    }
  }
  return problem(404, "link not found");
}

function mergeContact(pid: string, body: unknown): DemoReply {
  const { into_pid, reason, dry_run } = (body ?? {}) as {
    into_pid?: string;
    reason?: string;
    dry_run?: boolean;
  };
  if (!reason?.trim()) return problem(422, "reason is required");
  if (pid === into_pid)
    return problem(422, "a contact cannot be merged into itself");
  const merged = state.contacts.find((c) => c.pid === pid);
  const survivor = state.contacts.find((c) => c.pid === into_pid);
  if (!merged || !survivor) return problem(404, "contact not found");
  const rank = (s: string) => (s === "withdrawn" ? 2 : s === "granted" ? 0 : 1);
  const result =
    rank(merged.marketing_consent) > rank(survivor.marketing_consent)
      ? merged.marketing_consent
      : survivor.marketing_consent;
  const from = state.links[pid] ?? { deals: [], tickets: [] };
  const filled = survivor.job_title
    ? []
    : merged.job_title
      ? ["job_title"]
      : [];
  const reply = {
    survivor_pid: survivor.pid,
    merged_pid: merged.pid,
    dry_run: dry_run === true,
    moved: {
      activities: 0,
      leads: 0,
      deals: from.deals.length,
      tickets: from.tickets.length,
      email_deliveries: 0,
      consent_events: 0,
      nurture_enrollments: 0,
      working_group_members: 0,
      external_links: (state.external[pid] ?? []).length,
    },
    dropped: { nurture_enrollments: 0, working_group_members: 0 },
    filled,
    consent: {
      survivor: survivor.marketing_consent,
      merged: merged.marketing_consent,
      result,
    },
  };
  if (dry_run === true) return ok(reply);
  const to = state.links[survivor.pid] ?? { deals: [], tickets: [] };
  state.links[survivor.pid] = {
    deals: [...to.deals, ...from.deals],
    tickets: [...to.tickets, ...from.tickets],
  };
  state.merged.push({
    contact: merged,
    survivor_pid: survivor.pid,
    links: from,
    external: state.external[pid] ?? [],
    at: new Date().toISOString(),
  });
  state.external[survivor.pid] = [
    ...(state.external[survivor.pid] ?? []),
    ...(state.external[pid] ?? []).map((l) => ({
      ...l,
      subject_pid: survivor.pid,
    })),
  ];
  delete state.external[pid];
  delete state.links[pid];
  if (filled.length) survivor.job_title = merged.job_title;
  survivor.marketing_consent = result;
  state.contacts = state.contacts.filter((c) => c.pid !== pid);
  return ok(reply);
}

function eraseContact(pid: string): DemoReply {
  const contact = state.contacts.find((c) => c.pid === pid);
  if (!contact) return problem(404, "contact not found");
  const link = state.links[pid] ?? { deals: [], tickets: [] };
  const finalStages = new Set(
    state.stages.filter((s) => s.is_won || s.is_lost).map((s) => s.pid),
  );
  if (
    state.deals.some(
      (d) => link.deals.includes(d.pid) && !finalStages.has(d.stage_pid),
    )
  ) {
    return problem(
      422,
      "This contact has an open deal. Close or reassign it before erasing.",
    );
  }
  if (
    state.tickets.some((t) => link.tickets.includes(t.pid) && open(t.status))
  ) {
    return problem(
      422,
      "This contact has an open ticket. Resolve it before erasing.",
    );
  }
  contact.display_name = "Erased contact";
  contact.job_title = null;
  contact.status = "erased";
  contact.marketing_consent = "withdrawn";
  return ok({
    erased: pid,
    note: "Personal details were anonymised (demo data).",
  });
}

function leadDetail(pid: string): DemoReply {
  const lead = state.leads.find((l) => l.pid === pid);
  if (!lead) return problem(404, "lead not found");
  const pool: Array<[string, number]> = [
    ["source_referral", 20],
    ["replied_within_week", 15],
    ["matches_target_industry", 12],
    ["senior_title", 10],
    ["opened_recent_campaign", 8],
    ["no_email_on_file", -6],
  ];
  const rules: Array<{ rule: string; points: number }> = [];
  let remaining = lead.score;
  for (const [rule, points] of pool) {
    if (remaining >= points && points > 0) {
      rules.push({ rule, points });
      remaining -= points;
    }
  }
  if (remaining > 0)
    rules.push({ rule: "engagement_history", points: remaining });
  const label = lead.score >= 70 ? "hot" : lead.score >= 40 ? "warm" : "cold";
  return ok({ lead, score: { score: lead.score, label, rules } });
}

function transition<T extends { status: string }>(
  rows: T[],
  pid: string,
  body: unknown,
  what: string,
): DemoReply {
  const row = rows.find((r) => (r as unknown as { pid: string }).pid === pid);
  if (!row) return problem(404, `${what} not found`);
  const to = (body as { to?: string } | null)?.to;
  if (!to) return problem(422, "`to` is required");
  row.status = to;
  return ok(row);
}

function moveDeal(pid: string, body: unknown): DemoReply {
  const deal = state.deals.find((d) => d.pid === pid);
  if (!deal) return problem(404, "deal not found");
  const stagePid = (body as { stage_pid?: string } | null)?.stage_pid;
  const stage = state.stages.find((s) => s.pid === stagePid);
  if (!stage) return problem(422, "unknown stage");
  deal.stage_pid = stage.pid;
  deal.won = stage.is_won;
  deal.closed_at =
    stage.is_won || stage.is_lost ? new Date().toISOString() : null;
  return ok(deal);
}

/** Put `pid` at `position` among the rows that share its column, keeping every other row's slot. */
function placeInColumn<T extends { pid: string }>(
  rows: T[],
  pid: string,
  position: number,
  column: (row: T) => string,
): DemoReply {
  const row = rows.find((r) => r.pid === pid);
  if (!row) return problem(404, "not found");
  const slots = rows.flatMap((r, i) => (column(r) === column(row) ? [i] : []));
  const order = slots.map((i) => rows[i]!).filter((r) => r.pid !== pid);
  order.splice(Math.max(0, Math.min(position, order.length)), 0, row);
  slots.forEach((slot, i) => {
    rows[slot] = order[i]!;
  });
  return ok(row);
}

function bodyOf(body: unknown): Record<string, unknown> {
  return (body ?? {}) as Record<string, unknown>;
}

function reopenDeal(pid: string, body: unknown): DemoReply {
  const deal = state.deals.find((d) => d.pid === pid);
  if (!deal) return problem(404, "deal not found");
  const reason = (bodyOf(body).reason as string | undefined) ?? "";
  if (!reason.trim()) return problem(422, "a reopen requires a reason");
  if (!deal.closed_at) return problem(422, "deal is not closed");
  const open = [...state.stages].reverse().find((s) => !s.is_won && !s.is_lost);
  if (!open) return problem(422, "pipeline has no open stage");
  deal.stage_pid = open.pid;
  deal.closed_at = null;
  deal.won = false;
  return ok(deal);
}

function editDeal(pid: string, body: unknown): DemoReply {
  const deal = state.deals.find((d) => d.pid === pid);
  if (!deal) return problem(404, "deal not found");
  if (deal.closed_at) return problem(422, "deal is closed — reopen it first");
  const { name, amount_minor, expected_close_on } = bodyOf(body);
  if (
    name === undefined &&
    amount_minor === undefined &&
    expected_close_on === undefined
  ) {
    return problem(422, "nothing to change");
  }
  if (name !== undefined) {
    if (typeof name !== "string" || !name.trim())
      return problem(422, "name is required");
    deal.name = name.trim();
  }
  if (amount_minor !== undefined) {
    if (typeof amount_minor !== "number" || amount_minor < 0) {
      return problem(422, "amount_minor must not be negative");
    }
    deal.amount_minor = Math.round(amount_minor);
  }
  if (typeof expected_close_on === "string")
    deal.expected_close_on = expected_close_on;
  return ok(deal);
}

function editText<T extends { pid: string }>(
  rows: T[],
  pid: string,
  body: unknown,
  field: keyof T & string,
): DemoReply {
  const row = rows.find((r) => r.pid === pid);
  if (!row) return problem(404, "not found");
  const value = bodyOf(body)[field];
  if (value === undefined) return problem(422, "nothing to change");
  if (typeof value !== "string" || !value.trim())
    return problem(422, `${field} is required`);
  (row as Record<string, unknown>)[field] = value.trim();
  return ok(row);
}

function editFollowup(pid: string, body: unknown): DemoReply {
  const f = state.followups.find((x) => x.pid === pid);
  if (!f) return problem(404, "activity not found");
  const { due_on, summary } = bodyOf(body);
  if (due_on === undefined && summary === undefined)
    return problem(422, "nothing to change");
  if (due_on !== undefined) {
    if (typeof due_on !== "string" || Number.isNaN(Date.parse(due_on))) {
      return problem(422, "due_on must be a date");
    }
    f.due_on = due_on;
  }
  if (typeof summary === "string" && summary.trim()) f.summary = summary.trim();
  return ok(f);
}

function runCampaign(pid: string): DemoReply {
  const c = state.campaigns.find((x) => x.pid === pid);
  if (!c) return problem(404, "campaign not found");
  if (c.status !== "scheduled" && c.status !== "running") {
    return problem(422, "Only a scheduled or running campaign can be run.");
  }
  if (c.recipients === 0) c.recipients = 420;
  c.delivered = Math.round(c.recipients * 0.97);
  c.opened = Math.round(c.delivered * 0.48);
  c.clicked = Math.round(c.delivered * 0.11);
  c.status = "completed";
  return ok(c);
}

function campaignFunnel(pid: string): DemoReply {
  const campaign = state.campaigns.find((c) => c.pid === pid);
  if (!campaign) return problem(404, "campaign not found");
  const index = state.campaigns.indexOf(campaign);
  const leads = [14, 9, 3, 0][index] ?? 0;
  const won = [3, 1, 0, 0][index] ?? 0;
  const revenue = [16_750_000, 5_200_000, 0, 0][index] ?? 0;
  return ok({
    campaign,
    leads,
    won_deals: won,
    won_revenue_minor: revenue,
    roi: campaign.cost_minor
      ? ratio(revenue - campaign.cost_minor, campaign.cost_minor)
      : null,
    // The tour's won revenue is all in the campaign's currency, so there is nothing to convert.
    won_revenue_other_currencies: {},
    rate_date: null,
    rates: [],
    unconverted: [],
  });
}

function breakGlass(
  method: string,
  path: string[],
  body: unknown,
): DemoReply | null {
  if (path.length === 1 && method === "GET") {
    const grants = [...state.grants].sort((a, b) =>
      b.granted_at.localeCompare(a.granted_at),
    );
    return ok({
      unreviewed: grants.filter((g) => !g.reviewed_at).length,
      grants,
    });
  }
  if (path.length === 1 && method === "POST") {
    const { reason, minutes } = (body ?? {}) as {
      reason?: string;
      minutes?: number;
    };
    if (!reason || reason.trim().length < 20) {
      return problem(422, "Give a reason of at least 20 characters.");
    }
    const mins = Math.min(Math.max(minutes ?? 15, 1), 60);
    const now = Date.now();
    const grant: Grant = {
      pid: `bg-${state.grants.length + 1}`,
      subject: "example@example.com",
      reason: reason.trim(),
      granted_at: new Date(now).toISOString(),
      expires_at: new Date(now + mins * 60_000).toISOString(),
      reviewed_at: null,
      reviewed_by: null,
      review_note: null,
    };
    state.grants.push(grant);
    return ok({ pid: grant.pid, expires_at: grant.expires_at, minutes: mins });
  }
  if (path.length === 3 && path[2] === "review" && method === "POST") {
    const grant = state.grants.find((g) => g.pid === path[1]);
    if (!grant) return problem(404, "grant not found");
    if (grant.subject === "example@example.com") {
      return problem(422, "You cannot review your own emergency access.");
    }
    if (grant.reviewed_at)
      return problem(422, "This grant has already been reviewed.");
    grant.reviewed_at = new Date().toISOString();
    grant.reviewed_by = "example@example.com";
    grant.review_note = String((body as { note?: string } | null)?.note ?? "");
    return ok({ pid: grant.pid, reviewed_by: grant.reviewed_by });
  }
  return null;
}

/** The tour's lead import (CRM-R34): a plain comma split, no quoting, nothing is saved. */
/** The tour's contact and account imports: the lead import's answers, with this kind's mapping. */
function recordImports(
  contacts: boolean,
  method: string,
  id: string | undefined,
  tail: string | undefined,
  body: unknown,
): DemoReply {
  const reply = leadImports(method, id, tail, body);
  if (reply.status !== 200) return reply;
  const out = reply.body as Record<string, unknown>;
  if (method === "POST" && id === "preview") {
    const base = out.suggested_mapping as Record<string, string | null>;
    out.suggested_mapping = contacts
      ? {
          display_name: base.display_name,
          email: base.email,
          given_name: null,
          family_name: null,
          job_title: null,
        }
      : {
          display_name: base.display_name ?? base.company,
          organization_ref: null,
          tier: null,
          industry: null,
        };
  } else if (method === "POST" && id === undefined) {
    out.kind = contacts ? "contact" : "account";
    out.failed_count = 0;
  }
  return reply;
}

function leadImports(
  method: string,
  id: string | undefined,
  tail: string | undefined,
  body: unknown,
): DemoReply {
  if (method === "POST" && id === "preview") {
    const { csv, mapping } = (body ?? {}) as {
      csv?: string;
      mapping?: { display_name?: string; email?: string };
    };
    const lines = (csv ?? "").split(/\r?\n/).filter((l) => l.trim() !== "");
    if (lines.length === 0) return problem(422, "the file is empty");
    const columns = lines[0]!.split(",").map((c) => c.trim());
    const guess = (...names: string[]) =>
      columns.find((c) => names.includes(c.toLowerCase())) ?? null;
    const suggested = {
      display_name: guess("name", "full name", "display name"),
      email: guess("email", "e-mail"),
      company: guess("company", "organisation", "organization"),
    };
    if (!mapping) {
      return ok({
        columns,
        rows: lines.length - 1,
        suggested_mapping: suggested,
        report: null,
        valid: 0,
        invalid: 0,
        likely_duplicates: 0,
      });
    }
    const at = columns.indexOf(mapping.display_name ?? "");
    if (at < 0) return problem(422, "display_name must be mapped to a column");
    const known = state.contacts.map((c) => c.display_name.toLowerCase());
    const report = lines.slice(1).map((line, i) => {
      const name = (line.split(",")[at] ?? "").trim();
      const twin = known.find((k) => name !== "" && k === name.toLowerCase());
      return {
        row: i + 1,
        status: name === "" ? "invalid" : "valid",
        reason: name === "" ? "has no name" : null,
        name: name === "" ? null : name,
        likely_duplicates: twin
          ? [
              {
                kind: "contact",
                pid: null,
                row: null,
                name: twin,
                probability: 0.97,
                verdict: "match",
              },
            ]
          : [],
      };
    });
    return ok({
      columns,
      rows: report.length,
      suggested_mapping: suggested,
      report,
      valid: report.filter((r) => r.status === "valid").length,
      invalid: report.filter((r) => r.status === "invalid").length,
      likely_duplicates: report.filter((r) => r.likely_duplicates.length > 0)
        .length,
    });
  }
  if (method === "POST" && id === undefined) {
    const preview = leadImports("POST", "preview", undefined, body);
    if (preview.status !== 200) return preview;
    const p = preview.body as { rows: number; valid: number; invalid: number };
    const f = body as { source_note: string; obtained_on: string };
    return ok({
      pid: "import-1",
      source_note: f.source_note,
      obtained_on: f.obtained_on,
      status: "completed",
      row_count: p.rows,
      invalid_count: p.invalid,
      skipped_count: 0,
      created_count: p.valid,
      undone_count: null,
    });
  }
  if (method === "POST" && tail === "undo") return ok({ removed: 2, kept: 0 });
  return problem(404, "not found");
}

/**
 * Answer one API call. `path` is the service path without `/api/proxy/`
 * (for example `deals/deal-1/stage`).
 */
export function demoHandle(
  method: string,
  path: string,
  query: URLSearchParams,
  body: unknown,
): DemoReply {
  const seg = path.replace(/^\/+|\/+$/g, "").split("/");
  const [head, id, tail] = seg;
  const get = method === "GET";
  const post = method === "POST";

  if (head === "contacts") {
    if (seg.length === 1 && get) return ok(taggedOnly(state.contacts, query));
    if (seg.length === 2 && get) return contactDetail(id!);
    if (tail === "duplicates" && get) return contactDuplicates(id!);
    if (tail === "consent" && post) {
      const contact = state.contacts.find((c) => c.pid === id);
      if (!contact) return problem(404, "contact not found");
      contact.marketing_consent =
        (body as { action?: string } | null)?.action ??
        contact.marketing_consent;
      return ok(contact);
    }
    if (seg.length === 2 && method === "PUT") return editContact(id!, body);
    if (tail === "erase" && post) return eraseContact(id!);
    if (tail === "merge" && post) return mergeContact(id!, body);
    if (tail === "merges" && get) return contactMerges(id!);
    if (tail === "unmerge" && post) return unmergeContact(id!, body);
    if (tail === "external-links" && (get || post))
      return recordLinks("contact", id!, method, body);
  }
  if (head === "kb") return kbPublic(method, seg, query, body);
  if (head === "commission-plans") return commissionPlans(method, id, body);
  if (head === "quotas") return quotaRoutes(method, seg, query, body);
  if (head === "email-templates") return emailTemplates(method, id, body);
  if (head === "custom-fields") return customFieldDefs(method, id, body);
  if (tail === "custom-fields" && id)
    return customFieldValues(head, id, method, body);
  if (tail === "attachments" && id) return attachmentList(id, method, query);
  if (head === "tags" && !id && method === "GET") return ok(demoTags());
  if (head === "reports") return reports(method, id, tail, body);
  if (head === "fx-rates") return fxRates(method, id, body);
  if (head === "attribution" && get) return attributionAnswer(query);
  if (tail === "tags" && id) return recordTags(id, method, body);
  if (head === "attachments" && id && method === "DELETE")
    return attachmentDelete(id);
  if (head === "workflows") return workflows(method, id, tail, body);
  if (head === "views") {
    if (get)
      return ok(
        state.views.filter((v) => v.kind === (query.get("kind") ?? v.kind)),
      );
    const input = (body ?? {}) as Partial<SavedView>;
    if (post) {
      if (!input.name?.trim()) return problem(422, "name is required");
      const made: SavedView = {
        pid: `view-${state.views.length + 1}-${Date.now()}`,
        owner_ref: "worker:demo",
        mine: true,
        kind: input.kind ?? "contact",
        name: input.name.trim(),
        filter: input.filter ?? {},
        sort: input.sort ?? null,
        columns: input.columns ?? null,
        shared: input.shared ?? false,
      };
      state.views.push(made);
      return ok({ pid: made.pid });
    }
    const at = state.views.findIndex((v) => v.pid === id);
    if (at < 0) return problem(404, "view not found");
    if (!state.views[at]!.mine)
      return problem(403, "only the owner can change a view");
    if (method === "PUT") {
      state.views[at] = { ...state.views[at]!, ...input, mine: true, pid: id! };
      return ok({ pid: id });
    }
    if (method === "DELETE") {
      state.views.splice(at, 1);
      return ok({ pid: id });
    }
  }
  if (head === "lead-imports") return leadImports(method, id, tail, body);
  if (head === "contact-imports" || head === "account-imports")
    return recordImports(head === "contact-imports", method, id, tail, body);
  if (head === "search" && get) {
    const q = query.get("q") ?? "";
    if (q.trim().length < 2) return ok([]);
    const hit = (
      kind: string,
      pid: string,
      title: string,
      detail: string | null | undefined,
      fields: (string | null | undefined)[],
    ) =>
      matches(fields, q) ? [{ kind, pid, title, detail: detail ?? null }] : [];
    return ok([
      ...state.contacts.flatMap((c) =>
        hit("contact", c.pid, c.display_name, c.job_title, [
          c.display_name,
          c.job_title,
        ]),
      ),
      ...state.accounts.flatMap((a) =>
        hit("account", a.pid, a.display_name, a.industry, [
          a.display_name,
          a.industry,
        ]),
      ),
      ...state.leads.flatMap((l) =>
        hit("lead", l.pid, l.display_name, null, [l.display_name]),
      ),
      ...state.deals.flatMap((d) => hit("deal", d.pid, d.name, null, [d.name])),
      ...state.tickets.flatMap((x) =>
        hit("ticket", x.pid, x.title, x.status, [x.title]),
      ),
      ...state.campaigns.flatMap((c) =>
        hit("campaign", c.pid, c.name, c.status, [c.name]),
      ),
      ...state.articles.flatMap((a) =>
        hit("article", a.pid, a.title, a.status, [a.title, a.body]),
      ),
    ]);
  }
  if (head === "notifications") {
    const unread = state.notifications.filter((n) => n.read_at === null);
    if (seg.length === 1 && get)
      return ok(
        query.get("unread") === "true" ? unread : [...state.notifications],
      );
    if (id === "unread-count" && get)
      return ok({ unread: unread.length, capped: false });
    if (id === "read-all" && post) {
      for (const n of unread) n.read_at = new Date().toISOString();
      return ok({ marked: unread.length });
    }
    if (tail === "read" && method === "PUT") {
      const found = state.notifications.find((n) => n.pid === id);
      if (!found) return problem(404, "notification not found");
      found.read_at ??= new Date().toISOString();
      return ok(found);
    }
  }
  if (head === "external-links" && method === "DELETE" && id)
    return removeLink(id);
  if (head === "accounts" && seg.length === 1 && get)
    return ok(taggedOnly(state.accounts, query));
  if (head === "accounts" && seg.length === 2 && method === "PUT") {
    const account = state.accounts.find((a) => a.pid === id);
    if (!account) return problem(404, "account not found");
    const input = bodyOf(body);
    if (
      !["display_name", "tier", "industry"].some((n) => input[n] !== undefined)
    )
      return problem(422, "nothing to change");
    if (input.display_name !== undefined) {
      if (typeof input.display_name !== "string" || !input.display_name.trim())
        return problem(422, "display_name is required");
      account.display_name = input.display_name.trim();
    }
    if (typeof input.tier === "string") account.tier = input.tier;
    if (input.industry === null) account.industry = null;
    else if (typeof input.industry === "string")
      account.industry = input.industry.trim() || null;
    return ok(account);
  }
  if (head === "accounts" && seg.length === 3 && tail === "clv" && get)
    return accountClv(id!, query);
  if (head === "accounts" && seg.length === 2 && get) {
    const account = state.accounts.find((a) => a.pid === id);
    if (!account) return problem(404, "account not found");
    const people = state.contacts.filter((c) => c.account_pid === account.pid);
    // The demo ties deals to contacts, so an account's deals are its contacts' deals.
    const dealPids = new Set(
      people.flatMap((c) => state.links[c.pid]?.deals ?? []),
    );
    return ok({
      account,
      contacts: people.map((c) => ({
        pid: c.pid,
        display_name: c.display_name,
      })),
      deals: state.deals.filter((d) => dealPids.has(d.pid)),
    });
  }
  if (tail === "external-links" && (get || post)) {
    const kinds = {
      accounts: "account",
      deals: "deal",
      leads: "lead",
    } as const;
    const kind = kinds[head as keyof typeof kinds];
    if (kind) return recordLinks(kind, id!, method, body);
  }
  if (head === "leads") {
    if (seg.length === 1 && get) return ok(taggedOnly(state.leads, query));
    if (seg.length === 2 && get) return leadDetail(id!);
    if (tail === "status" && post)
      return transition(state.leads, id!, body, "lead");
    if (method === "PUT" && seg.length === 2)
      return editText(state.leads, id!, body, "display_name");
    if (method === "PUT" && tail === "position") {
      return placeInColumn(
        state.leads,
        id!,
        Number(bodyOf(body).position),
        (l) => l.status,
      );
    }
  }
  if (head === "pipelines" && get) {
    return ok([{ pipeline: PIPELINE, stages: state.stages }]);
  }
  if (head === "deals") {
    if (seg.length === 1 && get) {
      const pipeline = query.get("pipeline");
      return ok(pipeline && pipeline !== PIPELINE.pid ? [] : state.deals);
    }
    if (tail === "stage" && post) return moveDeal(id!, body);
    if (tail === "reopen" && post) return reopenDeal(id!, body);
    if (method === "PUT" && seg.length === 2) return editDeal(id!, body);
    if (method === "PUT" && tail === "position") {
      return placeInColumn(
        state.deals,
        id!,
        Number(bodyOf(body).position),
        (d) => d.stage_pid,
      );
    }
  }
  if (head === "activities" && method === "PUT" && seg.length === 2)
    return editFollowup(id!, body);
  if (head === "forecast" && get) return forecastNow(query);
  if (head === "segments" && get) return ok(state.segments);
  if (head === "campaigns") {
    if (seg.length === 1 && get) return ok(state.campaigns);
    if (seg.length === 1 && post) {
      const input = (body ?? {}) as {
        name?: string;
        template_pid?: string;
        subject?: string;
        body?: string;
      };
      if (!input.name?.trim()) return problem(422, "name is required");
      if (input.template_pid && (input.subject || input.body))
        return problem(
          422,
          "template_pid cannot be given together with a subject or body",
        );
      const made = {
        pid: `camp-${state.campaigns.length + 1}-${Date.now()}`,
        name: input.name.trim(),
        status: "draft",
        cost_minor: 0,
        currency: "GBP",
        recipients: 0,
        delivered: 0,
        opened: 0,
        clicked: 0,
      } satisfies Campaign;
      state.campaigns.push(made);
      return ok({ pid: made.pid });
    }
    if (tail === "funnel" && get) return campaignFunnel(id!);
    if (tail === "run" && post) return runCampaign(id!);
    if (tail === "status" && post)
      return transition(state.campaigns, id!, body, "campaign");
  }
  if (head === "tickets") {
    if (seg.length === 1 && get) return ok(state.tickets);
    if (method === "PUT" && seg.length === 2)
      return editText(state.tickets, id!, body, "title");
    if (method === "PUT" && tail === "position") {
      return placeInColumn(
        state.tickets,
        id!,
        Number(bodyOf(body).position),
        (t) => t.status,
      );
    }
    if (tail === "customer-reply" && post) {
      // A waiting ticket goes back to open and its clock runs again (CRM-R63).
      const t = state.tickets.find((x) => x.pid === id);
      if (!t) return problem(404, "ticket not found");
      if (t.status === "pending") {
        t.status = "open";
        t.paused_at = null;
        t.wait_limit_exceeded_at = null;
      } else if (t.status !== "open")
        return problem(
          422,
          `a customer reply does not reopen a ${t.status} ticket`,
        );
      return ok(t);
    }
    if (tail === "suggested-articles" && get) {
      const t = state.tickets.find((x) => x.pid === id);
      if (!t) return problem(404, "ticket not found");
      return ok(suggestArticles(t.title));
    }
    if (tail === "status" && post) {
      const reply = transition(state.tickets, id!, body, "ticket");
      const t = state.tickets.find((x) => x.pid === id);
      // The SLA clock stops while a ticket waits for the customer (CRM-R55).
      if (t)
        t.paused_at = t.status === "pending" ? new Date().toISOString() : null;
      if (t && t.status !== "pending") t.wait_limit_exceeded_at = null;
      if (t && !open(t.status)) {
        t.live_first_response_breached = false;
        t.live_resolution_breached = false;
      }
      return reply;
    }
  }
  if (head === "articles") {
    if (seg.length === 1 && get) {
      const q = (query.get("q") ?? "").trim().toLowerCase();
      return ok(
        q
          ? state.articles.filter((a) =>
              `${a.title} ${a.body}`.toLowerCase().includes(q),
            )
          : state.articles,
      );
    }
    if (tail === "status" && post) {
      const reply = transition(state.articles, id!, body, "article");
      const a = state.articles.find((x) => x.pid === id);
      if (a && reply.status === 200) a.version += 1;
      return reply;
    }
  }
  if (head === "dashboards" && get) {
    if (id === "sales") return ok(salesDashboard());
    if (id === "sla") return ok(slaDashboard());
  }
  if (head === "insights" && get && id) {
    const reply = insight(id, query);
    if (reply) return reply;
  }
  if (head === "break-glass") {
    const reply = breakGlass(method, seg, body);
    if (reply) return reply;
  }
  return problem(
    404,
    "This endpoint is not part of the guided tour's sample data.",
  );
}

const MERGE_TOKENS: MergeToken[] = [
  ["given_name", "The first word of the contact's name", "there"],
  ["name", "The contact's full name", "there"],
  [
    "family_name",
    "The contact's family name, when one is recorded",
    "customer",
  ],
  ["account_name", "The name of the contact's account", "your organisation"],
  ["rep_name", "The name of the contact's owner, or the account's", "the team"],
  ["job_title", "The contact's job title", "your role"],
].map(([name, description, fallback]) => ({
  name: name!,
  description: description!,
  fallback: fallback!,
  required: false,
}));
MERGE_TOKENS.push({
  name: "unsubscribe_url",
  description:
    "The contact's signed unsubscribe link; every body must contain it",
  fallback: null,
  required: true,
});

const SYNTHETIC: Record<string, string> = {
  given_name: "Alex",
  name: "Alex Example",
  family_name: "Example",
  account_name: "Example Ltd",
  rep_name: "Sam Rep",
  job_title: "Operations Lead",
};

/** Render a draft the way the service does, for the made-up contact (text only). */
function renderDraft(
  draft: { subject: string; body: string; fallbacks: Record<string, string> },
  blank: string[],
): TemplatePreview | { unknown: string } {
  const fell: string[] = [];
  let unknown: string | null = null;
  const fill = (source: string) =>
    source.replace(/\{\{\s*([a-z_]+)\s*\}\}/g, (_all, name: string) => {
      if (name === "unsubscribe_url")
        return "https://example.invalid/unsubscribe/preview";
      const token = MERGE_TOKENS.find((t) => t.name === name);
      if (!token) {
        unknown = name;
        return "";
      }
      const value = blank.includes(name) ? undefined : SYNTHETIC[name];
      if (value) return value;
      if (!fell.includes(name)) fell.push(name);
      return draft.fallbacks[name] || token.fallback!;
    });
  const subject = fill(draft.subject);
  const text = fill(draft.body);
  if (unknown) return { unknown };
  const problems: string[] = [];
  if (!/\{\{\s*unsubscribe_url\s*\}\}/.test(draft.body))
    problems.push(
      "body must contain {{unsubscribe_url}}, the link a recipient uses to unsubscribe",
    );
  return { subject, text, html: text, fell_back: fell, problems };
}

/** The demo's workflow rules: stored and listed, with a dry run over the sample contacts. */
function workflows(
  method: string,
  id: string | undefined,
  tail: string | undefined,
  body: unknown,
) {
  const input = (body ?? {}) as Partial<Workflow>;
  if (method === "GET" && !id) return ok(state.workflows);
  if (method === "POST" && !id) {
    if (!input.name?.trim()) return problem(422, "name is required");
    if (!input.actions?.length)
      return problem(422, "actions needs one to 5 actions");
    const made: Workflow = {
      pid: `wf-${state.workflows.length + 1}-${Date.now()}`,
      name: input.name.trim(),
      enabled: input.enabled ?? true,
      trigger: input.trigger ?? "contact_created",
      every_minutes: input.every_minutes ?? null,
      conditions: input.conditions ?? {
        statuses: [],
        channels: [],
        account_tiers: [],
        sources: [],
      },
      actions: input.actions,
      updated_at: new Date().toISOString(),
    };
    state.workflows.push(made);
    return ok({ pid: made.pid });
  }
  const at = state.workflows.findIndex((x) => x.pid === id);
  if (tail === "runs") {
    return ok(state.workflowRuns.filter((r) => r.workflow_pid === id));
  }
  if (at < 0) return problem(404, "workflow not found");
  const found = state.workflows[at]!;
  if (tail === "dry-run" && method === "POST") {
    const channels = found.conditions.channels;
    const meets = state.contacts.filter(
      (c) => !channels.length || channels.includes(c.preferred_channel),
    );
    return ok({
      examined: state.contacts.length,
      would_fire: meets.length,
      truncated: false,
      // One contact without consent is shown first, so the re-check is visible.
      samples: [
        ...meets.filter((c) => c.marketing_consent !== "granted").slice(0, 1),
        ...meets.filter((c) => c.marketing_consent === "granted").slice(0, 2),
      ].map((c) => ({
        subject_kind: "contact",
        subject_pid: c.pid,
        steps: found.actions.map((a) => ({
          action: a.type.replace(/_/g, " "),
          result:
            a.type === "enrol_nurture" && c.marketing_consent !== "granted"
              ? "would_skip"
              : "would_do",
          detail:
            a.type === "enrol_nurture" && c.marketing_consent !== "granted"
              ? "no marketing consent at this moment"
              : "",
        })),
      })),
    });
  }
  if (method === "GET") return ok(found);
  if (method === "PUT") {
    state.workflows[at] = {
      ...found,
      ...input,
      pid: id!,
      updated_at: new Date().toISOString(),
    };
    return ok({ pid: id });
  }
  if (method === "DELETE") {
    state.workflows.splice(at, 1);
    return ok({ pid: id });
  }
  return problem(405, "method not allowed");
}

/** The tour's reports: computed over the sample records, as the service computes them over its own. */
function reportRows(spec: ReportSpec): ReportResult["rows"] | string {
  type Rec = Record<string, unknown>;
  const none = "(none)";
  const text = (v: unknown, fallback = none) =>
    v == null || v === "" ? fallback : String(v);
  const tagsOf = (r: Rec) => state.tagged[String(r.pid)] ?? [];
  const source: Rec[] = (
    {
      contact: state.contacts,
      account: state.accounts,
      lead: state.leads,
      deal: state.deals,
      ticket: state.tickets,
    } as Record<string, unknown[]>
  )[spec.kind] as Rec[];
  if (!source) return "kind is not a kind of record";
  const stageName = (pid: unknown) =>
    state.stages.find((x) => x.pid === pid)?.name ?? "(unknown)";
  const tierOf = (r: Rec) =>
    text(
      state.accounts.find((a) => a.pid === r.account_pid)?.tier,
      "(no account)",
    );
  const pick: Record<string, (r: Rec) => string> = {
    status: (r) => text(r.status),
    marketing_consent: (r) => text(r.marketing_consent),
    account_tier: tierOf,
    owner: (r) => text(r.owner_ref),
    created_month: () => "2026-10",
    tier: (r) => text(r.tier),
    industry: (r) => text(r.industry),
    source: (r) => text(r.source),
    stage: (r) => stageName(r.stage_pid),
    state: (r) => (r.closed_at ? (r.won ? "won" : "lost") : "open"),
    currency: (r) => text(r.currency),
    pipeline: () => PIPELINE.name,
    closed_month: (r) =>
      r.closed_at ? String(r.closed_at).slice(0, 7) : "(open)",
    priority: (r) => text(r.priority),
    channel: (r) => text(r.channel),
    assignee: (r) => text(r.assignee_ref),
    opened_month: () => "2026-10",
  };
  const group = (r: Rec): string[] =>
    spec.group_by === "tag"
      ? tagsOf(r).length
        ? tagsOf(r)
        : ["(no tag)"]
      : [pick[spec.group_by]?.(r) ?? none];
  const kept = source.filter((r) =>
    Object.entries(spec.filter).every(([field, values]) => {
      const held = field === "tag" ? tagsOf(r) : [pick[field]?.(r) ?? none];
      return held.some((h) =>
        values.some((v) => v.toLowerCase() === h.toLowerCase()),
      );
    }),
  );
  const sums = new Map<string, number>();
  for (const r of kept) {
    for (const g of group(r)) {
      const money = spec.measure === "sum_amount";
      const key = money ? `${g}\u0000${text(r.currency)}` : g;
      sums.set(
        key,
        (sums.get(key) ?? 0) + (money ? Number(r.amount_minor ?? 0) : 1),
      );
    }
  }
  return [...sums.entries()]
    .map(([key, value]) => {
      const [g, currency] = key.split("\u0000");
      return { group: g!, currency: currency ?? null, value };
    })
    .sort((a, b) => b.value - a.value || a.group.localeCompare(b.group));
}

function reportResult(spec: ReportSpec): DemoReply {
  const kinds = ["contact", "account", "lead", "deal", "ticket"];
  if (!kinds.includes(spec.kind))
    return problem(422, "kind is not a kind of record");
  if (spec.measure === "sum_amount" && spec.kind !== "deal")
    return problem(422, "only a deal report can sum amounts");
  const rows = reportRows(spec);
  if (typeof rows === "string") return problem(422, rows);
  const converting = spec.convert_to
    ? convertReport(spec, rows)
    : { rows, rateDate: null, rates: [], unconverted: [] };
  if (typeof converting === "string") return problem(422, converting);
  return ok({
    spec,
    as_of: new Date().toISOString(),
    rows: converting.rows.slice(0, 200),
    truncated: converting.rows.length > 200,
    rate_date: converting.rateDate,
    rates: converting.rates,
    unconverted: converting.unconverted,
    note: "counted over the records that exist now; sums are per currency and never added across currencies",
  } satisfies ReportResult);
}

const ONE = 1_000_000_000n;

/** A rate such as `1.27` as billionths, or null when it is not a positive decimal below 1,000,000. */
function ppbOf(text: string): bigint | null {
  if (!/^\d{1,6}(\.\d{1,9})?$/.test(text)) return null;
  const [whole, fraction = ""] = text.split(".");
  const ppb = BigInt(whole!) * ONE + BigInt(fraction.padEnd(9, "0"));
  return ppb > 0n && BigInt(whole!) < 1_000_000n ? ppb : null;
}

/** `n / d` rounded half to even. */
function divideRound(n: bigint, d: bigint): bigint {
  const q = n / d;
  const twice = (n % d < 0n ? -(n % d) : n % d) * 2n;
  const step = n < 0n ? -1n : 1n;
  return twice > d || (twice === d && q % 2n !== 0n) ? q + step : q;
}

/** The latest rate for the pair on or before `day`, its own or the reverse inverted. */
function pickRate(from: string, to: string, day: string) {
  const latest = (a: string, b: string) =>
    state.fxRates
      .filter((r) => r.from === a && r.to === b && r.effective_on <= day)
      .sort((x, y) => y.effective_on.localeCompare(x.effective_on))[0];
  const own = latest(from, to);
  const reverse = latest(to, from);
  if (own && !(reverse && reverse.effective_on > own.effective_on))
    return { rate: own, inverted: false };
  return reverse ? { rate: reverse, inverted: true } : null;
}

/** Convert a deal report's sums as the service does: once per group and currency, then merge. */
function convertReport(spec: ReportSpec, rows: ReportResult["rows"]) {
  if (spec.measure !== "sum_amount" && spec.convert_to)
    return "only a sum of amounts can be converted";
  return convertSums(spec.convert_to!, spec.rate_date ?? null, rows);
}

/** Convert `(group, currency, value)` sums into `target`: once per group and currency, then merge. */
function convertSums(
  target: string,
  rateDate: string | null,
  rows: ReportResult["rows"],
):
  | string
  | {
      rows: ReportResult["rows"];
      rateDate: string;
      rates: ReportRateUsed[];
      unconverted: ReportUnconverted[];
    } {
  if (!/^[A-Z]{3}$/.test(target))
    return "convert_to is three capital letters, such as GBP";
  const today = new Date().toISOString().slice(0, 10);
  const day = rateDate ?? today;
  if (day > today) return "rate_date is in the future";
  const merged = new Map<string, number>();
  const used = new Map<string, ReturnType<typeof pickRate>>();
  const left = new Map<string, { groups: number; amount_minor: number }>();
  for (const row of rows) {
    const currency = row.currency ?? target;
    let value = row.value;
    if (currency !== target) {
      if (!used.has(currency))
        used.set(currency, pickRate(currency, target, day));
      const picked = used.get(currency);
      if (!picked) {
        const entry = left.get(currency) ?? { groups: 0, amount_minor: 0 };
        entry.groups += 1;
        entry.amount_minor += row.value;
        left.set(currency, entry);
        continue;
      }
      const ppb = ppbOf(picked.rate.rate)!;
      const amount = BigInt(row.value);
      value = Number(
        picked.inverted
          ? divideRound(amount * ONE, ppb)
          : divideRound(amount * ppb, ONE),
      );
    }
    merged.set(row.group, (merged.get(row.group) ?? 0) + value);
  }
  return {
    rows: [...merged.entries()]
      .map(([group, value]) => ({ group, currency: target, value }))
      .sort((a, b) => b.value - a.value || a.group.localeCompare(b.group)),
    rateDate: day,
    rates: [...used.entries()]
      .filter(([, picked]) => picked)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([from, picked]) => ({
        from,
        to: target,
        rate: picked!.rate.rate,
        effective_on: picked!.rate.effective_on,
        source: picked!.rate.source,
        inverted: picked!.inverted,
      })),
    unconverted: [...left.entries()]
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([currency, entry]) => ({ currency, ...entry })),
  };
}

function fxRate(
  pid: string,
  from: string,
  to: string,
  rate: string,
  daysAgo: number,
): FxRate {
  return {
    pid,
    from,
    to,
    rate,
    effective_on: day(daysAgo),
    source: "Tour sample rate",
    set_by: "worker:demo",
  };
}

/** The tour's exchange rates (CRM-R50): list, set (replacing a pair and day), delete. */
function fxRates(method: string, id: string | undefined, body: unknown) {
  if (method === "GET" && !id)
    return ok(
      [...state.fxRates].sort((a, b) =>
        b.effective_on.localeCompare(a.effective_on),
      ),
    );
  if (method === "PUT" && !id) {
    const input = bodyOf(body) as Partial<Record<string, string>>;
    const from = input.from ?? "";
    const to = input.to ?? "";
    if (!/^[A-Z]{3}$/.test(from))
      return problem(422, "from is three capital letters");
    if (!/^[A-Z]{3}$/.test(to) || to === from)
      return problem(422, "to is a different currency, three capital letters");
    if (!ppbOf(input.rate ?? ""))
      return problem(
        422,
        "a rate is a positive decimal below 1000000 with at most 9 decimal places",
      );
    if (!input.effective_on) return problem(422, "effective_on is a day");
    const source = (input.source ?? "").trim();
    if (!source || source.length > 80)
      return problem(
        422,
        "a rate needs a source in words, up to 80 characters",
      );
    // Trailing zeros are not kept, as the service writes a rate.
    const entered = input.rate!;
    const rate = entered.includes(".")
      ? entered.replace(/0+$/, "").replace(/\.$/, "")
      : entered;
    const at = state.fxRates.findIndex(
      (r) =>
        r.from === from && r.to === to && r.effective_on === input.effective_on,
    );
    const made: FxRate = {
      pid:
        at >= 0
          ? state.fxRates[at]!.pid
          : `fx-${state.fxRates.length + 1}-${Date.now()}`,
      from,
      to,
      rate,
      effective_on: input.effective_on,
      source,
      set_by: "worker:demo",
    };
    if (at >= 0) state.fxRates[at] = made;
    else state.fxRates.push(made);
    return ok(made);
  }
  if (method === "DELETE" && id) {
    const at = state.fxRates.findIndex((r) => r.pid === id);
    if (at < 0) return problem(404, "no such rate");
    state.fxRates.splice(at, 1);
    return ok({ pid: id });
  }
  return problem(405, "method not allowed");
}

/**
 * The campaigns that touched each won deal's buyer in the tour, as days before the deal closed.
 * (The service reads these from the e-mails it sent and the leads campaigns brought in.)
 */
type Touched = {
  campaign: string;
  daysBefore: number;
  kind: "sent" | "opened" | "clicked";
};
const TOUCHED: Record<string, Touched[]> = {
  "deal-9": [
    { campaign: "camp-1", daysBefore: 40, kind: "sent" },
    { campaign: "camp-2", daysBefore: 9, kind: "sent" },
    // What the mail provider reported (CRM-R58): the newer campaign's e-mail was opened.
    { campaign: "camp-2", daysBefore: 8, kind: "opened" },
  ],
  "deal-10": [
    { campaign: "camp-1", daysBefore: 20, kind: "sent" },
    { campaign: "camp-2", daysBefore: 5, kind: "clicked" },
  ],
};

/** Share `amount` among `weights` in proportion, exactly: floors, then the leftover units to the
 * largest remainders, the earliest first on a tie (as the service does, in integers). */
function apportion(amount: number, weights: bigint[]): number[] {
  const total = weights.reduce((a, b) => a + b, 0n);
  if (total <= 0n)
    return apportion(
      amount,
      weights.map(() => 1n),
    );
  const sum = BigInt(amount);
  const shares = weights.map((w) => (sum * w) / total);
  let left = sum - shares.reduce((a, b) => a + b, 0n);
  const order = weights
    .map((w, i) => ({ i, rest: (sum * w) % total }))
    .sort((a, b) => (a.rest === b.rest ? a.i - b.i : a.rest < b.rest ? 1 : -1));
  for (const { i } of order) {
    if (left === 0n) break;
    shares[i]! += 1n;
    left -= 1n;
  }
  return shares.map(Number);
}

/** A touch's weight under time decay: halves every seven days, straight between (integers). */
function decayWeight(daysBefore: number): bigint {
  const half = 7n * 86_400n;
  const age = BigInt(Math.max(0, daysBefore)) * 86_400n;
  const whole = age / half;
  const part = age % half;
  const start = (1n << 100n) >> (whole > 100n ? 100n : whole);
  return (start * (2n * half - part)) / (2n * half);
}

/** Share `amount` among touches in time order, as the service does (CRM-R49, CRM-R56). */
function shareAmount(
  model: string,
  amount: number,
  touches: { campaign: string; daysBefore: number }[],
): Map<string, number> {
  const ordered = [...touches].sort(
    (a, b) =>
      b.daysBefore - a.daysBefore || a.campaign.localeCompare(b.campaign),
  );
  const n = ordered.length;
  const middle = BigInt(Math.max(n - 2, 1));
  const weights = ordered.map((t, i): bigint => {
    switch (model) {
      case "first":
        return i === 0 ? 1n : 0n;
      case "last":
        return i === n - 1 ? 1n : 0n;
      case "time_decay":
        return decayWeight(t.daysBefore);
      case "position":
        return n <= 2 ? 1n : i === 0 || i === n - 1 ? 40n * middle : 20n;
      default:
        return 1n;
    }
  });
  const out = new Map<string, number>();
  apportion(amount, weights).forEach((share, i) => {
    if (weights[i]! > 0n) {
      const campaign = ordered[i]!.campaign;
      out.set(campaign, (out.get(campaign) ?? 0) + share);
    }
  });
  return out;
}

/** The tour's commission plans (CRM-R59): list, set by name, delete one no quota uses. */
function commissionPlans(
  method: string,
  id: string | undefined,
  body: unknown,
) {
  if (method === "GET" && !id) {
    for (const p of state.plans)
      p.quotas = state.quotas.filter((q) => q.plan_pid === p.pid).length;
    return ok(state.plans);
  }
  if (method === "PUT" && !id) {
    const input = (body ?? {}) as { name?: string; tiers?: CommissionTier[] };
    const name = (input.name ?? "").trim().replace(/\s+/g, " ");
    const tiers = input.tiers ?? [];
    if (!name) return problem(422, "a plan needs a name");
    if (
      tiers.length < 1 ||
      tiers.length > 8 ||
      tiers[0]!.from_percent !== 0 ||
      tiers.some(
        (t, i) => i > 0 && t.from_percent <= tiers[i - 1]!.from_percent,
      ) ||
      tiers.some((t) => t.rate_bps < 0 || t.rate_bps > 10_000)
    )
      return problem(
        422,
        "tiers are 1 to 8, the first at 0, each above the one before",
      );
    const found = state.plans.find(
      (p) => p.name.toLowerCase() === name.toLowerCase(),
    );
    if (found) {
      found.name = name;
      found.tiers = tiers;
      return ok(found);
    }
    const made: CommissionPlan = {
      pid: `plan-${state.plans.length + 1}`,
      name,
      tiers,
      quotas: 0,
      set_by: "worker:demo",
    };
    state.plans.push(made);
    return ok(made);
  }
  if (method === "DELETE" && id) {
    const at = state.plans.findIndex((p) => p.pid === id);
    if (at < 0) return problem(404, "no such plan");
    if (state.quotas.some((q) => q.plan_pid === id))
      return problem(409, "a quota is still paid under this plan");
    state.plans.splice(at, 1);
    return ok({ pid: id });
  }
  return problem(405, "method not allowed");
}

/** The bands of a plan over a quota and the revenue reached, in integers (as the service does). */
function commissionBands(
  quota: number,
  revenue: number,
  tiers: CommissionTier[],
): CommissionBand[] {
  const start = (percent: number) => (BigInt(quota) * BigInt(percent)) / 100n;
  return tiers.map((tier, i) => {
    const next = tiers[i + 1];
    const low = start(tier.from_percent);
    const rev = BigInt(Math.max(revenue, 0));
    let inBand =
      (next
        ? rev < start(next.from_percent)
          ? rev
          : start(next.from_percent)
        : rev) - low;
    if (inBand < 0n) inBand = 0n;
    return {
      from_percent: tier.from_percent,
      to_percent: next ? next.from_percent : null,
      revenue_minor: Number(inBand),
      rate_bps: tier.rate_bps,
      commission_minor: Number(
        divideRound(inBand * BigInt(tier.rate_bps), 10_000n),
      ),
    };
  });
}

/** The tour's CLV for an account (CRM-R13, CRM-R60): won deals per currency, margin, and the look-ahead. */
function accountClv(accountPid: string, query: URLSearchParams) {
  const account = state.accounts.find((a) => a.pid === accountPid);
  if (!account) return problem(404, "account not found");
  const people = state.contacts.filter((c) => c.account_pid === accountPid);
  const dealPids = new Set(
    people.flatMap((c) => state.links[c.pid]?.deals ?? []),
  );
  const won = state.deals.filter((d) => dealPids.has(d.pid) && d.won);
  const asOf = new Date();
  const yearAgo = asOf.getTime() - 365 * 86_400_000;
  const total: Record<string, bigint> = {};
  const recent: Record<string, bigint> = {};
  const prior: Record<string, bigint> = {};
  for (const d of won) {
    const amount = BigInt(d.amount_minor ?? 0);
    total[d.currency] = (total[d.currency] ?? 0n) + amount;
    const at = d.closed_at ? new Date(d.closed_at).getTime() : null;
    if (at !== null && at >= yearAgo)
      recent[d.currency] = (recent[d.currency] ?? 0n) + amount;
    else if (at !== null && at >= yearAgo - 365 * 86_400_000)
      prior[d.currency] = (prior[d.currency] ?? 0n) + amount;
  }
  const bps = (name: string) => {
    const raw = query.get(name);
    if (raw === null) return null;
    const n = Number(raw);
    return Number.isInteger(n) && n >= 0 && n <= 10_000 ? n : NaN;
  };
  const marginBps = bps("margin_bps");
  const retentionBps = bps("retention_bps");
  const fromHistory = query.get("retention_from") === "history";
  const discountBps = bps("discount_bps") ?? 0;
  const years = query.get("years") === null ? null : Number(query.get("years"));
  if (Number.isNaN(marginBps)) return problem(422, "/margin_bps: 0 to 10000");
  if (Number.isNaN(retentionBps))
    return problem(422, "/retention_bps: 0 to 10000");
  if (Number.isNaN(discountBps))
    return problem(422, "/discount_bps: 0 to 10000");
  if ((retentionBps === null && !fromHistory) === (years !== null))
    return problem(422, "/years: retention_bps and years go together");
  if (years !== null && !(Number.isInteger(years) && years >= 1 && years <= 20))
    return problem(422, "/years: 1 to 20");
  const keep = (m: Record<string, bigint>) =>
    marginBps === null
      ? m
      : Object.fromEntries(
          Object.entries(m).map(([c, v]) => [
            c,
            divideRound(v * BigInt(marginBps), 10_000n),
          ]),
        );
  const plain = (m: Record<string, bigint>) =>
    Object.fromEntries(Object.entries(m).map(([c, v]) => [c, Number(v)]));
  let projection = null;
  if ((retentionBps !== null || fromHistory) && years !== null) {
    const history: Record<string, number | null> = {};
    const base = keep(recent);
    const baseWon = keep(total);
    const projected: Record<string, bigint> = {};
    for (const [c, b] of Object.entries(base)) {
      let retention = retentionBps;
      if (fromHistory) {
        const before = prior[c] ?? 0n;
        retention =
          before > 0n
            ? Number(
                divideRound((recent[c] ?? 0n) * 10_000n, before) > 10_000n
                  ? 10_000n
                  : divideRound((recent[c] ?? 0n) * 10_000n, before),
              )
            : null;
        history[c] = retention;
      }
      if (retention === null) {
        projected[c] = 0n;
        continue;
      }
      let year = b;
      let sum = 0n;
      for (let i = 0; i < years; i++) {
        year = divideRound(
          year * BigInt(retention),
          BigInt(10_000 + discountBps),
        );
        // Year t is year t-1 x retention / (1 + discount).
        sum += year;
      }
      projected[c] = sum;
    }
    const totals: Record<string, bigint> = { ...baseWon };
    for (const [c, v] of Object.entries(projected))
      totals[c] = (totals[c] ?? 0n) + v;
    projection = {
      retention_bps: retentionBps,
      retention_history_bps: fromHistory ? history : null,
      discount_bps: discountBps,
      years,
      annual_base_minor: plain(base),
      projected_minor: plain(projected),
      total_minor: plain(totals),
    };
  }
  const marginMinor = marginBps === null ? null : plain(keep(total));
  // `convert_to` converts the CLV, the margin and the look-ahead at the same rates (CRM-R62).
  const target = query.get("convert_to");
  let conversion: Record<string, unknown> = {
    convert_to: null,
    converted_minor: null,
    rate_date: null,
    rates: [],
    unconverted: [],
    margin_converted_minor: null,
  };
  if (target) {
    const groups: [string, Record<string, number> | null][] = [
      ["clv", plain(total)],
      ["margin", marginMinor],
      ["base", projection?.annual_base_minor ?? null],
      ["projected", projection?.projected_minor ?? null],
      ["total", projection?.total_minor ?? null],
    ];
    const rows = groups.flatMap(([group, amounts]) =>
      Object.entries(amounts ?? {}).map(([currency, value]) => ({
        group,
        currency,
        value,
      })),
    );
    const done = convertSums(target, query.get("rate_date"), rows);
    if (typeof done === "string") return problem(422, done);
    const of = (group: string) =>
      done.rows.find((r) => r.group === group)?.value ?? 0;
    // Only the CLV's own sums say what was left out, as the service does.
    const own = convertSums(
      target,
      query.get("rate_date"),
      rows.filter((r) => r.group === "clv"),
    );
    if (typeof own === "string") return problem(422, own);
    if (projection)
      projection = {
        ...projection,
        converted: {
          annual_base_minor: of("base"),
          projected_minor: of("projected"),
          total_minor: of("total"),
        },
      };
    conversion = {
      convert_to: target,
      converted_minor: of("clv"),
      rate_date: own.rateDate,
      rates: own.rates,
      unconverted: own.unconverted,
      margin_converted_minor: marginMinor ? of("margin") : null,
    };
  }
  return ok({
    account_pid: accountPid,
    as_of: asOf.toISOString(),
    won_deals: won.length,
    clv_minor: plain(total),
    margin_minor: marginMinor,
    projection,
    ...conversion,
  });
}

const STOP_WORDS = new Set(
  "the and for are but not you can with has have this that from was our its when what how why does did into".split(
    " ",
  ),
);

/** The words of a text: lower case, letters and digits, three or more characters, not common. */
function wordsOf(text: string): Set<string> {
  return new Set(
    text
      .toLowerCase()
      .split(/[^\p{L}\p{N}]+/u)
      .filter((w) => [...w].length >= 3 && !STOP_WORDS.has(w)),
  );
}

/** The published articles sharing words with a ticket's title, as the service ranks them (CRM-R66). */
function suggestArticles(title: string) {
  const wanted = wordsOf(title);
  return state.articles
    .filter((a) => a.status === "published")
    .map((a, index) => {
      const headline = wordsOf(a.title);
      const text = wordsOf(a.body);
      const matched = [...wanted].filter((w) => headline.has(w) || text.has(w));
      const score = matched.reduce((s, w) => s + (headline.has(w) ? 2 : 1), 0);
      return { index, article_pid: a.pid, title: a.title, score, matched };
    })
    .filter((m) => m.score > 0)
    .sort((a, b) => b.score - a.score || a.index - b.index)
    .slice(0, 5)
    .map((m) => ({
      article_pid: m.article_pid,
      title: m.title,
      score: m.score,
      matched: m.matched,
    }));
}

/** Share an amount by basis points to the minor unit, leftovers to the largest remainders (CRM-R65). */
function splitAmount(amount: number, shares: number[]): number[] {
  const exact = shares.map((s) => BigInt(amount) * BigInt(s));
  const parts = exact.map((e) => e / 10_000n);
  let left = BigInt(amount) - parts.reduce((a, b) => a + b, 0n);
  const order = shares
    .map((_, i) => i)
    .sort((a, b) => {
      const ra = exact[a]! % 10_000n;
      const rb = exact[b]! % 10_000n;
      return ra === rb ? a - b : ra < rb ? 1 : -1;
    });
  for (const i of order) {
    if (left <= 0n) break;
    parts[i]! += 1n;
    left -= 1n;
  }
  return parts.map(Number);
}

/** The tour's deal splits (CRM-R65). */
function dealSplits(
  method: string,
  dealPid: string | undefined,
  body: unknown,
) {
  const deal = state.deals.find((d) => d.pid === dealPid);
  if (!deal) return problem(404, "deal not found");
  if (method === "GET")
    return ok({ deal_pid: deal.pid, shares: state.splits[deal.pid] ?? [] });
  if (method === "DELETE") {
    if (!state.splits[deal.pid]) return problem(404, "that deal is not split");
    delete state.splits[deal.pid];
    return ok({ pid: deal.pid });
  }
  const shares = (
    (body ?? {}) as { shares?: { owner_ref: string; share_bps: number }[] }
  ).shares;
  if (
    !shares ||
    shares.length < 1 ||
    shares.length > 8 ||
    new Set(shares.map((s) => s.owner_ref)).size !== shares.length ||
    shares.some((s) => !(s.share_bps >= 1 && s.share_bps <= 10_000)) ||
    shares.reduce((a, s) => a + s.share_bps, 0) !== 10_000
  )
    return problem(
      422,
      "/shares: from 1 to 8 workers named once, adding up to 10000",
    );
  state.splits[deal.pid] = shares;
  return ok({ deal_pid: deal.pid, shares });
}

/** The quotas whose period contains `at`, with what each worker's won deals credit them. */
function attainmentRows(at: string, owner: string | null): QuotaAttainment[] {
  return state.quotas
    .filter((q) => q.period_from <= at && q.period_to >= at)
    .filter((q) => !owner || q.owner_ref === owner)
    .map((q) => {
      // What each won deal credits this worker: all of it, or their share of a split (CRM-R65).
      const won = state.deals
        .filter(
          (d) =>
            d.won &&
            d.closed_at &&
            d.closed_at.slice(0, 10) >= q.period_from &&
            d.closed_at.slice(0, 10) <= q.period_to,
        )
        .flatMap((d) => {
          const shares = state.splits[d.pid];
          if (!shares) {
            return d.owner_ref === q.owner_ref
              ? [{ currency: d.currency, credited: d.amount_minor ?? 0 }]
              : [];
          }
          const at = shares.findIndex((s) => s.owner_ref === q.owner_ref);
          if (at < 0) return [];
          const parts = splitAmount(
            d.amount_minor ?? 0,
            shares.map((s) => s.share_bps),
          );
          return [{ currency: d.currency, credited: parts[at]! }];
        });
      const mine = won.filter((d) => d.currency === q.currency);
      const revenue = mine.reduce((s, d) => s + d.credited, 0);
      const other: Record<string, number> = {};
      for (const d of won.filter((d) => d.currency !== q.currency))
        other[d.currency] = (other[d.currency] ?? 0) + d.credited;
      const plan = state.plans.find((p) => p.pid === q.plan_pid) ?? null;
      const bands = plan
        ? commissionBands(q.amount_minor, revenue, plan.tiers)
        : [];
      return {
        quota_pid: q.pid,
        owner_ref: q.owner_ref,
        period_from: q.period_from,
        period_to: q.period_to,
        currency: q.currency,
        quota_minor: q.amount_minor,
        revenue_minor: revenue,
        won_deals: mine.length,
        attainment: ratio(revenue, q.amount_minor),
        remaining_minor: Math.max(q.amount_minor - revenue, 0),
        other_currencies: other,
        plan: plan?.name ?? null,
        commission_minor: plan
          ? bands.reduce((s, b) => s + b.commission_minor, 0)
          : null,
        bands,
      };
    });
}

/** The tour's teams (CRM-R67) and their roll-up, added per currency and never converted. */
function teamRoutes(
  method: string,
  seg: string[],
  query: URLSearchParams,
  body: unknown,
) {
  if (seg[1] === "team-attainment") {
    const at = query.get("at") ?? new Date().toISOString().slice(0, 10);
    const standing = attainmentRows(at, null);
    const rows = [...state.teams]
      .sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()))
      .map((team) => {
        const mine = standing.filter((s) => team.members.includes(s.owner_ref));
        const per = new Map<
          string,
          { quota: number; revenue: number; won: number; workers: number }
        >();
        for (const s of mine) {
          const slot = per.get(s.currency) ?? {
            quota: 0,
            revenue: 0,
            won: 0,
            workers: 0,
          };
          slot.quota += s.quota_minor;
          slot.revenue += s.revenue_minor;
          slot.won += s.won_deals;
          slot.workers += 1;
          per.set(s.currency, slot);
        }
        const other: Record<string, number> = {};
        for (const s of mine)
          for (const [code, minor] of Object.entries(s.other_currencies))
            if (!per.has(code)) other[code] = (other[code] ?? 0) + minor;
        return {
          team_pid: team.pid,
          name: team.name,
          members: team.members,
          without_quota: team.members.filter(
            (m) => !mine.some((s) => s.owner_ref === m),
          ),
          currencies: [...per.entries()].map(([currency, v]) => ({
            currency,
            quota_minor: v.quota,
            revenue_minor: v.revenue,
            won_deals: v.won,
            attainment: ratio(v.revenue, v.quota),
            remaining_minor: Math.max(v.quota - v.revenue, 0),
            workers: v.workers,
          })),
          other_currencies: other,
        };
      });
    return ok({
      as_of: new Date().toISOString(),
      at,
      rows,
      note: "currencies are listed, never added or converted",
    });
  }
  if (method === "GET") return ok(state.teams);
  if (method === "DELETE") {
    const at = state.teams.findIndex((t) => t.pid === seg[2]);
    if (at < 0) return problem(404, "team not found");
    state.teams.splice(at, 1);
    return ok({ pid: seg[2] });
  }
  const input = (body ?? {}) as { name?: string; members?: string[] };
  const name = (input.name ?? "").trim();
  const members = input.members ?? [];
  if (!name || name.length > 80)
    return problem(422, "/name: 1 to 80 characters");
  if (
    members.length < 1 ||
    members.length > 50 ||
    new Set(members).size !== members.length
  )
    return problem(422, "/members: from 1 to 50 workers named once");
  const had = state.teams.find(
    (t) => t.name.toLowerCase() === name.toLowerCase(),
  );
  if (had) {
    had.name = name;
    had.members = members;
    return ok(had);
  }
  const team = {
    pid: `team-${state.teams.length + 1}-${name.length}`,
    name,
    members,
    set_by: "worker:demo",
  };
  state.teams.push(team);
  return ok(team);
}

/** The tour's quotas (CRM-R59) and their attainment, computed from the tour's won deals. */
function quotaRoutes(
  method: string,
  seg: string[],
  query: URLSearchParams,
  body: unknown,
) {
  const [, id] = seg;
  if (id === "splits") return dealSplits(method, seg[2], body);
  if (id === "teams" || id === "team-attainment")
    return teamRoutes(method, seg, query, body);
  if (method === "GET" && id === "attainment") {
    const at = query.get("at") ?? new Date().toISOString().slice(0, 10);
    const rows = attainmentRows(at, query.get("owner_ref"));
    return ok({
      as_of: new Date().toISOString(),
      at,
      rows,
      note: "a statement of what the plan pays, not a payment",
    });
  }
  if (method === "GET" && !id) return ok(state.quotas);
  if (method === "PUT" && !id) {
    const input = (body ?? {}) as Partial<Quota>;
    if (
      !input.owner_ref ||
      !input.currency ||
      !(input.amount_minor && input.amount_minor > 0)
    )
      return problem(
        422,
        "owner_ref, currency and an amount above zero are needed",
      );
    if (
      !input.period_from ||
      !input.period_to ||
      input.period_to < input.period_from
    )
      return problem(422, "the period ends before it starts");
    if (input.plan_pid && !state.plans.some((p) => p.pid === input.plan_pid))
      return problem(422, "there is no such plan");
    const overlap = state.quotas.some(
      (q) =>
        q.owner_ref === input.owner_ref &&
        q.currency === input.currency &&
        q.period_from <= input.period_to! &&
        q.period_to >= input.period_from! &&
        !(
          q.period_from === input.period_from && q.period_to === input.period_to
        ),
    );
    if (overlap)
      return problem(
        422,
        "overlaps another quota of this worker in this currency",
      );
    const same = state.quotas.find(
      (q) =>
        q.owner_ref === input.owner_ref &&
        q.currency === input.currency &&
        q.period_from === input.period_from &&
        q.period_to === input.period_to,
    );
    if (same) {
      same.amount_minor = input.amount_minor;
      same.plan_pid = input.plan_pid ?? null;
      return ok(same);
    }
    const made: Quota = {
      pid: `quota-${state.quotas.length + 1}-${Date.now() % 100000}`,
      owner_ref: input.owner_ref,
      period_from: input.period_from,
      period_to: input.period_to,
      currency: input.currency,
      amount_minor: input.amount_minor,
      plan_pid: input.plan_pid ?? null,
      set_by: "worker:demo",
    };
    state.quotas.push(made);
    return ok(made);
  }
  if (method === "DELETE" && id) {
    const at = state.quotas.findIndex((q) => q.pid === id);
    if (at < 0) return problem(404, "no such quota");
    state.quotas.splice(at, 1);
    return ok({ pid: id });
  }
  return problem(405, "method not allowed");
}

/** The published articles the public help centre may show. */
const publishedArticles = () =>
  state.articles.filter((a) => a.status === "published");

/** The public help centre as the service answers it (CRM-R57), counting what it is asked. */
function kbPublic(
  method: string,
  seg: string[],
  query: URLSearchParams,
  body: unknown,
): DemoReply {
  const [, what, pid, tail] = seg;
  if (what !== "articles") return problem(404, "not found");
  if (!pid && method === "GET") {
    const q = (query.get("q") ?? "").trim().toLowerCase();
    const found = publishedArticles()
      .filter((a) => !q || `${a.title} ${a.body}`.toLowerCase().includes(q))
      .sort((a, b) => a.title.localeCompare(b.title));
    if (q) {
      state.kb.searches += 1;
      if (found.length === 0) state.kb.empty += 1;
    }
    return ok(
      found.map((a) => ({
        pid: a.pid,
        title: a.title,
        snippet:
          a.body.length > 200 ? `${a.body.slice(0, 200).trimEnd()}…` : a.body,
        keywords: null,
        version: a.version,
        updated_at: new Date().toISOString(),
      })),
    );
  }
  const article = publishedArticles().find((a) => a.pid === pid);
  if (!article) return problem(404, "not found");
  if (!tail && method === "GET") {
    state.kb.views[pid!] = (state.kb.views[pid!] ?? 0) + 1;
    return ok({
      pid: article.pid,
      title: article.title,
      body: article.body,
      keywords: null,
      version: article.version,
      updated_at: new Date().toISOString(),
    });
  }
  if (tail === "feedback" && method === "POST") {
    const helpful = (body as { helpful?: unknown } | null)?.helpful;
    if (typeof helpful !== "boolean")
      return problem(422, "helpful is true or false");
    const counts = helpful ? state.kb.yes : state.kb.no;
    counts[pid!] = (counts[pid!] ?? 0) + 1;
    return { status: 202, body: { status: "received" } } as DemoReply;
  }
  return problem(404, "not found");
}

/** The tour's deflection report, from what the help centre has counted and the tour's tickets. */
function kbDeflectionAnswer(days: number) {
  const sum = (m: Record<string, number>) =>
    Object.values(m).reduce((a, b) => a + b, 0);
  const yes = sum(state.kb.yes);
  const tickets = state.tickets.length;
  const to = new Date();
  const from = new Date(to.getTime() - (days - 1) * 86_400_000);
  const day = (d: Date) => d.toISOString().slice(0, 10);
  return {
    from: day(from),
    to: day(to),
    days,
    views: sum(state.kb.views),
    searches: state.kb.searches,
    searches_without_result: state.kb.empty,
    helpful_yes: yes,
    helpful_no: sum(state.kb.no),
    tickets_opened: tickets,
    deflection: ratio(yes, yes + tickets),
    top_articles: Object.entries(state.kb.views)
      .map(([pid, views]) => ({
        pid,
        title:
          state.articles.find((a) => a.pid === pid)?.title ??
          "(deleted article)",
        views,
        helpful_yes: state.kb.yes[pid] ?? 0,
        helpful_no: state.kb.no[pid] ?? 0,
      }))
      .sort((a, b) => b.views - a.views || a.title.localeCompare(b.title))
      .slice(0, 10),
    note: "counts only, kept per day; a visitor is not identified, so a vote is a visitor's own word and a ticket is not tied to a visit; deflection is an estimate, not a measure",
  } satisfies KbDeflection;
}

/** The tour's attribution (CRM-R49), computed over its won deals as the service does over its own. */
function attributionAnswer(query: URLSearchParams): DemoReply {
  const model = query.get("model") ?? "linear";
  if (!["first", "last", "linear", "time_decay", "position"].includes(model))
    return problem(
      422,
      "the model is one of first, last, linear, time_decay, position",
    );
  const windowDays = Number(query.get("window_days") ?? 90);
  if (!Number.isInteger(windowDays) || windowDays < 1 || windowDays > 365)
    return problem(422, "the window is 1 to 365 days");
  const from = query.get("from");
  const to = query.get("to");
  const kinds = (query.get("touches") ?? "sent")
    .split(",")
    .map((k) => k.trim());
  if (kinds.some((k) => !["sent", "opened", "clicked"].includes(k)))
    return problem(422, "touches is a list of sent, opened, clicked");
  const convertTo = query.get("convert_to");
  const rateDate = query.get("rate_date");
  if (!convertTo && rateDate) return problem(422, "rate_date needs convert_to");
  const won = state.deals.filter((d) => {
    const day = d.closed_at?.slice(0, 10);
    return d.won && day && (!from || day >= from) && (!to || day <= to);
  });
  const tally = new Map<
    string,
    { touches: number; deals: number; credit: Map<string, number> }
  >();
  const unattributed = new Map<string, number>();
  let unattributedDeals = 0;
  for (const deal of won) {
    const amount = Number(deal.amount_minor ?? 0);
    const touches = (TOUCHED[deal.pid] ?? []).filter(
      (t) => t.daysBefore <= windowDays && kinds.includes(t.kind),
    );
    if (touches.length === 0) {
      unattributedDeals += 1;
      unattributed.set(
        deal.currency,
        (unattributed.get(deal.currency) ?? 0) + amount,
      );
      continue;
    }
    for (const t of touches) {
      const row = tally.get(t.campaign) ?? {
        touches: 0,
        deals: 0,
        credit: new Map(),
      };
      row.touches += 1;
      tally.set(t.campaign, row);
    }
    for (const campaign of new Set(touches.map((t) => t.campaign)))
      tally.get(campaign)!.deals += 1;
    for (const [campaign, share] of shareAmount(model, amount, touches)) {
      const credit = tally.get(campaign)!.credit;
      credit.set(deal.currency, (credit.get(deal.currency) ?? 0) + share);
    }
  }
  const amounts = (m: Map<string, number>): AttributionAmount[] =>
    [...m.entries()].map(([currency, amount_minor]) => ({
      currency,
      amount_minor,
    }));
  // With `convert_to`, each campaign's credit in each currency is converted once and merged.
  let converted: Exclude<ReturnType<typeof convertSums>, string> | null = null;
  if (convertTo) {
    const sums: ReportResult["rows"] = [];
    for (const [pid, row] of tally)
      for (const [currency, value] of row.credit)
        sums.push({ group: pid, currency, value });
    for (const [currency, value] of unattributed)
      sums.push({ group: "unattributed", currency, value });
    const done = convertSums(convertTo, rateDate, sums);
    if (typeof done === "string") return problem(422, done);
    converted = done;
  }
  const inTarget = (group: string): AttributionAmount[] => {
    const row = converted?.rows.find((r) => r.group === group);
    return row ? [{ currency: convertTo!, amount_minor: row.value }] : [];
  };
  return ok({
    model,
    window_days: windowDays,
    touches: ["sent", "opened", "clicked"].filter((k) => kinds.includes(k)),
    as_of: new Date().toISOString(),
    deals: won.length,
    unattributed_deals: unattributedDeals,
    unattributed: converted ? inTarget("unattributed") : amounts(unattributed),
    convert_to: convertTo,
    rate_date: converted?.rateDate ?? null,
    rates: converted?.rates ?? [],
    unconverted: converted?.unconverted ?? [],
    campaigns: [...tally.entries()]
      .map(([pid, row]) => ({
        campaign_pid: pid,
        name:
          state.campaigns.find((c) => c.pid === pid)?.name ??
          "(deleted campaign)",
        touches: row.touches,
        deals: row.deals,
        credit: converted ? inTarget(pid) : amounts(row.credit),
      }))
      .sort(
        (a, b) =>
          b.deals - a.deals ||
          b.touches - a.touches ||
          a.name.localeCompare(b.name),
      ),
    truncated: false,
    note: "each won deal's amount is shared among the campaigns that touched its buyer in the window before it closed; shares add up to the amount; per currency, never added across currencies",
  } satisfies Attribution);
}

function reports(
  method: string,
  id: string | undefined,
  tail: string | undefined,
  body: unknown,
) {
  if (method === "POST" && id === "run")
    return reportResult(bodyOf(body) as unknown as ReportSpec);
  if (method === "GET" && !id) return ok(state.reports);
  if (method === "POST" && !id) {
    const input = bodyOf(body) as {
      name?: string;
      spec?: ReportSpec;
      shared?: boolean;
    };
    if (!input.name?.trim()) return problem(422, "name is required");
    if (
      state.reports.some(
        (r) => r.name.toLowerCase() === input.name!.trim().toLowerCase(),
      )
    )
      return problem(422, "you already have a report with that name");
    const check = reportResult(input.spec as ReportSpec);
    if (check.status !== 200) return check;
    const made: Report = {
      pid: `rep-${state.reports.length + 1}-${Date.now()}`,
      owner_ref: "worker:demo",
      mine: true,
      name: input.name.trim(),
      spec: input.spec as ReportSpec,
      shared: input.shared ?? false,
      updated_at: new Date().toISOString(),
    };
    state.reports.push(made);
    return ok({ pid: made.pid });
  }
  const at = state.reports.findIndex((r) => r.pid === id);
  if (at < 0) return problem(404, "report not found");
  if (method === "GET" && tail === "run")
    return reportResult(state.reports[at]!.spec);
  if (method === "DELETE") {
    state.reports.splice(at, 1);
    return ok({ pid: id });
  }
  return problem(405, "method not allowed");
}

/** The tags that exist, with how many records carry each. */
function demoTags(): Tag[] {
  const names = new Set(Object.values(state.tagged).flat());
  return [...names].sort().map((name, i) => ({
    pid: `tag-${i + 1}`,
    name,
    uses: Object.values(state.tagged).filter((l) => l.includes(name)).length,
  }));
}

/** The tour's tags on one record: read, or set whole. */
function recordTags(pid: string, method: string, body: unknown) {
  if (method === "GET") return ok(state.tagged[pid] ?? []);
  if (method === "PUT") {
    const tags = ((bodyOf(body).tags ?? []) as unknown[]).map((t) =>
      String(t).trim().replace(/\s+/g, " "),
    );
    if (tags.some((n) => !n || n.length > 40))
      return problem(422, "a tag name is 1 to 40 characters");
    if (tags.length > 20) return problem(422, "a record has at most 20 tags");
    state.tagged[pid] = [...new Set(tags)];
    return ok(state.tagged[pid]);
  }
  return problem(405, "method not allowed");
}

/** A list narrowed by `?tag=` (any case), as the service does. */
function taggedOnly<T extends { pid: string }>(
  rows: T[],
  query: URLSearchParams,
): T[] {
  const tag = query.get("tag")?.trim().toLowerCase();
  if (!tag) return rows;
  return rows.filter((r) =>
    (state.tagged[r.pid] ?? []).some((n) => n.toLowerCase() === tag),
  );
}

/** The demo's attached files: listed and added by name and size; the bytes are not kept. */
function attachmentList(pid: string, method: string, query: URLSearchParams) {
  const held = (state.attachments[pid] ??= []);
  if (method === "GET") return ok(held);
  if (method === "POST") {
    const name = query.get("name")?.trim();
    if (!name) return problem(422, "the file needs a name");
    if (
      !/\.(pdf|png|jpe?g|gif|webp|txt|csv|md|docx|xlsx|pptx|odt|ods)$/i.test(
        name,
      )
    )
      return problem(422, "that kind of file is not accepted");
    const made: Attachment = {
      pid: `att-${Date.now()}-${held.length}`,
      name,
      content_type: "application/octet-stream",
      size_bytes: Number(query.get("size") ?? 0),
      sha256: "0".repeat(64),
      uploaded_by: "worker:demo",
      created_at: new Date().toISOString(),
    };
    held.push(made);
    return ok(made);
  }
  return problem(405, "method not allowed");
}

function attachmentDelete(pid: string) {
  for (const list of Object.values(state.attachments)) {
    const at = list.findIndex((a) => a.pid === pid);
    if (at >= 0) {
      list.splice(at, 1);
      return ok({ pid });
    }
  }
  return problem(404, "attachment not found");
}

const CUSTOM_ENTITY = {
  contacts: "contact",
  accounts: "account",
  leads: "lead",
  deals: "deal",
} as const;

/** The demo's declared custom fields: listed, declared and deleted. */
function customFieldDefs(
  method: string,
  id: string | undefined,
  body: unknown,
) {
  if (method === "GET" && !id) return ok(state.customFields);
  if (method === "POST" && !id) {
    const input = bodyOf(body) as Partial<CustomField>;
    if (!input.key || !/^[a-z][a-z0-9_]{0,39}$/.test(input.key))
      return problem(
        422,
        "key must be lower-case letters, digits and underscores",
      );
    if (
      state.customFields.some(
        (f) => f.entity === input.entity && f.key === input.key,
      )
    )
      return problem(
        422,
        `a ${input.entity} already has a custom field with the key ${input.key}`,
      );
    const made: CustomField = {
      pid: `cf-${state.customFields.length + 1}-${Date.now()}`,
      entity: input.entity ?? "contact",
      key: input.key,
      label: input.label?.trim() || input.key,
      kind: input.kind ?? "text",
      choices: input.choices ?? [],
      sensitivity: input.sensitivity ?? "business",
      position: state.customFields.length,
    };
    state.customFields.push(made);
    return ok({ pid: made.pid });
  }
  const at = state.customFields.findIndex((f) => f.pid === id);
  if (at < 0) return problem(404, "custom field not found");
  if (method === "DELETE") {
    state.customFields.splice(at, 1);
    for (const k of Object.keys(state.customValues))
      if (k.startsWith(`${id}:`)) delete state.customValues[k];
    return ok({ pid: id });
  }
  return problem(405, "method not allowed");
}

/** The demo's custom field values on one record. */
function customFieldValues(
  head: string | undefined,
  pid: string,
  method: string,
  body: unknown,
) {
  const entity = CUSTOM_ENTITY[head as keyof typeof CUSTOM_ENTITY];
  if (!entity) return problem(404, "not found");
  const rows = (): CustomFieldValue[] =>
    state.customFields
      .filter((f) => f.entity === entity)
      .map((f) => ({
        key: f.key,
        label: f.label,
        kind: f.kind,
        sensitivity: f.sensitivity,
        choices: f.choices,
        value: state.customValues[`${f.pid}:${pid}`] ?? null,
        masked: false,
      }));
  if (method === "GET") return ok(rows());
  if (method === "PUT") {
    const values = (bodyOf(body).values ?? {}) as Record<string, unknown>;
    if (!Object.keys(values).length) return problem(422, "nothing to change");
    for (const [key, value] of Object.entries(values)) {
      const def = state.customFields.find(
        (f) => f.entity === entity && f.key === key,
      );
      if (!def)
        return problem(422, `${key} is not a custom field of a ${entity}`);
      if (value === null) {
        delete state.customValues[`${def.pid}:${pid}`];
        continue;
      }
      if (def.kind === "number" && typeof value !== "number")
        return problem(422, `${key} must be a number`);
      if (def.kind === "choice" && !def.choices.includes(String(value)))
        return problem(422, `${key} must be one of ${def.choices.join(", ")}`);
      state.customValues[`${def.pid}:${pid}`] = value as string | number;
    }
    return ok(rows());
  }
  return problem(405, "method not allowed");
}

function emailTemplates(method: string, id: string | undefined, body: unknown) {
  const input = (body ?? {}) as Partial<EmailTemplate> & { blank?: string[] };
  if (method === "GET" && id === "tokens") return ok(MERGE_TOKENS);
  if (method === "GET" && !id) return ok(state.templates);
  if (method === "POST" && id === "preview") {
    const out = renderDraft(
      {
        subject: input.subject ?? "",
        body: input.body ?? "",
        fallbacks: input.fallbacks ?? {},
      },
      input.blank ?? [],
    );
    return "unknown" in out
      ? problem(422, `{{${out.unknown}}} is not a merge token`)
      : ok(out);
  }
  if (method === "POST" && !id) {
    const problems = renderDraft(
      { subject: input.subject ?? "", body: input.body ?? "", fallbacks: {} },
      [],
    );
    if (!input.name?.trim()) return problem(422, "name is required");
    if ("unknown" in problems)
      return problem(422, `{{${problems.unknown}}} is not a merge token`);
    if (problems.problems.length) return problem(422, problems.problems[0]!);
    const made: EmailTemplate = {
      pid: `template-${state.templates.length + 1}-${Date.now()}`,
      name: input.name.trim(),
      subject: input.subject ?? "",
      body: input.body ?? "",
      fallbacks: input.fallbacks ?? {},
      in_use: 0,
      updated_at: new Date().toISOString(),
    };
    state.templates.push(made);
    return ok({ pid: made.pid });
  }
  const at = state.templates.findIndex((x) => x.pid === id);
  if (at < 0) return problem(404, "template not found");
  if (method === "GET") return ok(state.templates[at]);
  if (method === "PUT") {
    state.templates[at] = { ...state.templates[at]!, ...input, pid: id! };
    return ok({ pid: id });
  }
  if (method === "DELETE") {
    state.templates.splice(at, 1);
    return ok({ pid: id });
  }
  return problem(405, "method not allowed");
}
