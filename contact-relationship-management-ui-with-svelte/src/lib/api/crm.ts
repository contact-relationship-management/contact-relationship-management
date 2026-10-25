// Typed CRM API surface over the BFF proxy, plus the family `money()`
// formatter. Paths mirror the service routes one-to-one — the
// Playwright suite stubs these exact paths, so drift fails loudly.

import { api, apiAll } from "#lib/api/client.js";

type FetchLike = { fetch?: typeof fetch };

/** One contact wrapper. */
export interface Contact {
  pid: string;
  person_ref: string;
  account_pid: string | null;
  owner_ref: string | null;
  display_name: string;
  given_name: string | null;
  family_name: string | null;
  status: string;
  job_title: string | null;
  preferred_channel: string;
  marketing_consent: string;
}

/** One account wrapper. */
export interface Account {
  pid: string;
  organization_ref: string;
  display_name: string;
  tier: string;
  industry: string | null;
}

/** One activity row. */
export interface Activity {
  pid: string;
  subject_kind: string;
  subject_pid: string;
  kind: string;
  occurred_at: string;
  summary: string;
}

/** One lead (score is derived). */
export interface Lead {
  pid: string;
  source: string;
  display_name: string;
  email: string | null;
  score: number;
  status: string;
}

/** One score-breakdown rule. */
export interface RuleScore {
  rule: string;
  points: number;
}

/** The score + explanation. */
export interface ScoreBreakdown {
  score: number;
  label: string;
  rules: RuleScore[];
}

/** One pipeline stage. */
export interface Stage {
  pid: string;
  name: string;
  position: number;
  probability_percent: number;
  is_won: boolean;
  is_lost: boolean;
}

/** One deal (amount may arrive null when masked). */
export interface Deal {
  pid: string;
  name: string;
  pipeline_pid: string;
  stage_pid: string;
  amount_minor: number | null;
  currency: string;
  won: boolean;
  closed_at: string | null;
  owner_ref?: string | null;
  expected_close_on?: string | null;
}

/** One campaign with counters. */
export interface Campaign {
  pid: string;
  name: string;
  status: string;
  /** `null` when the caller's policy masks amounts. */
  cost_minor: number | null;
  currency: string;
  recipients: number;
  delivered: number;
  opened: number;
  clicked: number;
  /** When the campaign was created and last changed (ISO timestamps). */
  created_at?: string;
  updated_at?: string;
}

/** One ticket with live breach flags. */
export interface Ticket {
  pid: string;
  title: string;
  priority: string;
  status: string;
  channel?: string;
  first_response_due_at: string | null;
  live_first_response_breached?: boolean;
  live_resolution_breached?: boolean;
}

/** One KB article. */
export interface Article {
  pid: string;
  title: string;
  body: string;
  status: string;
  version: number;
}

/** An honest ratio (null value on a zero denominator). */
export interface Ratio {
  numerator: number;
  denominator: number;
  value: number | null;
}

/**
 * Format minor units + ISO-4217 as a locale-aware money string.
 * `null` renders as an em dash (the masked/absent state) — never 0.
 */
export function money(
  minor: number | null | undefined,
  currency: string | null | undefined,
  locale?: string,
): string {
  if (minor === null || minor === undefined || !currency) return "—";
  return new Intl.NumberFormat(locale, {
    style: "currency",
    currency,
  }).format(minor / 100);
}

/** Contacts. */
export function listContacts(init?: FetchLike): Promise<Contact[]> {
  return apiAll("/contacts", init);
}

/** One contact + timeline. */
export function getContact(
  pid: string,
  init?: FetchLike,
): Promise<{
  contact: Contact;
  activities: Activity[];
  deals: Deal[];
  tickets: Ticket[];
}> {
  return api(`/contacts/${pid}`, init);
}

/** One field's part in a duplicate verdict; names fields, never values. */
export interface DuplicateField {
  field: string;
  outcome: "agree" | "partial" | "disagree" | "missing";
  similarity: number;
  weight: number;
}

/** A contact that is probably the same person as another (CRM-R23). */
export interface Duplicate {
  contact: {
    pid: string;
    display_name: string;
    account_pid: string | null;
    job_title: string | null;
    status: string;
  };
  probability: number;
  verdict: "match" | "review";
  deterministic: boolean;
  email_checked: boolean;
  fields: DuplicateField[];
}

/** Likely duplicates of one contact, best first. Advice only. */
export function getContactDuplicates(
  pid: string,
  init?: FetchLike,
): Promise<{
  contact_pid: string;
  compared: number;
  truncated: boolean;
  duplicates: Duplicate[];
}> {
  return api(`/contacts/${pid}/duplicates`, init);
}

