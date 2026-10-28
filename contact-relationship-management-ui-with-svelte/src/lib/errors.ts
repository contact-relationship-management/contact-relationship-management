import { t } from "./i18n.svelte";

/**
 * A message a page wrote itself, already in the reader's language (for example "cards cannot move
 * between lanes here"). It is the one kind of `Error` whose text may be shown as it is.
 */
export class PageError extends Error {}

/** The status a failed call carried, when it carried one. */
function statusOf(cause: unknown): number | null {
  const status = (cause as { status?: unknown } | null)?.status;
  return typeof status === "number" ? status : null;
}

/**
 * The text a page shows for a failed call: **never the service's own words**. What the service
 * says in `description` or in a problem's `detail` is written for a developer, in English, and can
 * name a record, a rule or a person; it is kept out of the page. The reader gets a fixed,
 * translated message chosen by what kind of failure it was (the status), or `fallback` when the
 * page has a more specific message of its own. A [`PageError`] is shown as written.
 */
export function errorMessage(cause: unknown, fallback?: string): string {
  if (cause instanceof PageError) return cause.message;
  if (fallback !== undefined) return fallback;
  const status = statusOf(cause);
  if (status === null) {
    // A network failure is a `TypeError` from `fetch`; anything else is not worth describing.
    return cause instanceof TypeError
      ? t("error.unavailable")
      : t("error.generic");
  }
  if (status === 400 || status === 422) return t("error.invalid");
  if (status === 401) return t("error.unauthorized");
  if (status === 403) return t("error.forbidden");
  if (status === 404) return t("error.notFound");
  if (status === 409) return t("error.conflict");
  if (status === 429) return t("error.tooMany");
  if (status >= 500) return t("error.unavailable");
  return t("error.generic");
}

/**
 * The members a failed call names as not accepted, as `{ member: message }`. The service's
 * validation `422` carries `errors: [{ pointer, detail }]` (RFC 9457); a pointer such as
 * `/display_name` names the member `display_name`. **Only the member is taken**: the message is a
 * fixed translated one, never the service's `detail`. A problem about the request as a whole has
 * no pointer and is left out (the page still shows [`errorMessage`]). Anything that is not such an
 * error gives `{}`.
 */
export function fieldErrors(cause: unknown): Record<string, string> {
  const body = (cause as { body?: unknown } | null)?.body;
  const list = (body as { errors?: unknown } | null)?.errors;
  if (!Array.isArray(list)) return {};
  const out: Record<string, string> = {};
  for (const item of list) {
    const { pointer } = (item ?? {}) as { pointer?: unknown };
    if (typeof pointer !== "string") continue;
    const member = pointer.replace(/^\//, "");
    if (member && !(member in out)) out[member] = t("error.fieldInvalid");
  }
  return out;
}
