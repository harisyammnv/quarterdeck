import { useRef } from "react";
import { Markdown } from "../../components";
import { useFindHighlight } from "./useFindHighlight";

/**
 * A document body: untrusted Markdown through the shared sanitizing renderer, with matches
 * of the current find marked. Used for memory documents and skill bodies.
 */
export function DocBody({ text, truncated, find }: { text: string; truncated: boolean; find: string }) {
  const ref = useRef<HTMLDivElement>(null);
  useFindHighlight(ref, find, text);
  return (
    <div className="memory-body" ref={ref}>
      <div className="memory-doc">
        <Markdown source={text} />
      </div>
      {truncated && <p className="memory-trunc">This document is longer than 1 MiB; only its start is shown.</p>}
    </div>
  );
}
