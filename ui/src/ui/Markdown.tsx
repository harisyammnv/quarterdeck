import { useMemo, useState, type MouseEvent } from "react";
import { WarningCircleIcon as WarningCircle } from "@phosphor-icons/react";
import { urlOpen } from "../bridge";
import { renderMarkdown } from "./sanitize";

/**
 * Renders untrusted Markdown (reports, memory, skill bodies). This is the one place in the
 * app that sets HTML: the source goes through `renderMarkdown` (marked, raw HTML dropped,
 * DOMPurify). A click on a link never navigates: it asks the core, which shows the full
 * address in a native confirmation and opens only https links the person approves.
 */
export function Markdown({ source, className }: { source: string; className?: string }) {
  const html = useMemo(() => renderMarkdown(source), [source]);
  const [error, setError] = useState<string | null>(null);

  const onClick = (e: MouseEvent<HTMLDivElement>) => {
    const a = (e.target as HTMLElement).closest("a");
    if (!a) return;
    e.preventDefault();
    const href = a.getAttribute("href") ?? "";
    if (href.startsWith("#")) {
      document.getElementById(href.slice(1))?.scrollIntoView({ block: "start" });
      return;
    }
    if (!href) return;
    setError(null);
    urlOpen(href).catch((err: unknown) => {
      setError(err instanceof Error ? err.message : String(err));
    });
  };

  return (
    <>
      {error && (
        <p className="ui-doc-err" role="alert">
          <WarningCircle size={14} /> {error}
        </p>
      )}
      {/* The HTML is sanitized by renderMarkdown; see ui/src/ui/sanitize.ts. */}
      <div className={`doc${className ? ` ${className}` : ""}`} onClick={onClick} dangerouslySetInnerHTML={{ __html: html }} />
    </>
  );
}
