import { useDeferredValue, useRef } from "react";
import { Markdown } from "../../ui";
import { useFindHighlight } from "./useFindHighlight";

/**
 * A document body: untrusted Markdown through the shared sanitizing renderer, with matches
 * of the current find marked. Used for memory documents and skill bodies.
 */
export function DocBody({ text, truncated, find }: { text: string; truncated: boolean; find: string }) {
  const ref = useRef<HTMLDivElement>(null);
  // Marking waits for typing to settle rather than running on every keystroke.
  useFindHighlight(ref, useDeferredValue(find), text);
  return (
    <div className="memory-body" ref={ref}>
      <Markdown source={text} className="memory-doc" />
      {truncated && <p className="memory-trunc">This document is longer than 1 MiB; only its start is shown.</p>}
    </div>
  );
}