/** What a merge moves or drops, by table, and what consent becomes (CRM-R24). */
export interface MergeResult {
  survivor_pid: string;
  merged_pid: string;
  dry_run: boolean;
  moved: Record<string, number>;
  dropped: Record<string, number>;
  filled: string[];
  consent: { survivor: string; merged: string; result: string };
}

/**
 * Merge the contact `pid` into `intoPid`, which survives (CRM-R24).
 * Destructive-classified. With `dryRun` nothing changes and the counts are
 * exact. Consent never widens: the stricter of the two wins.
 */
export function mergeContact(
  pid: string,
  intoPid: string,
  reason: string,
  dryRun: boolean,
): Promise<MergeResult> {
  return api(`/contacts/${pid}/merge`, {
    method: "POST",
    body: { into_pid: intoPid, reason, dry_run: dryRun },
  });
}

/** A merge that can still be looked at, and whether it can be undone. */
export interface MergeEntry {
  merged_pid: string;
  merged_name: string;
  merged_at: string;
  undoable: boolean;
}

/** Contacts merged into this one (CRM-R26). */
export function getContactMerges(
  pid: string,
): Promise<{ merges: MergeEntry[] }> {
  return api(`/contacts/${pid}/merges`);
}

/** What an unmerge restored, or with `dry_run` would restore. */
export interface UnmergeResult {
  survivor_pid: string;
  merged_pid: string;
  dry_run: boolean;
  restored: Record<string, number>;
  left_behind: Record<string, number>;
  dropped_restored: Record<string, number>;
  fields_cleared: string[];
  fields_kept: string[];
  consent: { survivor: string; merged_before: string; merged_result: string };
}

/**
 * Undo the merge of the contact `pid` (CRM-R26). Destructive-classified.
 * Consent never widens: the returning contact gets the stricter of its own
 * state and the survivor's.
 */
export function unmergeContact(
  pid: string,
  reason: string,
  dryRun: boolean,
): Promise<UnmergeResult> {
  return api(`/contacts/${pid}/unmerge`, {
    method: "POST",
    body: { reason, dry_run: dryRun },
  });
}

/** The systems a link can point at (CRM-R27); brand names, so not translated. */
export const LINK_SYSTEMS = [
  "hubspot",
  "salesforce",
  "attio",
  "pipedrive",
  "dynamics",
  "other",
] as const;
export type LinkSystem = (typeof LINK_SYSTEMS)[number];

/** A link from a record to the same record in another system. */
export interface ExternalLink {
  pid: string;
  subject_kind: string;
  subject_pid: string;
  system: LinkSystem;
  url: string;
  external_id: string | null;
  label: string | null;
}

/** The records that can carry links (CRM-R27). */
export type LinkKind = "contacts" | "accounts" | "deals" | "leads";

/** The links on a record. */
export function getLinks(
  kind: LinkKind,
  pid: string,
): Promise<{ links: ExternalLink[] }> {
  return api(`/${kind}/${pid}/external-links`);
}

/**
 * Add a link (CRM-R27). The service accepts only `https` addresses, and for a
 * named system only that system's own domains; it refuses a repeat with 409.
 */
export function addLink(
  kind: LinkKind,
  pid: string,
  link: { system: LinkSystem; url: string; label: string },
): Promise<ExternalLink> {
  return api(`/${kind}/${pid}/external-links`, {
    method: "POST",
    body: {
      system: link.system,
      url: link.url,
      label: link.label.trim() || null,
    },
  });
}

/** Remove a link. */
export function removeLink(linkPid: string): Promise<unknown> {
  return api(`/external-links/${linkPid}`, { method: "DELETE" });
}

/** Record a consent change. */
export function recordConsent(
  pid: string,
  action: "granted" | "withdrawn",
): Promise<unknown> {
  return api(`/contacts/${pid}/consent`, {
    method: "POST",
    body: { action, source: "operator ui" },
  });
}

/**
 * Change a contact's own fields (CRM-R38). Only the members sent change;
 * `null` clears an optional one.
 */
export function editContact(
  pid: string,
  fields: {
    display_name?: string;
    given_name?: string | null;
    family_name?: string | null;
    job_title?: string | null;
  },
): Promise<Contact> {
  return api(`/contacts/${pid}`, { method: "PUT", body: fields });
}

/** Change an account's own fields (CRM-R39); `null` clears an optional one. */
export function editAccount(
  pid: string,
  fields: {
    display_name?: string;
    tier?: string;
    industry?: string | null;
  },
): Promise<Account> {
  return api(`/accounts/${pid}`, { method: "PUT", body: fields });
}

