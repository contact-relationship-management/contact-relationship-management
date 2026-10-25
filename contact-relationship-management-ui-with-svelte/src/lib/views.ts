// Saved views (CRM-R35): what each list can filter and show, and how a view is applied.
// The service checks a view against the same closed sets when it is saved; applying one is
// done here, on the rows the caller's own list read returned, so sharing a view shares no
// access and a masked value cannot be filtered on.

import type { SavedView } from "#lib/api/crm.js";

/** The lists that have views. */
export type ViewKind = "contact" | "account" | "deal" | "lead" | "ticket";

/** What a list's view can filter on (field to allowed values) and show (column ids). */
export interface KindConfig {
  filters: Record<string, readonly string[]>;
  columns: readonly string[];
}

export const KINDS: Record<ViewKind, KindConfig> = {
  contact: {
    filters: {
      status: ["active", "inactive"],
      marketing_consent: ["granted", "withdrawn", "never", "pending"],
    },
    columns: ["display_name", "job_title", "status", "marketing_consent"],
  },
  account: {
    filters: { tier: ["prospect", "customer", "partner", "former"] },
    columns: ["display_name", "tier", "industry"],
  },
  deal: {
    filters: { state: ["open", "won", "lost"] },
    columns: ["name", "amount_minor", "currency", "state", "expected_close_on"],
  },
  lead: {
    filters: {
      source: ["web", "referral", "event", "import", "campaign"],
      status: ["new", "contacted", "qualified", "converted", "disqualified"],
    },
    columns: ["display_name", "source", "status", "score"],
  },
  ticket: {
    filters: {
      priority: ["low", "normal", "high", "urgent"],
      status: ["open", "pending", "resolved", "closed"],
      channel: ["email", "phone", "web", "chat"],
    },
    columns: ["title", "priority", "status", "channel"],
  },
};

/** The value a record has for a filterable field. */
function valueOf(item: Record<string, unknown>, field: string): string {
  if (field === "state") {
    // A deal is open until it closes, then won or lost.
    if (item.closed_at === null || item.closed_at === undefined) return "open";
    return item.won ? "won" : "lost";
  }
  const value = item[field];
  return value === null || value === undefined ? "" : String(value);
}

/** Keep the records a view's filter keeps: alternatives within a field, AND across fields. */
export function applyFilter<T extends object>(
  items: readonly T[],
  view: Pick<SavedView, "filter"> | null,
): T[] {
  const clauses = Object.entries(view?.filter ?? {});
  if (clauses.length === 0) return [...items];
  return items.filter((item) =>
    clauses.every(([field, values]) =>
      values.includes(valueOf(item as Record<string, unknown>, field)),
    ),
  );
}

/** Sort rows and keep the view's columns, in its order. */
export function applyShape<
  R extends Record<string, unknown>,
  C extends { id: string },
>(
  rows: readonly R[],
  columns: readonly C[],
  view: Pick<SavedView, "sort" | "columns"> | null,
): { rows: R[]; columns: C[] } {
  let out = [...rows];
  const sort = view?.sort;
  if (sort) {
    const sign = sort.direction === "desc" ? -1 : 1;
    out.sort((a, b) => {
      const x = a[sort.field];
      const y = b[sort.field];
      if (typeof x === "number" && typeof y === "number") return sign * (x - y);
      return (
        sign *
        String(x ?? "").localeCompare(String(y ?? ""), undefined, {
          numeric: true,
        })
      );
    });
  }
  const wanted = view?.columns;
  const shown = wanted
    ? wanted
        .map((id) => columns.find((c) => c.id === id))
        .filter((c): c is C => c !== undefined)
    : [...columns];
  // A view that names no column this list has still shows the list.
  return { rows: out, columns: shown.length > 0 ? shown : [...columns] };
}
