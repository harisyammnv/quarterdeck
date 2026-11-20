import { useMemo, useState, type MouseEvent } from "react";
import { ConfirmLink } from "./ConfirmLink";
import { renderMarkdown } from "./sanitize";

/**
 * Renders untrusted Markdown (reports, memory, skill bodies). This is the one place in the
 * app that sets HTML: the source goes through `renderMarkdown` (marked, raw HTML dropped,
 * DOMPurify). A click on a link never navigates; it opens the confirm step instead.
 */
export function Markdown({ source, className }: { source: string; className?: string }) {
  const html = useMemo(() => renderMarkdown(source), [source]);
  const [link, setLink] = useState<string | null>(null);

  const onClick = (e: MouseEvent<HTMLDivElement>) => {
    const a = (e.target as HTMLElement).closest("a");
    if (!a) return;
    e.preventDefault();
    const href = a.getAttribute("href") ?? "";
    if (href.startsWith("#")) {
      document.getElementById(href.slice(1))?.scrollIntoView({ block: "start" });
      return;
    }
    if (href) setLink(href);
  };

  return (
    <>
      {/* The HTML is sanitized by renderMarkdown; see ui/src/ui/sanitize.ts. */}
      <div className={`doc${className ? ` ${className}` : ""}`} onClick={onClick} dangerouslySetInnerHTML={{ __html: html }} />
      {link && (
        <ConfirmLink
          url={link}
          onClose={() => {
            setLink(null);
          }}
        />
      )}
    </>
  );
}