/**
 * Erase (anonymise) a contact (CRM-R20). Destructive; the service
 * refuses `422` while an open deal, an open ticket, or an active
 * nurture enrolment exists.
 */
export function eraseContact(
  pid: string,
): Promise<{ erased: string; note: string }> {
  return api(`/contacts/${pid}/erase`, { method: "POST" });
}

/** One account with its contacts and deals. */
export function getAccount(
  pid: string,
  init?: FetchLike,
): Promise<{
  account: Account;
  contacts: { pid: string; display_name: string }[];
  deals: Deal[];
}> {
  return api(`/accounts/${pid}`, init);
}

/** Accounts. */
export function listAccounts(init?: FetchLike): Promise<Account[]> {
  return apiAll("/accounts", init);
}

/** The score-sorted lead queue. */
export function listLeads(init?: FetchLike): Promise<Lead[]> {
  return apiAll("/leads", init);
}

/** One lead + live breakdown. */
export function getLead(
  pid: string,
  init?: FetchLike,
): Promise<{ lead: Lead; score: ScoreBreakdown }> {
  return api(`/leads/${pid}`, init);
}

/** Edit a lead in place (its display name). */
export function editLead(
  pid: string,
  fields: { display_name: string },
): Promise<Lead> {
  return api(`/leads/${pid}`, { method: "PUT", body: fields });
}

/** Lead status transition. */
export function leadStatus(pid: string, to: string): Promise<Lead> {
  return api(`/leads/${pid}/status`, { method: "POST", body: { to } });
}

/** Pipelines with stages. */
export function listPipelines(
  init?: FetchLike,
): Promise<{ pipeline: { pid: string; name: string }; stages: Stage[] }[]> {
  return api("/pipelines", init);
}

/** Deals (optionally by pipeline). */
export function listDeals(
  pipeline?: string,
  init?: FetchLike,
): Promise<Deal[]> {
  return apiAll(`/deals${pipeline ? `?pipeline=${pipeline}` : ""}`, init);
}

/** Move a deal to a stage. */
export function moveDeal(
  pid: string,
  stagePid: string,
  lostReason?: string,
): Promise<Deal> {
  return api(`/deals/${pid}/stage`, {
    method: "POST",
    body: {
      stage_pid: stagePid,
      ...(lostReason ? { lost_reason: lostReason } : {}),
    },
  });
}

/** Reopen a closed deal into the pipeline's last open stage; a reason is required. */
export function reopenDeal(pid: string, reason: string): Promise<Deal> {
  return api(`/deals/${pid}/reopen`, { method: "POST", body: { reason } });
}

/** Edit an open deal in place (name, amount, expected close). */
export function editDeal(
  pid: string,
  fields: { name?: string; amount_minor?: number; expected_close_on?: string },
): Promise<Deal> {
  return api(`/deals/${pid}`, { method: "PUT", body: fields });
}

/** Store a card's place (0-based) among the cards in its column. */
export function placeCard(
  kind: "deals" | "leads" | "tickets",
  pid: string,
  position: number,
): Promise<unknown> {
  return api(`/${kind}/${pid}/position`, { method: "PUT", body: { position } });
}

/** Change a follow-up's due day or summary (a due-day change is audited as a reschedule). */
export function editFollowup(
  pid: string,
  fields: { due_on?: string; summary?: string },
): Promise<unknown> {
  return api(`/activities/${pid}`, { method: "PUT", body: fields });
}

/** A saved view of a list (CRM-R35): a filter, a sort and columns, no record data. */
export interface SavedView {
  pid: string;
  owner_ref: string;
  /** Whether the caller owns it. */
  mine: boolean;
  kind: string;
  name: string;
  filter: Record<string, string[]>;
  sort: { field: string; direction: "asc" | "desc" } | null;
  columns: string[] | null;
  shared: boolean;
}

/** What is sent to save or replace a view. */
export type SavedViewInput = Pick<
  SavedView,
  "kind" | "name" | "filter" | "sort" | "columns" | "shared"
>;

/** The caller's views of one list, and everyone's shared ones. */
export function listViews(
  kind: string,
  init?: FetchLike,
): Promise<SavedView[]> {
  return api(`/views?kind=${encodeURIComponent(kind)}`, init);
}

/** Save a new view. */
export function saveView(view: SavedViewInput): Promise<{ pid: string }> {
  return api("/views", { method: "POST", body: view });
}

/** Replace a view you own. */
export function replaceView(
  pid: string,
  view: SavedViewInput,
): Promise<{ pid: string }> {
  return api(`/views/${pid}`, { method: "PUT", body: view });
}

