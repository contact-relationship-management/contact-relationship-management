// Language in the address: `/de-de/contacts` shows the contacts page in German.
//
// The prefix is removed before routing, so every page is reachable under every
// language without a copy of its route; the address bar keeps the prefix. The
// one real `/<locale>/…` route (the kanban page) is left to the router.
import type { Reroute } from "@sveltejs/kit/hooks";
import { splitLocale } from "./params";

export const reroute: Reroute = ({ url }) => {
  const { locale, path } = splitLocale(url.pathname);
  if (locale === undefined || path === "/kanban") return undefined;
  return path;
};
