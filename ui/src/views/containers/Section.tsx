import { useEffect, useSyncExternalStore } from "react";
import { CubeIcon as Cube } from "@phosphor-icons/react";
import { InspectorSection, StatusChip, Tag, ZeroState } from "../../ui";
import { useApp } from "../../state/app";
import { lastContainers, loadContainers, onContainers, type ContainerRow } from "./bridge";
import "./containers.css";

let asked = false;

function Chip({ row }: { row: ContainerRow }) {
  if (row.state === "running") return <StatusChip tone="success">Running</StatusChip>;
  if (row.state === "exited" && row.exit_code !== null && row.exit_code !== 0) return <StatusChip tone="danger">Exited</StatusChip>;
  if (row.state === "paused" || row.state === "restarting") return <StatusChip tone="warning">{row.state === "paused" ? "Paused" : "Restarting"}</StatusChip>;
  return <Tag>{row.state === "exited" ? "Stopped" : row.state === "created" ? "Created" : "Unknown"}</Tag>;
}

/** The inspector's Containers section: the containers of the task in focus, by worktree. */
export default function ContainersSection() {
  const { selectedTask } = useApp();
  const view = useSyncExternalStore(onContainers, lastContainers);
  // One listing when the section first shows; the Containers view refreshes it from then on.
  useEffect(() => {
    if (asked || lastContainers()) return;
    asked = true;
    loadContainers().catch(() => undefined);
  }, []);
  if (!selectedTask) return null;
  if (!view) {
    return (
      <InspectorSection title="Containers">
        <span className="qd-muted">Reading Docker.</span>
      </InspectorSection>
    );
  }
  if (view.status !== "ready") {
    return (
      <InspectorSection title="Containers">
        <span className="qd-muted">{view.status === "unsupported" ? "The Docker context is not local." : "Docker is not running."}</span>
      </InspectorSection>
    );
  }
  const group = view.groups.find((g) => g.task_id === selectedTask);
  const rows = group?.containers ?? [];
  return (
    <InspectorSection title="Containers" aside={rows.length > 0 ? <span className="qd-muted tn">{rows.length}</span> : undefined}>
      {rows.length === 0 ? (
        <ZeroState icon={Cube} title="No containers">
          Nothing runs from this worktree.
        </ZeroState>
      ) : (
        <ul className="containers-isect">
          {rows.map((r) => {
            const port = r.ports.find((p) => p.host !== null)?.host;
            return (
              <li key={r.id}>
                <span className="mono t12 trunc" title={r.name}>
                  {r.service ?? r.name}
                </span>
                {port !== undefined && port !== null && <span className="mono t12 mut">{port}</span>}
                <span className="grow" />
                <Chip row={r} />
              </li>
            );
          })}
        </ul>
      )}
    </InspectorSection>
  );
}