/** Delete a view you own. */
export function deleteView(pid: string): Promise<{ pid: string }> {
  return api(`/views/${pid}`, { method: "DELETE" });
}

/** An email template with merge tokens (CRM-R36). */
export interface EmailTemplate {
  pid: string;
  name: string;
  subject: string;
  body: string;
  fallbacks: Record<string, string>;
  /** Campaigns and nurture steps that use it. */
  in_use: number;
  updated_at: string;
}

/** What is sent to save or replace a template. */
export type EmailTemplateInput = Pick<
  EmailTemplate,
  "name" | "subject" | "body" | "fallbacks"
>;

/** One merge token; `unsubscribe_url` is required in every body and has no fallback. */
export interface MergeToken {
  name: string;
  description: string;
  fallback: string | null;
  required: boolean;
}

/** A draft rendered for a made-up contact. */
export interface TemplatePreview {
  subject: string;
  text: string;
  html: string;
  fell_back: string[];
  /** What would stop the draft being saved. */
  problems: string[];
}

/** Email templates. */
export function listTemplates(init?: FetchLike): Promise<EmailTemplate[]> {
  return api("/email-templates", init);
}

/** The merge tokens and their default fallbacks. */
export function mergeTokens(init?: FetchLike): Promise<MergeToken[]> {
  return api("/email-templates/tokens", init);
}

/** Save a new template. */
export function saveTemplate(
  template: EmailTemplateInput,
): Promise<{ pid: string }> {
  return api("/email-templates", { method: "POST", body: template });
}

/** Replace a template. */
export function replaceTemplate(
  pid: string,
  template: EmailTemplateInput,
): Promise<{ pid: string }> {
  return api(`/email-templates/${pid}`, { method: "PUT", body: template });
}

/** Delete a template; refused while a campaign or step uses it. */
export function deleteTemplate(pid: string): Promise<{ pid: string }> {
  return api(`/email-templates/${pid}`, { method: "DELETE" });
}

/** Render a draft for a made-up contact; `blank` lists tokens to treat as missing. */
export function previewTemplate(
  draft: Omit<EmailTemplateInput, "name">,
  blank: string[],
): Promise<TemplatePreview> {
  return api("/email-templates/preview", {
    method: "POST",
    body: { ...draft, blank },
  });
}

/** Which CSV column holds what (CRM-R34). */
export interface LeadImportMapping {
  display_name?: string | null;
  email?: string | null;
  company?: string | null;
}

/** One row of an import report: valid or invalid with a reason, and advice about duplicates. */
export interface LeadImportRow {
  row: number;
  status: "valid" | "invalid";
  reason: string | null;
  name: string | null;
  likely_duplicates: {
    kind: string;
    pid: string | null;
    row: number | null;
    name: string;
    probability: number;
    verdict: string;
  }[];
}

/** The answer to a preview: the columns and a suggestion, and the report once mapped. */
export interface LeadImportPreview {
  columns: string[];
  rows: number;
  suggested_mapping: LeadImportMapping;
  report: LeadImportRow[] | null;
  valid: number;
  invalid: number;
  likely_duplicates: number;
}

/** One import record. */
export interface LeadImport {
  pid: string;
  source_note: string;
  obtained_on: string;
  status: string;
  row_count: number;
  invalid_count: number;
  skipped_count: number;
  created_count: number;
  undone_count: number | null;
}

/** Read a CSV and report each row; saves nothing. */
export function previewLeadImport(
  csv: string,
  mapping?: LeadImportMapping,
): Promise<LeadImportPreview> {
  return api("/lead-imports/preview", {
    method: "POST",
    body: mapping ? { csv, mapping } : { csv },
  });
}

/** Save the valid rows as leads. */
export function commitLeadImport(fields: {
  csv: string;
  mapping: LeadImportMapping;
  source_note: string;
  obtained_on: string;
  skip_likely_duplicates: boolean;
}): Promise<LeadImport> {
  return api("/lead-imports", { method: "POST", body: fields });
}

/** Take an import back: new, untouched leads are removed, the rest kept. */
export function undoLeadImport(
  pid: string,
): Promise<{ removed: number; kept: number }> {
  return api(`/lead-imports/${pid}/undo`, { method: "POST" });
}

/** One search hit (CRM-R33): a record named, with one plain descriptor. */
export interface SearchHit {
  kind: string;
  pid: string;
  title: string;
  detail: string | null;
}

/** Records whose name or title matches, best first (the service's first page, up to 50). */
export function searchRecords(
  q: string,
  init?: FetchLike,
): Promise<SearchHit[]> {
  return api(`/search?q=${encodeURIComponent(q)}`, init);
}

