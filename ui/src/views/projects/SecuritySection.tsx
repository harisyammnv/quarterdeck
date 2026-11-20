import { ShieldCheckIcon as ShieldCheck } from "@phosphor-icons/react";
import { Zero } from "../../ui";
import { useApp } from "../../state/app";
import { findRow, useProjects } from "./bridge";

/**
 * The inspector's Security section for the worktree in focus. A placeholder until its data source
 * exists: it says so plainly instead of showing invented numbers.
 */
export default function SecuritySection() {
  const { selectedTask } = useApp();
  const { view } = useProjects();
  const project = findRow(view, selectedTask)?.project ?? null;
  if (!selectedTask) return null;
  return (
    <section>
      <div className="row">
        <h2 className="h2 grow">Security</h2>
        {project && <span className="mut projects-small trunc">{project}</span>}
      </div>
      <Zero icon={ShieldCheck} title="Not scanned yet" dashed={false}>
        Findings show here once scans run.
      </Zero>
    </section>
  );
}
