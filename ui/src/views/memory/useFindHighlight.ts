import { useEffect, type RefObject } from "react";

const NAME = "memory-find";
/** At most this many matches are marked in one document. */
const MAX_MARKS = 2000;

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
    // Bounded however the document is shaped: one huge text node cannot add more marks.
    const marks = new Highlight();
    let first: Range | null = null;
    let count = 0;
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
    for (let n = walker.nextNode(); n && count < MAX_MARKS; n = walker.nextNode()) {
      const text = (n.nodeValue ?? "").toLowerCase();
      for (let at = text.indexOf(q); at >= 0 && count < MAX_MARKS; at = text.indexOf(q, at + q.length)) {
        const r = document.createRange();
        r.setStart(n, at);
        r.setEnd(n, at + q.length);
        marks.add(r);
        first ??= r;
        count += 1;
      }
    }
    registry.set(NAME, marks);
    const firstEl = first?.startContainer.parentElement;
    firstEl?.scrollIntoView({ block: "center", behavior: "auto" });
    return () => {
      registry.delete(NAME);
    };
  }, [root, query, deps]);
}
