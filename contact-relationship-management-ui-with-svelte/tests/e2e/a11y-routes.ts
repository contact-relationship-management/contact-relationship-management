// Every page route and how the accessibility gate reaches it (CRM-R77). The axe run
// (`a11y.spec.ts`) scans each entry; `tests/unit/a11y-routes.test.ts` fails if a page exists that is
// not listed here, or an entry names a page that does not exist, so a new route cannot go unscanned.

/** How the gate gets to a page. */
export type Mode =
  /** Signed out, as a visitor. */
  | "public"
  /** In the tour, with its sample data (signed in as the demo user). */
  | "tour";

export interface Entry {
  /** The route as the file tree names it: `/accounts/[pid]`. */
  route: string;
  /** A real address for it (a sample id in place of `[pid]`). */
  path: string;
  mode: Mode;
  /**
   * Why it is **not** scanned, in words. An excluded page is still listed, so it cannot be
   * forgotten; leave this out for every page that is scanned.
   */
  excluded?: string;
}

export const ROUTES: Entry[] = [
  { route: "/", path: "/", mode: "public" },
  { route: "/tour", path: "/tour", mode: "public" },
  { route: "/signin", path: "/signin", mode: "public" },
  { route: "/accessibility", path: "/accessibility", mode: "public" },
  { route: "/verify", path: "/verify", mode: "public" },
  { route: "/help", path: "/help", mode: "tour" },
  { route: "/help/[pid]", path: "/help/kb-1", mode: "tour" },
  { route: "/[locale=locale]/kanban", path: "/de-de/kanban", mode: "tour" },
  { route: "/accounts", path: "/accounts", mode: "tour" },
  { route: "/accounts/[pid]", path: "/accounts/acc-1", mode: "tour" },
  { route: "/accounts/import", path: "/accounts/import", mode: "tour" },
  { route: "/articles", path: "/articles", mode: "tour" },
  { route: "/attribution", path: "/attribution", mode: "tour" },
  { route: "/break-glass", path: "/break-glass", mode: "tour" },
  { route: "/break-glass/review", path: "/break-glass/review", mode: "tour" },
  { route: "/campaigns", path: "/campaigns", mode: "tour" },
  { route: "/contacts", path: "/contacts", mode: "tour" },
  { route: "/contacts/[pid]", path: "/contacts/con-1", mode: "tour" },
  { route: "/contacts/import", path: "/contacts/import", mode: "tour" },
  { route: "/custom-fields", path: "/custom-fields", mode: "tour" },
  { route: "/deals", path: "/deals", mode: "tour" },
  { route: "/deals/[pid]", path: "/deals/deal-10", mode: "tour" },
  { route: "/dpo", path: "/dpo", mode: "tour" },
  { route: "/engagement", path: "/engagement", mode: "tour" },
  { route: "/executive", path: "/executive", mode: "tour" },
  { route: "/followups", path: "/followups", mode: "tour" },
  { route: "/fx-rates", path: "/fx-rates", mode: "tour" },
  { route: "/leads", path: "/leads", mode: "tour" },
  { route: "/leads/[pid]", path: "/leads/lead-1", mode: "tour" },
  { route: "/leads/board", path: "/leads/board", mode: "tour" },
  { route: "/leads/import", path: "/leads/import", mode: "tour" },
  { route: "/notifications", path: "/notifications", mode: "tour" },
  { route: "/partners", path: "/partners", mode: "tour" },
  { route: "/quotas", path: "/quotas", mode: "tour" },
  { route: "/reports", path: "/reports", mode: "tour" },
  { route: "/search", path: "/search", mode: "tour" },
  { route: "/templates", path: "/templates", mode: "tour" },
  { route: "/tickets", path: "/tickets", mode: "tour" },
  { route: "/tickets/board", path: "/tickets/board", mode: "tour" },
  { route: "/workflows", path: "/workflows", mode: "tour" },
];
