import { CursorClickIcon as CursorClick } from "@phosphor-icons/react";
import { Facts, InspectorSection, StatePill, ZeroState, ago, useNow } from "../../ui";
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
      <InspectorSection title="Task">
        <ZeroState icon={CursorClick} title="No task in focus">
          Pick a task to see its facts here.
        </ZeroState>
      </InspectorSection>
    );
  }
  const ev = row.latest_event;
  return (
    <>
      <InspectorSection title="Task" aside={<StatePill state={row.state} />}>
        <Facts
          items={[
            ["Name", row.name],
            ["Id", <span className="qd-mono">{row.id}</span>],
            ["Project", row.project ?? "unknown"],
            ["Kind", row.kind ?? "task"],
          ]}
        />
      </InspectorSection>
      <InspectorSection title="Latest event">
        {ev ? (
          <div className="ui-evt">
            <span className="ui-evt__tm">{ev.at ? ago(ev.at * 1000, now) : ""}</span>
            <span className="qd-row">{ev.text || ev.word}</span>
          </div>
        ) : (
          <span className="qd-muted">No status events yet.</span>
        )}
        <span className="qd-muted ui-t12">An event is a moment, not the current state.</span>
      </InspectorSection>
    </>
  );
}
