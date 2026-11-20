import { ScalesIcon as Scales } from "@phosphor-icons/react";
import { Zero } from "../../ui";
import { useApp } from "../../state/app";

/**
 * The inspector's Council section for the worktree in focus. A placeholder until its data source
 * exists: it says so plainly instead of showing invented numbers.
 */
export default function CouncilSection() {
  const { selectedTask } = useApp();
  if (!selectedTask) return null;
  return (
    <section>
      <div className="row">
        <h2 className="h2 grow">Council</h2>
      </div>
      <Zero icon={Scales} title="No review yet" dashed={false}>
        Verdicts on its pull requests show here.
      </Zero>
    </section>
  );
}