/** One inbox entry (CRM-R32): which record, why, and whether it was read. No record text. */
export interface Notification {
  pid: string;
  /** `task_reminder` or `sla_breach`. */
  kind: string;
  subject_kind: string;
  subject_pid: string;
  created_at: string;
  read_at: string | null;
}

/** The caller's inbox, newest first, without records they may not read. */
export function notifications(
  unreadOnly = false,
  init?: FetchLike,
): Promise<Notification[]> {
  return api(`/notifications${unreadOnly ? "?unread=true" : ""}`, init);
}

/** How many unread notifications the caller may see. */
export function unreadNotifications(
  init?: FetchLike,
): Promise<{ unread: number; capped: boolean }> {
  return api("/notifications/unread-count", init);
}

/** Mark one notification read. */
export function markNotificationRead(pid: string): Promise<Notification> {
  return api(`/notifications/${pid}/read`, { method: "PUT" });
}

/** Mark the whole inbox read. */
export function markAllNotificationsRead(): Promise<{ marked: number }> {
  return api("/notifications/read-all", { method: "POST" });
}

/** The live forecast. */
export function forecast(init?: FetchLike): Promise<{
  as_of: string;
  open_deals: number;
  /** `null` when the caller's policy masks amounts. */
  totals_minor: Record<string, number> | null;
}> {
  return api("/forecast", init);
}

/** A saved audience (CRM-R7). */
export interface Segment {
  pid: string;
  name: string;
}

/** The saved audiences a campaign can be sent to. */
export function listSegments(init?: FetchLike): Promise<Segment[]> {
  return api("/segments", init);
}

/** What is sent to make a campaign: a template, or its own subject and body. */
export interface CampaignInput {
  name: string;
  segment_pid?: string;
  template_pid?: string;
  subject?: string;
  body?: string;
}

/** Make a campaign (a draft). */
export function createCampaign(input: CampaignInput): Promise<{ pid: string }> {
  return api("/campaigns", { method: "POST", body: input });
}

/** Campaigns. */
export function listCampaigns(init?: FetchLike): Promise<Campaign[]> {
  return api("/campaigns", init);
}

/** One campaign's funnel + ROI. */
export function campaignFunnel(
  pid: string,
  init?: FetchLike,
): Promise<{
  campaign: Campaign;
  leads: number;
  won_deals: number;
  /** `null` when the caller's policy masks amounts. */
  won_revenue_minor: number | null;
  roi: Ratio | null;
}> {
  return api(`/campaigns/${pid}/funnel`, init);
}

/** Run (simulated send). */
export function runCampaign(pid: string): Promise<Campaign> {
  return api(`/campaigns/${pid}/run`, { method: "POST" });
}

/** Campaign status transition. */
export function campaignStatus(pid: string, to: string): Promise<Campaign> {
  return api(`/campaigns/${pid}/status`, { method: "POST", body: { to } });
}

/** Tickets with live breach flags. */
export function listTickets(init?: FetchLike): Promise<Ticket[]> {
  return apiAll("/tickets", init);
}

/** Edit a ticket in place (its title). */
export function editTicket(
  pid: string,
  fields: { title: string },
): Promise<Ticket> {
  return api(`/tickets/${pid}`, { method: "PUT", body: fields });
}

/** Ticket status transition. */
export function ticketStatus(pid: string, to: string): Promise<Ticket> {
  return api(`/tickets/${pid}/status`, { method: "POST", body: { to } });
}

/** Articles (searchable). */
export function listArticles(q?: string, init?: FetchLike): Promise<Article[]> {
  return apiAll(`/articles${q ? `?q=${encodeURIComponent(q)}` : ""}`, init);
}

/** Publish / archive an article. */
export function articleStatus(pid: string, to: string): Promise<Article> {
  return api(`/articles/${pid}/status`, { method: "POST", body: { to } });
}

/** The sales dashboard. */
export function salesDashboard(init?: FetchLike): Promise<{
  win_rate: Ratio;
  open_deals: number;
  pipeline_by_stage: Record<string, unknown>;
}> {
  return api("/dashboards/sales", init);
}

/** The SLA dashboard. */
export function slaDashboard(init?: FetchLike): Promise<{
  open_tickets: number;
  by_priority: { priority: string; open: number; breached: number }[];
}> {
  return api("/dashboards/sla", init);
}

// ─── Insight views (read-only derivations; as_of + ETag) ────────────

/** `GET /insights/stale-deals` row. */
export interface StaleDeal {
  pid: string;
  name: string;
  stage: string | null;
  owner_ref: string | null;
  /** Null when the caller's policy masks amounts. */
  amount_minor: number | null;
  currency: string;
  days_in_stage: number;
  stale: boolean;
}

