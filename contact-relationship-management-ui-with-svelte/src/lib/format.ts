// Pure rendering helpers for the honest-ratio, deal-board and SLA views
// (CRM-D10). Kept free of Svelte so vitest can pin the edge cases.

import type { Ratio, Stage, Ticket } from "./api/crm";

const DASH = "—";

/** A ratio as a whole percent; a zero denominator (null value) is a dash, never 0%. */
export function ratioPercent(ratio: Ratio | null | undefined): string {
  if (!ratio || ratio.value === null) return DASH;
  return `${Math.round(ratio.value * 100)}%`;
}

/** The "n/d" provenance of a ratio, or empty when there is none. */
export function ratioFraction(ratio: Ratio | null | undefined): string {
  return ratio ? `${ratio.numerator}/${ratio.denominator}` : "";
}

/** Percent plus provenance, e.g. "50% (1/2)"; `noData` replaces a null value. */
export function ratioLabel(ratio: Ratio | null | undefined, noData = DASH): string {
  if (!ratio) return noData;
  if (ratio.value === null) return noData;
  return `${ratioPercent(ratio)} (${ratioFraction(ratio)})`;
}

/** A board column header: stage name and its win probability. */
export function stageLabel(stage: Pick<Stage, "name" | "probability_percent">): string {
  return `${stage.name} ${stage.probability_percent}%`;
}

/** Whether either live SLA clock has breached. */
export function slaBreached(
  ticket: Pick<Ticket, "live_first_response_breached" | "live_resolution_breached">,
): boolean {
  return Boolean(ticket.live_first_response_breached || ticket.live_resolution_breached);
}

/** How many tickets in the list have breached. */
export function breachedCount(
  tickets: readonly Pick<Ticket, "live_first_response_breached" | "live_resolution_breached">[],
): number {
  return tickets.filter(slaBreached).length;
}
