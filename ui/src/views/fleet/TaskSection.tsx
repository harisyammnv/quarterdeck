import { CursorClickIcon as CursorClick } from "@phosphor-icons/react";
import { Zero, Facts, StatePill, ago, useNow } from "../../ui";
import { useApp } from "../../state/app";

/**
 * The inspector's first section: facts about the task in focus. Other areas add their own
 * sections (containers, events) through `inspectorSections` in the registry.
 */
export default function TaskSection() {
  const { live, selectedTask } = useApp();
  const now = useNow(5000);
  const row = live.view?.rows.find((r) => r.id === selectedTask) ?? null;
  if (!row) {
    return (
      <section>
        <h2 className="h2">Task</h2>
        <Zero icon={CursorClick} title="No task in focus" dashed={false}>
          Pick a task to see its facts here.
        </Zero>
      </section>
    );
  }
  const ev = row.latest_event;
  return (
    <>
      <section>
        <div className="row">
          <h2 className="h2 grow trunc">Task</h2>
          <StatePill state={row.state} />
        </div>
        <Facts
          items={[
            ["Name", row.name],
            ["Id", <span className="mono">{row.id}</span>],
            ["Project", row.project ?? "unknown"],
            ["Kind", row.kind ?? "task"],
          ]}
        />
      </section>
      <section>
        <h2 className="h2">Latest event</h2>
        {ev ? (
          <div className="evt">
            <span className="tm">{ev.at ? ago(ev.at * 1000, now) : ""}</span>
            <span className="grow">{ev.text || ev.word}</span>
          </div>
        ) : (
          <span className="mut">No status events yet.</span>
        )}
        <span className="mut t12">An event is a moment, not the current state.</span>
      </section>
    </>
  );
}
