import { useEffect, type RefObject } from "react";

const NAME = "memory-find";

/**
 * Marks every case-insensitive occurrence of `query` inside `root` with the CSS Custom
 * Highlight API and scrolls the first into view. Nothing in the rendered document changes;
 * where the API is missing (older WebKit) the document is simply shown unmarked.
 */
export function useFindHighlight(root: RefObject<HTMLElement | null>, query: string, deps: unknown): void {
  useEffect(() => {
    const registry = typeof CSS !== "undefined" && "highlights" in CSS ? CSS.highlights : null;
    const el = root.current;
    const q = query.trim().toLowerCase();
    if (!registry || typeof Highlight === "undefined" || !el || q.length < 2) {
      registry?.delete(NAME);
      return;
    }
    const ranges: Range[] = [];
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
    for (let n = walker.nextNode(); n && ranges.length < 2000; n = walker.nextNode()) {
      const text = (n.nodeValue ?? "").toLowerCase();
      for (let at = text.indexOf(q); at >= 0; at = text.indexOf(q, at + q.length)) {
        const r = document.createRange();
        r.setStart(n, at);
        r.setEnd(n, at + q.length);
        ranges.push(r);
      }
    }
    registry.set(NAME, new Highlight(...ranges));
    const first = ranges[0]?.startContainer.parentElement;
    first?.scrollIntoView({ block: "center", behavior: "auto" });
    return () => {
      registry.delete(NAME);
    };
  }, [root, query, deps]);
}
