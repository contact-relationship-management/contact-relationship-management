// Server-only (prerender time): the repository documents and their Markdown, rendered to HTML.
import { Marked, type Tokens } from "marked";
import docs from "../../content/docs.json" with { type: "json" };
import { REPO_URL, routeOfRepoPath } from "#lib/site.js";

const FILES: Record<string, string> = docs.files;

export function source(path: string): string {
  const text = FILES[path];
  if (text === undefined) throw new Error(`not in src/content/docs.json: ${path} (run pnpm run sync)`);
  return text;
}

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

/** GitHub's heading anchor, as `scripts/check-links.py` in the monorepo computes it. */
export function slug(text: string): string {
  let h = text.trim().toLowerCase();
  h = h.replace(/`|\*|_(?=\W|$)/g, "");
  h = h.replace(/\[([^\]]*)\]\([^)]*\)/g, "$1");
  h = h.replace(/[^\p{L}\p{N}_\- ]/gu, "");
  return h.replace(/ /g, "-");
}

/** Join a relative link onto the directory of `from` (a repository path); null if it climbs out of the repository. */
export function joinRepoPath(from: string, href: string): string | null {
  const parts = href.startsWith("/") ? [] : from.split("/").slice(0, -1);
  for (const seg of href.split("/")) {
    if (seg === "" || seg === ".") continue;
    if (seg === "..") {
      if (parts.length === 0) return null;
      parts.pop();
    } else parts.push(seg);
  }
  return parts.join("/");
}

/**
 * Where a link in the document at `from` should go on the site. A link to another document the site shows goes to
 * its page; any other repository path goes to the file on GitHub; external links and in-page anchors are kept.
 * `route` turns a site route ("/install/") into an href that honours the base path.
 */
export function rewriteHref(from: string, href: string, route: (r: string) => string): string {
  if (href.startsWith("#") || /^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(href) || href.startsWith("//")) return href;
  const [rawPath = "", frag] = href.split("#", 2);
  const target = rawPath === "" ? from : joinRepoPath(from, decodeURIComponent(rawPath));
  const hash = frag ? `#${frag}` : "";
  if (target === null) return `${REPO_URL}${hash}`;
  const page = routeOfRepoPath(target);
  if (page) return route(page) + hash;
  const kind = /\.[^/]+$/.test(target) ? "blob" : "tree";
  return `${REPO_URL}/${kind}/main/${target.split("/").map(encodeURIComponent).join("/")}${hash}`;
}

export type Rendered = { title: string; html: string };

/** Render one repository document. Raw HTML in the Markdown is shown as text, never passed through. */
export function render(path: string, route: (r: string) => string): Rendered {
  let text = source(path);
  // YAML front matter, if a document has any, is metadata, not page text.
  text = text.replace(/^---\n[\s\S]*?\n---\n/, "");
  const seen = new Map<string, number>();
  let title = "";
  const md = new Marked({
    gfm: true,
    walkTokens(token) {
      if (token.type === "link") (token as Tokens.Link).href = rewriteHref(path, (token as Tokens.Link).href, route);
    },
    renderer: {
      html(token) {
        return escapeHtml(token.text);
      },
      heading(token) {
        const inner = this.parser.parseInline(token.tokens);
        const base = slug(token.text);
        const n = seen.get(base) ?? 0;
        seen.set(base, n + 1);
        const id = n === 0 ? base : `${base}-${n}`;
        if (token.depth === 1 && !title) title = token.text.replace(/[`*_]/g, "");
        return `<h${token.depth} id="${escapeHtml(id)}">${inner}</h${token.depth}>\n`;
      },
      code(token) {
        // tabindex: a block that scrolls sideways must be reachable from the keyboard (axe scrollable-region-focusable).
        const lang = token.lang ? ` class="language-${escapeHtml(token.lang.split(/\s/)[0] ?? "")}"` : "";
        return `<pre tabindex="0"><code${lang}>${escapeHtml(token.text)}</code></pre>\n`;
      },
      table(token) {
        // A wide table scrolls inside its own region instead of widening the page (phones, long paths).
        const head = token.header.map((c) => `<th>${this.parser.parseInline(c.tokens)}</th>`).join("");
        const body = token.rows
          .map((row) => `<tr>${row.map((c) => `<td>${this.parser.parseInline(c.tokens)}</td>`).join("")}</tr>`)
          .join("\n");
        return `<div class="table-scroll" tabindex="0"><table><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table></div>\n`;
      },
    },
  });
  const html = md.parse(text, { async: false });
  return { title: title || path, html };
}

/** A brief's title (its H1) and summary (the quoted note under it, Markdown marks removed), for the list of briefs. */
export function briefSummary(path: string): { title: string; summary: string } {
  const text = source(path);
  const title = /^# (.+)$/m.exec(text)?.[1]?.trim() ?? path;
  const quote = /^(?:> ?.*\n?)+/m.exec(text.slice(text.indexOf("\n") + 1))?.[0] ?? "";
  const summary = quote
    .split("\n")
    .map((l) => l.replace(/^> ?/, "").trim())
    .join(" ")
    .replace(/[*_`]/g, "")
    .trim();
  return { title, summary };
}
