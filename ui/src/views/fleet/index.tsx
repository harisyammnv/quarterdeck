import { Suspense } from "react";
import { ArrowsClockwiseIcon as ArrowsClockwise, BinocularsIcon as Binoculars, CircleDashedIcon as CircleDashed, SailboatIcon as Sailboat } from "@phosphor-icons/react";
import { inApp, refreshFleet, type FleetRow, type FleetView } from "../../bridge";
import { Banner, Button, DataTable, Zero, Composer, Panel, PanelHead, ProjectTile, StatePill, Tag, ago, useNow, type Column } from "../../ui";
import { fleetTop as FleetTop } from "../../registry";
import { useApp } from "../../state/app";

function taskCount(n: number): string {
  return n === 1 ? "1 task" : `${n} tasks`;
}

function sourceTitle(row: FleetRow): string {
  return row.source === "event"
    ? "From the latest status event: newer than the snapshot, or the snapshot could not tell"
    : row.source === "bootstrap"
      ? "From the task's status record, before the first snapshot"
      : "From the fleet snapshot";
}

function snapshotText(view: FleetView, now: number): string {
  const s = view.snapshot;
  if (s.cached) return s.running ? "Saved snapshot, reading a fresh one" : "Saved snapshot";
  if (s.running) return "Reading snapshot";
  return s.generated_at ? `Snapshot ${ago(s.generated_at * 1000, now)}` : "No snapshot yet";
}

export default function FleetViewArea() {
  const { live, info, ipcError, selectedTask, setSelectedTask } = useApp();
  const { view, changed } = live;
  const now = useNow(1000);
  const rows = view?.rows ?? [];

  const columns: Column<FleetRow>[] = [
    {
      key: "task",
      header: "Task",
      width: "34%",
      cell: (r) => (
        <span className="row">
          <span className="task trunc" title={r.name}>
            {r.name}
          </span>
          <span className="tid mono trunc">{r.id}</span>
        </span>
      ),
    },
    {
      key: "project",
      header: "Project",
      width: "22%",
      cell: (r) => (
        <span className="row">
          <ProjectTile name={r.project ?? r.id} />
          <span className="trunc">{r.project ?? "unknown project"}</span>
        </span>
      ),
    },
    { key: "kind", header: "Kind", width: "92px", cell: (r) => <Tag icon={r.kind === "scout" ? Binoculars : r.kind === "ship" ? Sailboat : CircleDashed}>{r.kind ?? "task"}</Tag> },
    {
      key: "state",
      header: "State",
      width: "120px",
      cell: (r) => <StatePill state={r.state} word={r.state === "unknown" && r.state_word !== "unknown" ? r.state_word : undefined} title={sourceTitle(r)} />,
    },
    {
      key: "latest",
      header: "Latest event",
      cell: (r) => {
        const latest = r.latest_event?.text ?? r.doing ?? "";
        return (
          <span className="row">
            <span className="mut trunc grow" title={latest}>
              {latest}
            </span>
            {r.latest_event?.at && <span className="when tn">{ago(r.latest_event.at * 1000, now)}</span>}
          </span>
        );
      },
    },
  ];

  return (
    <>
      {FleetTop && (
        <Suspense fallback={null}>
          <FleetTop />
        </Suspense>
      )}
      <Panel as="main" className="fill fleet-panel" label="Fleet">
      <PanelHead title="Fleet" meta={view ? taskCount(rows.length) : undefined}>
        {view && <span className="meta tn">{snapshotText(view, now)}</span>}
        <Button size="sm" icon={ArrowsClockwise} disabled={!view || view.snapshot.running} onClick={() => void refreshFleet()}>
          Refresh
        </Button>
      </PanelHead>

      {ipcError && <Banner title={ipcError}>The window could not reach the core: {ipcError}</Banner>}
      {view?.snapshot.error && <Banner title={view.snapshot.error}>Snapshot failed, showing the last known fleet: {view.snapshot.error}</Banner>}

      <div className="scroll fleet">
        {!view ? (
          <Zero icon={CircleDashed} title="Reading the fleet">
            Looking at the home's records.
          </Zero>
        ) : rows.length === 0 ? (
          <Zero icon={Sailboat} title="No tasks in flight">
            New tasks appear here as soon as Firstmate starts them.
          </Zero>
        ) : (
          <DataTable
            label="Tasks in flight"
            columns={columns}
            rows={rows}
            rowKey={(r) => r.id}
            selected={selectedTask}
            onSelect={(r) => {
              setSelectedTask(r.id);
            }}
            rowClass={(r) => (changed.has(r.id) ? "changed" : undefined)}
            rowVersion={(r) => changed.get(r.id) ?? 0}
          />
        )}
      </div>

      <Composer enabled={inApp && !info?.home_error && view !== null} />
      </Panel>
    </>
  );
}
