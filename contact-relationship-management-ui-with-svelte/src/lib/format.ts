// Pure rendering helpers for the honest-ratio, deal-board and SLA views
// (CRM-D10). Kept free of Svelte so vitest can pin the edge cases.

// Percentages and counts go through `locale-format.ts` in the page's locale (CRM-T172), so a French
// page reads "50 %" and a German "1.234/5.000".

import type { Ratio, Stage, Ticket } from "./api/crm";
import {
  DASH,
  formatInteger,
  formatPercent,
  formatPercentValue,
} from "./locale-format.js";

/** A ratio as a whole percent; a zero denominator (null value) is a dash, never 0%. */
export function ratioPercent(
  ratio: Ratio | null | undefined,
  locale?: string,
): string {
  if (!ratio || ratio.value === null) return DASH;
  return formatPercent(ratio.value, 0, locale);
}

/** The "n/d" provenance of a ratio, or empty when there is none. */
export function ratioFraction(
  ratio: Ratio | null | undefined,
  locale?: string,
): string {
  return ratio
    ? `${formatInteger(ratio.numerator, locale)}/${formatInteger(ratio.denominator, locale)}`
    : "";
}

/** Percent plus provenance, e.g. "50% (1/2)"; `noData` replaces a null value. */
export function ratioLabel(
  ratio: Ratio | null | undefined,
  noData = DASH,
  locale?: string,
): string {
  if (!ratio) return noData;
  if (ratio.value === null) return noData;
  return `${ratioPercent(ratio, locale)} (${ratioFraction(ratio, locale)})`;
}

/** A board column header: stage name and its win probability. */
export function stageLabel(
  stage: Pick<Stage, "name" | "probability_percent">,
  locale?: string,
): string {
  return `${stage.name} ${formatPercentValue(stage.probability_percent, 0, locale)}`;
}

/** Whether either live SLA clock has breached. */
export function slaBreached(
  ticket: Pick<
    Ticket,
    "live_first_response_breached" | "live_resolution_breached"
  >,
): boolean {
  return Boolean(
    ticket.live_first_response_breached || ticket.live_resolution_breached,
  );
}

/** How many tickets in the list have breached. */
export function breachedCount(
  tickets: readonly Pick<
    Ticket,
    "live_first_response_breached" | "live_resolution_breached"
  >[],
): number {
  return tickets.filter(slaBreached).length;
}
