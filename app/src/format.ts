// Pure text and number formatting for the UI: no DOM, no Tauri, no side effects
// on import. It lives apart from main.ts because main.ts boots the app the moment
// it is imported, which made all of this untestable (the same reason updates.ts
// was split out). Tested in format.test.ts.

/** A size in MB, in the one style every screen uses: MB below 1 GB, else GB to
 *  one decimal ("469 MB", "1.3 GB"). Five hand-written copies used to disagree,
 *  so one screen said "0.4 GB" where the next said "400 MB". */
export function fmtSize(mb: number): string {
  const whole = Math.max(0, Math.round(mb));
  return whole >= 1024 ? `${(whole / 1024).toFixed(1)} GB` : `${whole} MB`;
}

/** `fmtSize` for a byte count. Never shows "0 MB" for a file that exists. */
export function fmtBytes(bytes: number): string {
  return fmtSize(Math.max(1, bytes / 1_048_576));
}

/**
 * Backend errors arrive as human-written sentences (llm.rs/model.rs phrase them
 * for the user). Anything else — a thrown object, an empty string — would show
 * as "[object Object]" or a blank; show a calm fallback instead (the
 * truck-driver test).
 */
export function friendlyError(err: unknown): string {
  const msg = typeof err === "string" ? err : err instanceof Error ? err.message : "";
  return msg.trim() ? msg : "Something went wrong. Please try again.";
}

/** The backend's download progress line, "PROGRESS <done> <total>" in bytes. */
export function parseProgress(msg: string): { done: number; total: number } | null {
  const m = /^PROGRESS (\d+) (\d+)$/.exec(msg);
  return m ? { done: +m[1], total: +m[2] } : null;
}

/** Whether an error reads like an interrupted download (screen off, app left,
 *  network drop), so the UI can offer a one-tap resume instead of the raw error. */
export function looksInterrupted(msg: string): boolean {
  return /interrupt|abort|connection|os error|timed? ?out|network/i.test(msg);
}

export function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (ch) => {
    switch (ch) {
      case "&": return "&amp;";
      case "<": return "&lt;";
      case ">": return "&gt;";
      case '"': return "&quot;";
      default: return "&#39;";
    }
  });
}

/**
 * A minimal, SAFE Markdown renderer for model replies: the whole input is
 * HTML-escaped first, then only a fixed set of tags (p, ul/ol/li, strong, em,
 * code) is reintroduced. Nothing the model emits can inject markup — which
 * matters because a local model's output is still untrusted input.
 *
 * Deliberately not a Markdown library: this is ~40 lines against a dependency
 * the floor user would pay for in download size, for syntax we don't need
 * (tables, footnotes, HTML passthrough).
 */
export function renderMarkdown(src: string): string {
  const inline = (s: string): string =>
    escapeHtml(s)
      // Code first, so ** inside a code span isn't treated as bold.
      .replace(/`([^`]+)`/g, "<code>$1</code>")
      .replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>")
      .replace(/(^|[\s(])[*_]([^*_\s][^*_]*)[*_](?=[\s.,!?)]|$)/g, "$1<em>$2</em>");

  const out: string[] = [];
  let list: "ul" | "ol" | null = null;
  const closeList = () => {
    if (list) {
      out.push(`</${list}>`);
      list = null;
    }
  };

  for (const raw of src.split("\n")) {
    const line = raw.trimEnd();
    const bullet = /^\s*[-*+]\s+(.*)$/.exec(line);
    const numbered = /^\s*\d+[.)]\s+(.*)$/.exec(line);
    if (bullet || numbered) {
      const want = bullet ? "ul" : "ol";
      if (list !== want) {
        closeList();
        out.push(`<${want}>`);
        list = want;
      }
      out.push(`<li>${inline((bullet ?? numbered)![1])}</li>`);
      continue;
    }
    closeList();
    if (!line.trim()) continue; // blank line — the gap comes from CSS spacing
    const heading = /^\s*#{1,6}\s+(.*)$/.exec(line);
    out.push(heading ? `<strong>${inline(heading[1])}</strong>` : `<p>${inline(line)}</p>`);
  }
  closeList();
  return out.join("");
}
