// The site's search. A static site has no server to ask, so every page's plain text is prerendered into
// /search-index.json and this runs in the reader's browser. Safe for the browser: no server-only import.

/** One searchable page. `locale` is null on a document, which is English. */
export type Entry = { route: string; title: string; locale: string | null; text: string };

/** One result: the page and a short passage around the first match. */
export type Hit = { entry: Entry; snippet: string };

const SNIPPET_BEFORE = 60;
const SNIPPET_AFTER = 140;

function count(haystack: string, term: string): number {
  let n = 0;
  for (let at = haystack.indexOf(term); at !== -1; at = haystack.indexOf(term, at + term.length)) n += 1;
  return n;
}

function snippet(text: string, lower: string, term: string): string {
  const at = lower.indexOf(term);
  if (at === -1) return text.slice(0, SNIPPET_BEFORE + SNIPPET_AFTER);
  const start = Math.max(0, at - SNIPPET_BEFORE);
  const end = Math.min(text.length, at + SNIPPET_AFTER);
  return `${start > 0 ? "…" : ""}${text.slice(start, end)}${end < text.length ? "…" : ""}`;
}

/** The pages that hold every word of `query`, those with a word in the title first, then by how often they occur. */
export function search(entries: readonly Entry[], query: string): Hit[] {
  const terms = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (terms.length === 0) return [];
  const scored: { hit: Hit; score: number }[] = [];
  for (const entry of entries) {
    const title = entry.title.toLowerCase();
    const text = entry.text.toLowerCase();
    if (!terms.every((term) => title.includes(term) || text.includes(term))) continue;
    const inTitle = terms.filter((term) => title.includes(term)).length;
    const score = inTitle * 1000 + terms.reduce((sum, term) => sum + count(text, term), 0);
    scored.push({ hit: { entry, snippet: snippet(entry.text, text, terms[0]!) }, score });
  }
  return scored.sort((a, b) => b.score - a.score).map((s) => s.hit);
}
