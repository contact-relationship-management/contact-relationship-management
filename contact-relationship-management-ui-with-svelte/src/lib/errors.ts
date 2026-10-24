/**
 * The text a page shows for a failed call. An `Error` (including the
 * `ApiError` the client throws) speaks for itself; anything else is
 * shown as `fallback` when given, else as its string form.
 */
export function errorMessage(cause: unknown, fallback?: string): string {
  if (cause instanceof Error) return cause.message;
  return fallback ?? String(cause);
}