/** One open follow-up (activity with a due date, not done). */
export interface Followup {
  pid: string;
  kind: string;
  summary: string;
  subject_kind: string;
  subject_pid: string;
  actor_ref: string | null;
  due_on: string;
  overdue_days: number | null;
}

/** One rule-disclosed finding. */
export interface Finding {
  rule: string;
  detail: string;
  [key: string]: unknown;
}

/** Stale-deal aging (server derivation disclosed). */
export function staleDeals(
  days?: number,
  init?: FetchLike,
): Promise<{
  as_of: string;
  derivation: string;
  threshold_days: number;
  open_deals: number;
  stale_deals: number;
  deals: StaleDeal[];
}> {
  return api(`/insights/stale-deals${days ? `?days=${days}` : ""}`, init);
}

/** Open follow-ups: overdue + next 30 days (optional kind filter —
 * the renewals convention: due-dated `task` activities). */
export function followups(
  kind?: string,
  init?: FetchLike,
): Promise<{
  as_of: string;
  note: string;
  overdue: Followup[];
  upcoming_30d: Followup[];
  open_by_recorder: Record<string, number>;
}> {
  return api(
    `/insights/followups${kind ? `?kind=${encodeURIComponent(kind)}` : ""}`,
    init,
  );
}

/** Pipeline-hygiene findings. */
export function pipelineHygiene(
  days?: number,
  init?: FetchLike,
): Promise<{ as_of: string; threshold_days: number; findings: Finding[] }> {
  return api(`/insights/pipeline-hygiene${days ? `?days=${days}` : ""}`, init);
}

/** The period sales executive pack. */
export function executivePack(init?: FetchLike): Promise<{
  as_of: string;
  window: { from: string; to: string };
  deals_won: number;
  deals_lost: number;
  won_value_by_currency_minor: Record<string, number>;
  lost_reasons: Record<string, number>;
  new_leads: number;
  tickets_opened: number;
  tickets_resolved: number;
  campaigns_started: number;
  activities_logged: number;
  consent_withdrawals: number;
  note: string;
}> {
  return api("/insights/executive", init);
}

/** The stored forecast-snapshot series (no interpolation). */
export function forecastTrends(init?: FetchLike): Promise<{
  as_of: string;
  note: string;
  series: Array<{ taken_on: string; totals: Record<string, number> }>;
}> {
  return api("/insights/forecast-trends", init);
}

/** The SLA breach register + per-assignee workload. */
export function slaRegister(init?: FetchLike): Promise<{
  as_of: string;
  derivation: string;
  breaches: Array<{
    pid: string;
    title: string;
    priority: string;
    status: string;
    assignee_ref: string | null;
    breached: string;
    overdue_hours: number;
  }>;
  workload: Array<{
    assignee_ref: string;
    open: number;
    breached: number;
    at_risk_4h: number;
  }>;
}> {
  return api("/insights/sla", init);
}

/** The DPO view: consent coverage + duplicates. */
export function dpo(init?: FetchLike): Promise<{
  as_of: string;
  note: string;
  contacts: number;
  consent_coverage: Record<string, number>;
  window_days: number;
  withdrawals_in_window: number;
  consent_events_by_source: Record<string, number>;
  duplicate_person_refs: Array<{
    person_ref: string;
    contacts: Array<{ pid: string; display_name: string }>;
  }>;
}> {
  return api("/insights/dpo", init);
}

/** Relationship-cadence aging (untouched contacts/accounts). */
export function cadence(
  days?: number,
  init?: FetchLike,
): Promise<{
  as_of: string;
  derivation: string;
  threshold_days: number;
  untouched_contacts: Array<{
    pid: string;
    display_name: string;
    stakeholder_role: string | null;
    days_since_touch: number;
    has_next_touch: boolean;
  }>;
  untouched_accounts: Array<{
    pid: string;
    display_name: string;
    stakeholder_role: string | null;
    days_since_touch: number;
  }>;
  contacts_without_next_touch: number;
}> {
  return api(`/insights/cadence${days ? `?days=${days}` : ""}`, init);
}

/** Engagement workload (touches, kinds, recorded sentiment). */
export function engagementWorkload(
  days?: number,
  init?: FetchLike,
): Promise<{
  as_of: string;
  window_days: number;
  touches: number;
  per_recorder_month: Record<string, number>;
  per_kind: Record<string, number>;
  sentiment: Record<string, number>;
  note: string;
}> {
  return api(`/insights/engagement${days ? `?days=${days}` : ""}`, init);
}

