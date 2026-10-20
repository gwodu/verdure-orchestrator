import { describe, expect, it } from "vitest";
import {
  escapeHtml,
  fmtBytes,
  fmtSize,
  friendlyError,
  looksInterrupted,
  parseProgress,
  renderMarkdown,
} from "./format";

// renderMarkdown is the safety boundary for model output: a local model's reply
// is untrusted input, and it goes into innerHTML. These come first.
describe("renderMarkdown never lets markup through", () => {
  const attacks = [
    "<script>alert(1)</script>",
    '<img src=x onerror="alert(1)">',
    "**<b onmouseover=alert(1)>bold</b>**",
    "`<iframe src=javascript:alert(1)>`",
    "- <a href=\"javascript:alert(1)\">list</a>",
    "# <svg onload=alert(1)>",
    "_<script>x</script>_",
  ];
  for (const a of attacks) {
    it(`escapes ${JSON.stringify(a)}`, () => {
      const html = renderMarkdown(a);
      // No tag the model wrote survives as a tag: every < it emitted is escaped.
      expect(html).not.toMatch(/<(script|img|iframe|svg|a|b)[\s>]/i);
      expect(html).toContain("&lt;");
    });
  }

  it("only ever emits its own fixed tag set", () => {
    const html = renderMarkdown("# Title\n\n**bold** and *em* and `code`\n- one\n- two\n1. first\n2. second\nplain <i>x</i>");
    const tags = [...html.matchAll(/<\/?([a-z]+)/g)].map((m) => m[1]);
    for (const t of tags) expect(["p", "ul", "ol", "li", "strong", "em", "code"]).toContain(t);
  });
});

describe("renderMarkdown formatting", () => {
  it("renders bold, emphasis and code", () => {
    expect(renderMarkdown("a **b** c")).toBe("<p>a <strong>b</strong> c</p>");
    expect(renderMarkdown("an *idea* here")).toBe("<p>an <em>idea</em> here</p>");
    expect(renderMarkdown("run `ls -la` now")).toBe("<p>run <code>ls -la</code> now</p>");
  });

  it("does not bold inside a code span", () => {
    expect(renderMarkdown("`**x**`")).toBe("<p><code>**x**</code></p>");
  });

  it("keeps asterisks literal inside code, and still formats outside it", () => {
    expect(renderMarkdown("use `a*b*c` then **go**")).toBe("<p>use <code>a*b*c</code> then <strong>go</strong></p>");
  });

  it("groups bullets and numbers into lists and closes them", () => {
    expect(renderMarkdown("- a\n- b\ntext")).toBe("<ul><li>a</li><li>b</li></ul><p>text</p>");
    expect(renderMarkdown("1. a\n2) b")).toBe("<ol><li>a</li><li>b</li></ol>");
    expect(renderMarkdown("- a\n1. b")).toBe("<ul><li>a</li></ul><ol><li>b</li></ol>");
  });

  it("turns headings into bold lines and skips blank lines", () => {
    expect(renderMarkdown("## Plan\n\nGo")).toBe("<strong>Plan</strong><p>Go</p>");
  });

  it("leaves a lone asterisk alone (multiplication, not emphasis)", () => {
    expect(renderMarkdown("2 * 3 = 6")).toBe("<p>2 * 3 = 6</p>");
  });
});

describe("escapeHtml", () => {
  it("escapes all five HTML-significant characters", () => {
    expect(escapeHtml(`<a href="x">'&'</a>`)).toBe("&lt;a href=&quot;x&quot;&gt;&#39;&amp;&#39;&lt;/a&gt;");
  });
});

describe("fmtSize / fmtBytes — one size style everywhere", () => {
  it("uses MB below 1 GB and GB to one decimal above", () => {
    expect(fmtSize(469)).toBe("469 MB");
    expect(fmtSize(1023)).toBe("1023 MB");
    expect(fmtSize(1024)).toBe("1.0 GB");
    expect(fmtSize(1336)).toBe("1.3 GB");
    expect(fmtSize(8434)).toBe("8.2 GB");
  });

  it("never shows a negative or fractional MB", () => {
    expect(fmtSize(-5)).toBe("0 MB");
    expect(fmtSize(12.6)).toBe("13 MB");
  });

  it("formats bytes and never says 0 MB for a real file", () => {
    expect(fmtBytes(491_400_032)).toBe("469 MB");
    expect(fmtBytes(10)).toBe("1 MB");
  });
});

describe("friendlyError", () => {
  it("passes a backend sentence through", () => {
    expect(friendlyError("couldn't load the chat model")).toBe("couldn't load the chat model");
    expect(friendlyError(new Error("disk full"))).toBe("disk full");
  });

  it("never shows a blank or [object Object]", () => {
    for (const bad of ["", "   ", {}, null, undefined, 42]) {
      expect(friendlyError(bad)).toBe("Something went wrong. Please try again.");
    }
  });
});

describe("parseProgress", () => {
  it("reads the backend's progress line", () => {
    expect(parseProgress("PROGRESS 1024 2048")).toEqual({ done: 1024, total: 2048 });
  });

  it("ignores anything else, so it can be shown as a status message", () => {
    expect(parseProgress("Thinking…")).toBeNull();
    expect(parseProgress("PROGRESS 10")).toBeNull();
    expect(parseProgress("PROGRESS 10 20 extra")).toBeNull();
  });
});

describe("looksInterrupted", () => {
  it("recognises a dropped download", () => {
    for (const m of ["download interrupted", "connection reset", "os error 54", "timed out", "Network is down", "request aborted"]) {
      expect(looksInterrupted(m)).toBe(true);
    }
  });

  it("does not offer a resume for unrelated failures", () => {
    expect(looksInterrupted("not enough free space")).toBe(false);
  });
});
