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
  ): Lead => ({ pid, source, display_name: name, email, score, status });
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

function forecastNow() {
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
      const base = forecastNow().totals_minor.GBP ?? 0;
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
    if (seg.length === 1 && get) return ok(state.contacts);
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
  if (head === "email-templates") return emailTemplates(method, id, body);
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
  if (head === "accounts" && seg.length === 1 && get) return ok(state.accounts);
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
    if (seg.length === 1 && get) return ok(state.leads);
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
  if (head === "forecast" && get) return ok(forecastNow());
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
    if (tail === "status" && post) {
      const reply = transition(state.tickets, id!, body, "ticket");
      const t = state.tickets.find((x) => x.pid === id);
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
