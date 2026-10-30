// Site-wide search: every word of the query must appear somewhere in the
// record's fields, in any order, ignoring case and accents. The lists are
// filtered in the browser from the same reads the screens use, so masking and
// record-level access apply exactly as they do there.

// Every script (CRM-T170): a mark is dropped only from a Latin, Greek or Cyrillic letter (as the
// workforce planning UI's name fold, WPM-T381), so a Devanagari vowel sign or a Thai tone mark,
// which change the word, stay. Matching is by substring, so a part of a name written without
// spaces (`芳` in `王芳`) is found here as the service finds it by character pairs.

/** Lower-case and strip accents so "Zoë" matches "zoe"; other scripts' marks are kept. */
export function fold(text: string): string {
  return text
    .normalize("NFD")
    .replace(
      /([\p{Script=Latin}\p{Script=Greek}\p{Script=Cyrillic}])\p{M}+/gu,
      "$1",
    )
    .normalize("NFC")
    .toLowerCase();
}

/** An ideograph or kana, one of which can be a whole given name, as the service reads them. */
const IDEOGRAPH =
  /^[\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff\u{20000}-\u{323af}]$/u;

/** Whether a query is long enough to search, as the service decides: two characters, or one ideograph. */
export function longEnough(query: string): boolean {
  const chars = [...query.trim()];
  return chars.length >= 2 || (chars.length === 1 && IDEOGRAPH.test(chars[0]!));
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
