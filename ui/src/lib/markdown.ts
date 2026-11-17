import MarkdownIt from "markdown-it";

/**
 * Forge content (PR bodies, reviews, comments) is hostile (PLAN §5.6). With
 * `html: false`, raw HTML in the source is escaped and shown as text, and
 * markdown-it's own link check drops `javascript:`, `vbscript:`, `file:` and
 * non-image `data:` URLs. The CSP is the second line: no inline script runs
 * even if something slipped through.
 */
const md = new MarkdownIt({ html: false, linkify: true });

/** `source` as HTML that is safe to put in the page with `v-html`. */
export function renderMarkdown(source: string): string {
  return md.render(source);
}