/** Stage funnel for one pipeline (audit-derived, honest ratios). */
export function funnel(
  pipelinePid: string,
  init?: FetchLike,
): Promise<{
  as_of: string;
  pipeline: { pid: string; name: string };
  derivation: string;
  stages: Array<{
    stage: string;
    position: number;
    is_won: boolean;
    is_lost: boolean;
    entered: number;
    conversion_from_previous: {
      numerator: number;
      denominator: number;
      value: number | null;
    } | null;
  }>;
}> {
  return api(
    `/insights/funnel?pipeline=${encodeURIComponent(pipelinePid)}`,
    init,
  );
}

/** Member-account health (+ silent list). */
export function membersHealth(
  days?: number,
  init?: FetchLike,
): Promise<{
  as_of: string;
  derivation: string;
  threshold_days: number;
  silent_accounts: number;
  accounts: Array<{
    pid: string;
    display_name: string;
    tier: string;
    stakeholder_role: string | null;
    membership: {
      status: string;
      joined_on: string;
      renewal_on: string | null;
    } | null;
    contacts: number;
    days_since_touch: number;
    silent: boolean;
    open_followups: number;
    open_tickets: number;
  }>;
}> {
  return api(`/insights/members${days ? `?days=${days}` : ""}`, init);
}

/** Per-account consent rollup (DPO). */
export function consentByAccount(
  days?: number,
  init?: FetchLike,
): Promise<{
  as_of: string;
  window_days: number;
  note: string;
  accounts: Array<{
    pid: string;
    display_name: string;
    consent_coverage: Record<string, number>;
    withdrawals_in_window: number;
  }>;
}> {
  return api(
    `/insights/consent-by-account${days ? `?days=${days}` : ""}`,
    init,
  );
}

/** The declared-stakeholder register + power–interest grid. */
export function stakeholdersView(init?: FetchLike): Promise<{
  as_of: string;
  note: string;
  by_role: Record<
    string,
    Array<{
      pid: string;
      display_name: string;
      marketing_consent: string;
      influence: number | null;
      interest: number | null;
      days_since_touch: number;
    }>
  >;
  grid: Record<string, number>;
  stakeholders_without_grid_scores: number;
  undeclared_contacts: number;
  account_roles: Array<{ pid: string; display_name: string; role: string }>;
}> {
  return api("/insights/stakeholders", init);
}

/** The innovation-partnership register. */
export function partnershipsRegister(init?: FetchLike): Promise<{
  as_of: string;
  by_kind: Record<string, number>;
  by_stage: Record<string, number>;
  register: Array<{
    pid: string;
    account_pid: string;
    account: string | null;
    kind: string;
    stage: string;
    summary: string;
    started_on: string | null;
  }>;
}> {
  return api("/insights/partnerships", init);
}

/** Membership renewals due + the lapsed list. */
export function membershipsView(
  days?: number,
  init?: FetchLike,
): Promise<{
  as_of: string;
  window_days: number;
  memberships: number;
  renewals_due: Array<{
    pid: string;
    account: string | null;
    status: string;
    joined_on: string;
    renewal_on: string | null;
  }>;
  lapsed: Array<{ pid: string; account: string | null; status: string }>;
}> {
  return api(`/insights/memberships${days ? `?days=${days}` : ""}`, init);
}

/**
 * Take a time-limited emergency read elevation (CRM-T44). The reason is
 * recorded and reviewed later by someone else; the grant lifts masking for
 * reads only.
 */
export function requestBreakGlass(
  reason: string,
  minutes: number,
  idempotencyKey?: string,
): Promise<{ pid: string; expires_at: string; minutes: number }> {
  return api("/break-glass", {
    method: "POST",
    body: { reason, minutes },
    idempotencyKey,
  });
}

/** One break-glass grant as the review list shows it. */
export interface BreakGlassGrant {
  pid: string;
  subject: string;
  reason: string;
  granted_at: string;
  expires_at: string;
  reviewed_at: string | null;
  reviewed_by: string | null;
  review_note: string | null;
}

/** Grants newest first plus the unreviewed count (admin/service tier). */
export function listBreakGlass(
  init?: FetchLike,
): Promise<{ unreviewed: number; grants: BreakGlassGrant[] }> {
  return api("/break-glass", init);
}

/** Record the review of one grant; refused for the grantee, once only. */
export function reviewBreakGlass(
  pid: string,
  note: string,
  idempotencyKey?: string,
): Promise<{ pid: string; reviewed_by: string }> {
  return api(`/break-glass/${pid}/review`, {
    method: "POST",
    body: { note },
    idempotencyKey,
  });
}

/** What a workflow looks at: the segment filter language (CRM-R40). */
export interface WorkflowConditions {
  statuses: string[];
  channels: string[];
  account_tiers: string[];
}

