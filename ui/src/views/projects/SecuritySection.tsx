import { ShieldCheckIcon as ShieldCheck } from "@phosphor-icons/react";
import { InspectorSection, ZeroState } from "../../ui";
import { useApp } from "../../state/app";
import { findRow, useProjects } from "./bridge";

/**
 * The inspector's Security section for the worktree in focus: a placeholder until its data
 * source exists, saying so plainly instead of showing invented numbers.
 * Hand-over: the slice that owns security replaces this registry line with its own
 * section and deletes this file.
 */
export default function SecuritySection() {
  const { selectedTask } = useApp();
  const { view } = useProjects();
  const project = findRow(view, selectedTask)?.project ?? null;
  if (!selectedTask) return null;
  return (
    <InspectorSection title="Security" aside={project ? <span className="qd-muted projects-small qd-trunc">{project}</span> : undefined}>
      <ZeroState icon={ShieldCheck} title="Not scanned yet">
        Findings show here once scans run.
      </ZeroState>
    </InspectorSection>
  );
}
