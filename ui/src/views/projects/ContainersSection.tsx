import { CubeIcon as Cube } from "@phosphor-icons/react";
import { InspectorSection, ZeroState } from "../../ui";
import { useApp } from "../../state/app";

/**
 * The inspector's Containers section for the worktree in focus: a placeholder until its data
 * source exists, saying so plainly instead of showing invented numbers.
 * Hand-over: the slice that owns containers replaces this registry line with its own
 * section and deletes this file.
 */
export default function ContainersSection() {
  const { selectedTask } = useApp();
  if (!selectedTask) return null;
  return (
    <InspectorSection title="Containers">
      <ZeroState icon={Cube} title="Not connected yet">
        Containers for this worktree will show here.
      </ZeroState>
    </InspectorSection>
  );
}
