// Compile-time check of the hand-written types in `crm.ts` against the types generated from the
// service's OpenAPI document (`schema.d.ts`, see `pnpm api:types`). Never imported at run time.
//
// The direction that matters: what the service says it sends must be usable as what the UI says
// it reads. A field the UI expects but the service does not send, or sends with a wider type,
// is an error here. Extra fields the service sends are fine.

import type * as ui from "#lib/api/crm.js";
import type { components } from "#lib/api/schema.js";

type Sent = components["schemas"];

/** Compiles only when `Server` can stand in for `Client`. */
type Serves<Client, Server extends Client> = [Client, Server];

export type Checks = [
  Serves<ui.Contact, Sent["Contact"]>,
  Serves<ui.Account, Sent["Account"]>,
  Serves<ui.Activity, Sent["Activity"]>,
  Serves<ui.Lead, Sent["Lead"]>,
  Serves<ui.RuleScore, Sent["RuleScore"]>,
  Serves<ui.ScoreBreakdown, Sent["ScoreBreakdown"]>,
  Serves<ui.Deal, Sent["Deal"]>,
  Serves<ui.Campaign, Sent["Campaign"]>,
  Serves<ui.Attribution, Sent["Attribution"]>,
  Serves<ui.FxRate, Sent["FxRate"]>,
  Serves<ui.Ticket, Sent["Ticket"]>,
  Serves<ui.Article, Sent["Article"]>,
  // The service stores `system` as free text it validates on write; the UI narrows it to the
  // systems it offers.
  Serves<Omit<ui.ExternalLink, "system">, Sent["ExternalLink"]>,
  Serves<ui.Followup, Sent["Followup"]>,
  Serves<ui.Stage, Sent["StageRecord"]>,
  Serves<ui.Ratio, Sent["Ratio"]>,
  // `verdict` and `outcome` are closed sets the UI narrows; the document types them as strings.
  // (`fields` is checked on its own line.)
  Serves<
    Omit<ui.Duplicate, "verdict" | "fields">,
    Omit<Sent["ContactDuplicate"], "verdict" | "fields">
  >,
  Serves<
    Omit<ui.DuplicateField, "outcome">,
    Omit<Sent["ContactDuplicateField"], "outcome">
  >,
  Serves<ui.MergeEntry, Sent["ContactMergeEntry"]>,
  Serves<ui.MergeResult, Sent["ContactMerge"]>,
  Serves<ui.UnmergeResult, Sent["ContactUnmerge"]>,
  Serves<ui.StaleDeal, Sent["StaleDeal"]>,
  Serves<ui.BreakGlassGrant, Sent["GrantRecord"]>,
];
