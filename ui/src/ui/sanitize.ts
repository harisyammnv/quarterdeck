import DOMPurify from "dompurify";
import { Marked } from "marked";

/**
 * Untrusted Markdown to safe HTML: raw HTML in the source is dropped (not escaped into view),
 * then DOMPurify keeps only document tags, drops every image that is not a `data:` URL, and
 * strips event handlers, styles and scripts. Links keep their address for the confirm step;
 * nothing navigates by itself.
 */
const marked = new Marked({
  gfm: true,
  async: false,
  renderer: {
    html: () => "",
  },
});

const ALLOWED_TAGS = [
  "a", "b", "blockquote", "br", "code", "del", "em", "h1", "h2", "h3", "h4", "h5", "h6", "hr", "i",
  "img", "input", "li", "ol", "p", "pre", "s", "strong", "sub", "sup", "table", "tbody", "td", "th",
  "thead", "tr", "ul",
];
const ALLOWED_ATTR = ["href", "title", "alt", "src", "colspan", "rowspan", "align", "type", "checked", "disabled", "start"];

let hooked = false;
function hook(): void {
  if (hooked) return;
  hooked = true;
  DOMPurify.addHook("uponSanitizeElement", (node, data) => {
    if (data.tagName === "img" && node instanceof Element) {
      const src = node.getAttribute("src") ?? "";
      if (!/^data:image\/(png|gif|jpeg|webp);/i.test(src)) node.remove();
    }
    if (data.tagName === "input" && node instanceof Element && node.getAttribute("type") !== "checkbox") node.remove();
  });
  DOMPurify.addHook("afterSanitizeAttributes", (node) => {
    if (node.tagName === "A") {
      node.setAttribute("rel", "noreferrer noopener");
      node.removeAttribute("target");
    }
    if (node.tagName === "INPUT") node.setAttribute("disabled", "");
  });
}

export function renderMarkdown(source: string): string {
  hook();
  const html = marked.parse(source) as string;
  return DOMPurify.sanitize(html, {
    ALLOWED_TAGS,
    ALLOWED_ATTR,
    ALLOW_DATA_ATTR: false,
    ALLOWED_URI_REGEXP: /^(?:https?:|mailto:|#|data:image\/(?:png|gif|jpeg|webp);)/i,
  });
}
