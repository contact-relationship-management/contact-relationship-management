// Server-only: the readiness list of a path-to-production page (locales/<locale>/path-to-production/index.md).

/** One readiness estimate: `- **Name** (NN%): evidence`, the evidence rendered as HTML. */
export type Readiness = { name: string; percent: number; html: string };

const ITEM = /^\*\*([^*]+)\*\* \((\d{1,3})%\): ([\s\S]+)$/;

/**
 * Split the document at its readiness section: the first `##` section whose first list item has the form
 * `- **Name** (NN%): text` (the heading is in the page's own language). The text before and after renders as usual;
 * each list item in the section becomes a readiness estimate. A list item in another shape fails the build rather
 * than going missing.
 */
export function readiness(path: string, text: string): { before: string; items: { name: string; percent: number; text: string }[]; after: string } {
  const heads = [...text.matchAll(/^## .*$/gm)].map((m) => m.index ?? 0);
  const ranges = heads.map((start, i) => [start, heads[i + 1] ?? text.length] as const);
  const found = ranges.find(([a, b]) => {
    const first = /^- (.*)$/m.exec(text.slice(a, b));
    return first !== null && /^\*\*[^*]+\*\* \(\d{1,3}%\): /.test(first[1] ?? "");
  });
  if (!found) throw new Error(`${path}: no section whose list items read "- **Name** (NN%): text"`);
  const [start, end] = found;
  const section = text.slice(start, end);
  const after = text.slice(end);
  const listStart = section.search(/^- /m);
  if (listStart === -1) throw new Error(`${path}: the Readiness section has no list`);
  const intro = section.slice(0, listStart);
  const items = section
    .slice(listStart)
    .split(/^- /m)
    .filter((chunk) => chunk.trim() !== "")
    .map((chunk) => {
      const joined = chunk.replace(/\n\s+/g, " ").trim();
      const m = ITEM.exec(joined);
      if (!m || !m[1] || !m[2] || !m[3]) throw new Error(`${path}: readiness item not in the form "- **Name** (NN%): text": ${joined.slice(0, 80)}`);
      const percent = Number(m[2]);
      if (percent > 100) throw new Error(`${path}: readiness over 100%: ${m[1]}`);
      return { name: m[1], percent, text: m[3] };
    });
  return { before: text.slice(0, start) + intro, items, after };
}
