import { CubeIcon as Cube } from "@phosphor-icons/react";
import { Zero } from "../../ui";
import { useApp } from "../../state/app";

/**
 * The inspector's Containers section for the worktree in focus. A placeholder until its data source
 * exists: it says so plainly instead of showing invented numbers.
 */
export default function ContainersSection() {
  const { selectedTask } = useApp();
  if (!selectedTask) return null;
  return (
    <section>
      <div className="row">
        <h2 className="h2 grow">Containers</h2>
      </div>
      <Zero icon={Cube} title="No containers matched" dashed={false}>
        Containers from this worktree show here.
      </Zero>
    </section>
  );
}
