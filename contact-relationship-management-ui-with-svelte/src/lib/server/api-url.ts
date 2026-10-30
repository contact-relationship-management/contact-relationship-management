// The service URL the BFF proxy forwards to, kept free of `$app` imports so a
// unit test can pin it.

/** The service's API base when `CRM_API_URL` is unset: the Loco service's
 *  `/api` path on its dev port. The proxy appends the path after
 *  `/api/proxy/`, so the base must already end in `/api`. */
export const DEFAULT_CRM_API_URL = "http://localhost:5150/api";

/** `CRM_API_URL` as the proxy uses it: the default when unset or blank,
 *  otherwise the value without trailing slashes, so `<base>/<path>` never
 *  doubles the slash. No `/api` is added: a service behind another prefix
 *  sets the whole base itself. */
export function crmApiBase(raw: string | undefined): string {
  const value = (raw ?? "").trim();
  return value === "" ? DEFAULT_CRM_API_URL : value.replace(/\/+$/, "");
}

/** The upstream URL for a proxied request. */
export function upstreamUrl(
  base: string,
  path: string,
  search: string,
): string {
  return `${base}/${path}${search}`;
}
