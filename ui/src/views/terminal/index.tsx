import {
  CursorClickIcon as CursorClick,
  FileTextIcon as FileText,
  TerminalWindowIcon as TerminalWindow,
} from "@phosphor-icons/react";
import { inApp } from "../../bridge";
import { Composer, Panel, ProjectTile, StatePill, Tabs, Zero, ago, useNow, type Segment } from "../../ui";
import { useApp } from "../../state/app";
import { ReportView } from "../reports";
import { TerminalTailView } from "./Tail";
import { setTaskTab, useTaskTab, type TaskTab } from "./open";
import "./terminal.css";

const TABS: Segment<TaskTab>[] = [
  { id: "terminal", label: "Terminal", icon: TerminalWindow },
  { id: "report", label: "Report", icon: FileText },
];

/**
 * One task up close, read-only: the tail of its terminal or its report, with a note box for
 * Firstmate under the terminal. Both panes read again when the task reports a status event.
 */
export default function TaskView() {
  const { live, selectedTask, info } = useApp();
  const tab = useTaskTab();
  const now = useNow(30_000);
  const row = live.view?.rows.find((r) => r.id === selectedTask) ?? null;

  if (!row) {
    return (
      <Panel as="main" className="fill task" label="Task">
        <div className="task-zero">
          <Zero icon={CursorClick} title="No task in focus" dashed={false}>
            Pick a task in the fleet to read its terminal and report here.
          </Zero>
        </div>
      </Panel>
    );
  }

  const ev = row.latest_event;
  // Changes whenever the task reports, which is when both panes read again.
  const eventKey = ev ? `${String(ev.at)}:${ev.word}:${ev.text}` : null;
  const project = row.project ?? "unknown project";

  return (
    <Panel as="main" className="fill task" label={`Task ${row.id}`}>
      <div className="phead nob">
        <ProjectTile name={project} />
        <span className="crumb trunc">{project}</span>
        <span className="crumb" aria-hidden="true">
          /
        </span>
        <h1 className="ptitle trunc" title={row.name}>
          {row.id}
        </h1>
        <StatePill state={row.state} />
        <span className="grow" />
        {ev?.at && <span className="mut tn task-when">Last event {ago(ev.at * 1000, now)}</span>}
      </div>
      <div className="task-tabs">
        <Tabs items={TABS} value={tab} onChange={setTaskTab} label="Task panes" />
      </div>
      <div className="task-body">
        {tab === "terminal" ? (
          <TerminalTailView key={row.id} taskId={row.id} eventKey={eventKey} />
        ) : (
          <ReportView key={row.id} taskId={row.id} eventKey={eventKey} />
        )}
      </div>
      {tab === "terminal" && (
        <Composer
          key={row.id}
          enabled={inApp && !info?.home_error}
          about={row.id}
          placeholder="Leave a note for Firstmate about this task"
        />
      )}
    </Panel>
  );
}
