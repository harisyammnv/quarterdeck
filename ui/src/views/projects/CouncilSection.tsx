import { ScalesIcon as Scales } from "@phosphor-icons/react";
import { InspectorSection, ZeroState } from "../../ui";
import { useApp } from "../../state/app";

/**
 * The inspector's Council section for the worktree in focus: a placeholder until its data
 * source exists, saying so plainly instead of showing invented numbers.
 * Hand-over: the slice that owns council replaces this registry line with its own
 * section and deletes this file.
 */
export default function CouncilSection() {
  const { selectedTask } = useApp();
  if (!selectedTask) return null;
  return (
    <InspectorSection title="Council">
      <ZeroState icon={Scales} title="No review yet">
        Verdicts on its pull requests show here.
      </ZeroState>
    </InspectorSection>
  );
}
