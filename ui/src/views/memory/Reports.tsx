import { useEffect, useState, type ReactNode } from "react";
import { CircleDashedIcon as CircleDashed, FileTextIcon as FileText, WarningCircleIcon as WarningCircle } from "@phosphor-icons/react";
import { Banner, ProjectTile, VirtualList, ZeroState } from "../../ui";
import { reportRead, type ReportDoc, type ReportItem, type ReportsList } from "../reports/bridge";
import { DocBody } from "./DocBody";
import { plural, shortAge } from "./format";

/**
 * Reports in the memory view. The list and each report come from the reports area's
 * commands (`reports_list`, `report_read`); this file only lays them out.
 */

/** A report's display title: its task's title from the snapshot, else the task id. */
export function reportTitle(r: ReportItem): string {
  return r.title ?? r.task_id;
}

function Head({ title, meta, back }: { title: string; meta?: string; back?: ReactNode }) {
  return (
    <div className="phead memory-phead">
      {back ?? <FileText size={18} className="mut" aria-hidden="true" />}
      <h1 className="ptitle trunc">{title}</h1>
      {meta && <span className="mut t12 tn">{meta}</span>}
      <span className="grow" />
    </div>
  );
}

/** Every report the snapshot lists, newest first. */
export function ReportsPane({ list, now, onOpen }: { list: ReportsList; now: number; onOpen: (taskId: string) => void }) {
  return (
    <>
      <Head title="Reports" meta={plural(list.items.length, "report", "reports")} />
      {list.truncated && <Banner>Only the newest {list.items.length} reports are listed; older ones stay in the home.</Banner>}
      {list.items.length === 0 ? (
        <ZeroState icon={FileText} title="No reports yet">
          Investigation reports appear here once a task writes one.
        </ZeroState>
      ) : (
        <VirtualList
          className="memory-group"
          label="Reports"
          items={list.items}
          rowHeight={56}
          rowKey={(r) => r.task_id}
          render={(r) => (
            <button
              type="button"
              className="memory-li memory-group-row"
              onClick={() => {
                onOpen(r.task_id);
              }}
            >
              <ProjectTile name={r.task_id} size={20} />
              <span className="memory-li-text">
                <span className="t13 trunc">{reportTitle(r)}</span>
                <span className="t12 mut trunc">{r.title ? r.task_id : (r.kind ?? "Report")}</span>
              </span>
              <span className="t12 mut tn">{shortAge(r.modified, now, true)}</span>
            </button>
          )}
        />
      )}
    </>
  );
}

type Load = { id: string; doc: ReportDoc | null; missing: boolean; error: string | null };

/** One report, read on demand through `report_read`. */
export function ReportPane({ taskId, title, back, find }: { taskId: string; title: string; back: ReactNode; find: string }) {
  const [load, setLoad] = useState<Load>({ id: taskId, doc: null, missing: false, error: null });
  useEffect(() => {
    let live = true;
    reportRead(taskId)
      .then((doc) => {
        if (live) setLoad({ id: taskId, doc, missing: doc === null, error: null });
      })
      .catch((e: unknown) => {
        if (live) setLoad({ id: taskId, doc: null, missing: false, error: e instanceof Error ? e.message : String(e) });
      });
    return () => {
      live = false;
    };
  }, [taskId]);
  const cur = load.id === taskId ? load : null;
  return (
    <>
      <Head title={title} meta={taskId === title ? "Report" : taskId} back={back} />
      {cur?.error ? (
        <ZeroState icon={WarningCircle} title="This report could not be read">
          {cur.error}
        </ZeroState>
      ) : cur?.missing ? (
        <ZeroState icon={FileText} title="No report yet">
          This task has not written a report.
        </ZeroState>
      ) : !cur?.doc ? (
        <ZeroState icon={CircleDashed} title="Reading the report" />
      ) : (
        <DocBody text={cur.doc.markdown} truncated={cur.doc.truncated} find={find} />
      )}
    </>
  );
}