/** One thing a workflow does. The set is closed. */
export type WorkflowAction =
  | {
      type: "create_task";
      summary: string;
      assignee_ref?: string | null;
      due_in_days?: number | null;
    }
  | { type: "notify"; recipient_ref: string }
  | { type: "set_field"; field: string; value: string | null }
  | { type: "enrol_nurture"; sequence_pid: string }
  | { type: "send_webhook"; event: string };

/** A workflow rule. */
export interface Workflow {
  pid: string;
  name: string;
  enabled: boolean;
  trigger: string;
  every_minutes: number | null;
  conditions: WorkflowConditions;
  actions: WorkflowAction[];
  updated_at: string;
}

/** What is sent to save or replace a workflow. */
export type WorkflowInput = Omit<Workflow, "pid" | "updated_at">;

/** One action of a run, or of a dry run. */
export interface WorkflowStep {
  action: string;
  result: string;
  detail: string;
}

/** One run: why it fired and what it did. */
export interface WorkflowRun {
  pid: string;
  workflow_pid: string;
  workflow_name: string;
  trigger: string;
  subject_kind: string;
  subject_pid: string;
  depth: number;
  outcome: string;
  reason: string;
  steps: WorkflowStep[];
  created_at: string;
}

/** What a workflow would do to the current data. */
export interface WorkflowDryRun {
  examined: number;
  would_fire: number;
  truncated: boolean;
  samples: { contact_pid: string; steps: WorkflowStep[] }[];
}

/** The workflow rules. */
export function listWorkflows(init?: FetchLike): Promise<Workflow[]> {
  return api("/workflows", init);
}

/** Make a workflow. */
export function saveWorkflow(input: WorkflowInput): Promise<{ pid: string }> {
  return api("/workflows", { method: "POST", body: input });
}

/** Replace a workflow. */
export function replaceWorkflow(
  pid: string,
  input: WorkflowInput,
): Promise<{ pid: string }> {
  return api(`/workflows/${pid}`, { method: "PUT", body: input });
}

/** Delete a workflow; its run log stays. */
export function deleteWorkflow(pid: string): Promise<{ pid: string }> {
  return api(`/workflows/${pid}`, { method: "DELETE" });
}

/** Try a workflow against the current data; changes and sends nothing. */
export function dryRunWorkflow(pid: string): Promise<WorkflowDryRun> {
  return api(`/workflows/${pid}/dry-run`, { method: "POST" });
}

/** The newest runs of a workflow. */
export function workflowRuns(
  pid: string,
  init?: FetchLike,
): Promise<WorkflowRun[]> {
  return api(`/workflows/${pid}/runs`, init);
}

/** A declared custom field (CRM-R41). */
export interface CustomField {
  pid: string;
  entity: "contact" | "account" | "lead" | "deal";
  key: string;
  label: string;
  kind: "text" | "number" | "date" | "choice";
  choices: string[];
  sensitivity: "business" | "personal" | "sensitive";
  position: number;
}

/** A custom field of one record, with its value. */
export interface CustomFieldValue {
  key: string;
  label: string;
  kind: CustomField["kind"];
  sensitivity: CustomField["sensitivity"];
  choices: string[];
  /** `null` when none is set, or when a masked read hides it. */
  value: string | number | null;
  masked: boolean;
}

/** What is sent to declare a field. */
export type CustomFieldInput = Pick<
  CustomField,
  "entity" | "key" | "label" | "kind" | "choices" | "sensitivity"
>;

/** The declared custom fields. */
export function listCustomFields(init?: FetchLike): Promise<CustomField[]> {
  return api("/custom-fields", init);
}

/** Declare a custom field. */
export function createCustomField(
  input: CustomFieldInput,
): Promise<{ pid: string }> {
  return api("/custom-fields", { method: "POST", body: input });
}

/** Delete a custom field and every value it holds. */
export function deleteCustomField(pid: string): Promise<{ pid: string }> {
  return api(`/custom-fields/${pid}`, { method: "DELETE" });
}

/** The custom fields of a record with their values. */
export function getCustomFieldValues(
  kind: LinkKind,
  pid: string,
  init?: FetchLike,
): Promise<CustomFieldValue[]> {
  return api(`/${kind}/${pid}/custom-fields`, init);
}

/** Set or clear (`null`) values on a record. */
export function setCustomFieldValues(
  kind: LinkKind,
  pid: string,
  values: Record<string, string | number | null>,
): Promise<CustomFieldValue[]> {
  return api(`/${kind}/${pid}/custom-fields`, {
    method: "PUT",
    body: { values },
  });
}
