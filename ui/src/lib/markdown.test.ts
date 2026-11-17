import { describe, expect, it } from "vitest";
import { renderMarkdown } from "./markdown";

/** `html` parsed into a detached element, as the browser would build it. */
function parse(html: string): HTMLElement {
  const root = document.createElement("div");
  root.innerHTML = html;
  return root;
}

describe("renderMarkdown", () => {
  it("renders the Markdown a PR description uses", () => {
    const root = parse(renderMarkdown("## Why\n\nSay hello **properly**.\n\n- one\n- two\n\n`code`"));

    expect(root.querySelector("h2")?.textContent).toBe("Why");
    expect(root.querySelector("strong")?.textContent).toBe("properly");
    expect(root.querySelectorAll("li")).toHaveLength(2);
    expect(root.querySelector("code")?.textContent).toBe("code");
  });

  it("shows raw HTML as text, so a script never becomes an element", () => {
    const root = parse(
      renderMarkdown('<script>alert(1)</script>\n\n<img src=x onerror="alert(2)">\n\n<iframe src="https://x.test"></iframe>'),
    );

    expect(root.querySelector("script, img, iframe, [onerror]")).toBeNull();
    expect(root.textContent).toContain("<script>alert(1)</script>");
  });

  it("leaves out links that would run code", () => {
    const root = parse(renderMarkdown("[a](javascript:alert(1)) [b](vbscript:x) [c](data:text/html,<b>x</b>)"));

    expect(root.querySelector("a")).toBeNull();
  });

  it("keeps https links and images, and links bare URLs", () => {
    const root = parse(renderMarkdown("[docs](https://docs.test/a) ![logo](https://img.test/l.png) https://bare.test"));

    const hrefs = [...root.querySelectorAll("a")].map((a) => a.getAttribute("href"));
    expect(hrefs).toEqual(["https://docs.test/a", "https://bare.test"]);
    expect(root.querySelector("img")?.getAttribute("src")).toBe("https://img.test/l.png");
  });
});
