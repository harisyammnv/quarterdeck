import {
  CircleDashedIcon as CircleDashed,
  FileTextIcon as FileText,
  WarningCircleIcon as WarningCircle,
} from "@phosphor-icons/react";
import { Banner, Markdown, Zero } from "../../ui";
import { reportRead, type ReportDoc } from "./bridge";
import { useLoad } from "./load";
import "./reports.css";

export interface ReportViewProps {
  taskId: string;
  /** Changes whenever the task's latest status event changes; each change reads again. */
  eventKey?: string | null;
}

/**
 * A task's `report.md`, rendered through the shared sanitizing Markdown component. Mount it
 * with `key={taskId}`, so another task starts from the loading state.
 */
export function ReportView({ taskId, eventKey = null }: ReportViewProps) {
  const load = useLoad(reportRead, taskId, eventKey);
  return (
    <div className="reports">
      {load.kind === "loading" ? (
        <Zero icon={CircleDashed} title="Reading the report" dashed={false}>
          Opening the task's report.
        </Zero>
      ) : load.kind === "error" ? (
        <Zero icon={WarningCircle} title="Report not available" dashed={false}>
          {load.message}
        </Zero>
      ) : load.value === null ? (
        <Zero icon={FileText} title="No report yet" dashed={false}>
          The task writes its report when it finishes; it appears here.
        </Zero>
      ) : (
        <ReportBody doc={load.value} />
      )}
    </div>
  );
}

function ReportBody({ doc }: { doc: ReportDoc }) {
  return (
    <>
      <div className="reports-meta mut tn">
        <FileText size={14} />
        <span>report.md</span>
        {doc.modified !== null && (
          <>
            <span aria-hidden="true">·</span>
            <span>Updated {stamp(doc.modified)}</span>
          </>
        )}
      </div>
      {doc.truncated && <Banner>The report is longer than 1 MB; only its start is shown.</Banner>}
      <article className="reports-doc" tabIndex={0} aria-label="Task report">
        <Markdown source={doc.markdown} />
      </article>
    </>
  );
}

function stamp(unixSeconds: number): string {
  const d = new Date(unixSeconds * 1000);
  const time = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false });
  return new Date().toDateString() === d.toDateString() ? time : `${d.toLocaleDateString([], { day: "numeric", month: "short" })} ${time}`;
}
