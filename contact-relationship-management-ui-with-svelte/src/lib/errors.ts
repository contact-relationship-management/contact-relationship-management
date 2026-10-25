/**
 * The text a page shows for a failed call. An `Error` (including the
 * `ApiError` the client throws) speaks for itself; anything else is
 * shown as `fallback` when given, else as its string form.
 */
export function errorMessage(cause: unknown, fallback?: string): string {
  if (cause instanceof Error) return cause.message;
  return fallback ?? String(cause);
}

/**
 * The problems a failed call names by member, as `{ member: detail }`. The service's validation
 * `422` carries `errors: [{ pointer, detail }]` (RFC 9457); a pointer such as `/display_name`
 * names the member `display_name`. A problem about the request as a whole has no pointer and is
 * left out (the page still shows the message). Anything that is not such an error gives `{}`.
 */
export function fieldErrors(cause: unknown): Record<string, string> {
  const body = (cause as { body?: unknown } | null)?.body;
  const list = (body as { errors?: unknown } | null)?.errors;
  if (!Array.isArray(list)) return {};
  const out: Record<string, string> = {};
  for (const item of list) {
    const { pointer, detail } = (item ?? {}) as {
      pointer?: unknown;
      detail?: unknown;
    };
    if (typeof pointer !== "string" || typeof detail !== "string") continue;
    const member = pointer.replace(/^\//, "");
    if (member && !(member in out)) out[member] = detail;
  }
  return out;
}
