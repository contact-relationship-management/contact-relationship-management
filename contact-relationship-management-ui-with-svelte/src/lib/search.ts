// Site-wide search: every word of the query must appear somewhere in the
// record's fields, in any order, ignoring case and accents. The lists are
// filtered in the browser from the same reads the screens use, so masking and
// record-level access apply exactly as they do there.

/** Lower-case and strip accents so "Zoë" matches "zoe". */
export function fold(text: string): string {
  return text.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
}

/** The query's words, folded; empty for a blank query. */
export function words(query: string): string[] {
  return fold(query).split(/\s+/).filter(Boolean);
}

/** True when every word of `query` appears in at least one of `fields`. */
export function matches(
  fields: readonly (string | number | null | undefined)[],
  query: string,
): boolean {
  const wanted = words(query);
  if (wanted.length === 0) return false;
  const haystack = fold(
    fields.filter((f) => f !== null && f !== undefined).join(" "),
  );
  return wanted.every((word) => haystack.includes(word));
}
