import { ArrowSquareOutIcon as ArrowSquareOut, GitBranchIcon as GitBranch } from "@phosphor-icons/react";
import { useState } from "react";
import { Button, Facts, InspectorSection, KindPill, ZeroState, ago, useHotkey, useNow } from "../../ui";
import { useApp } from "../../state/app";
import { findRow, focusInOrca, useProjects, type ProjectRow } from "./bridge";
import { agentLabel, deliveryLabel, startedLabel } from "./look";

/**
 * The inspector's worktree facts for the task in focus: agent, branch, delivery and start,
 * and "Open in Orca" (⌘O) when Orca runs it with a terminal on record.
 */
export default function WorktreeSection() {
  const { view } = useProjects();
  const { selectedTask } = useApp();
  const found = findRow(view, selectedTask);
  if (!found) {
    return (
      <InspectorSection title="Worktree">
        <ZeroState icon={GitBranch} title="No worktree in focus">
          Pick one in the project list.
        </ZeroState>
      </InspectorSection>
    );
  }
  return <Facts_ row={found.row} />;
}

function Facts_({ row }: { row: ProjectRow }) {
  const now = useNow(30_000);
  const missing = <span className="qd-muted">Not recorded</span>;
  return (
    <InspectorSection title="Worktree" aside={<KindPill kind={row.kind} />}>
      <Facts
        items={[
          ["Agent", agentLabel(row.harness) ?? missing],
          ["Branch", row.branch ? <span className="qd-mono projects-branch" title={row.branch}>{row.branch}</span> : missing],
          ["Delivery", deliveryLabel(row.mode) ?? missing],
          ["Started", row.started_at ? <span className="projects-tn">{startedLabel(row.started_at, now, ago)}</span> : missing],
        ]}
      />
      {row.in_orca && <OpenInOrca id={row.id} />}
    </InspectorSection>
  );
}

/** Brings the task's terminal forward in Orca. ⌘O does the same while it is shown. */
function OpenInOrca({ id }: { id: string }) {
  const [error, setError] = useState<{ id: string; text: string } | null>(null);
  const open = () => {
    setError(null);
    focusInOrca(id).catch((e: unknown) => {
      setError({ id, text: e instanceof Error ? e.message : String(e) });
    });
  };
  useHotkey("mod+o", open);
  const shown = error?.id === id ? error.text : null;
  return (
    <div className="projects-orca">
      <Button size="sm" icon={ArrowSquareOut} shortcut="⌘O" onClick={open}>
        Open in Orca
      </Button>
      {shown && (
        <span className="projects-err" role="status">
          {shown}
        </span>
      )}
    </div>
  );
}
