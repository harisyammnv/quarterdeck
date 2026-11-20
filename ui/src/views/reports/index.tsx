import {
  CircleDashedIcon as CircleDashed,
  FileTextIcon as FileText,
  WarningCircleIcon as WarningCircle,
} from "@phosphor-icons/react";
import { Banner, Markdown, ZeroState } from "../../ui";
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
        <ZeroState icon={CircleDashed} title="Reading the report">
          Opening the task's report.
        </ZeroState>
      ) : load.kind === "error" ? (
        <ZeroState icon={WarningCircle} title="Report not available">
          {load.message}
        </ZeroState>
      ) : load.value === null ? (
        <ZeroState icon={FileText} title="No report yet">
          The task writes its report when it finishes; it appears here.
        </ZeroState>
      ) : (
        <ReportBody doc={load.value} />
      )}
    </div>
  );
}

function ReportBody({ doc }: { doc: ReportDoc }) {
  return (
    <>
      <div className="reports-meta qd-muted tn">
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
