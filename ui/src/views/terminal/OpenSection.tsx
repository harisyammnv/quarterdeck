import { FileTextIcon as FileText, TerminalWindowIcon as TerminalWindow } from "@phosphor-icons/react";
import { Button } from "../../ui";
import { useApp } from "../../state/app";
import { TASK_AREA, setTaskTab, type TaskTab } from "./open";

/** An inspector section that opens the task in focus on its terminal or its report. */
export default function OpenSection() {
  const { area, setArea, selectedTask } = useApp();
  if (!selectedTask || area === TASK_AREA) return null;
  const open = (tab: TaskTab) => {
    setTaskTab(tab);
    setArea(TASK_AREA);
  };
  return (
    <section aria-label="Open the task">
      <h2 className="h2">Worker</h2>
      <div className="row gap6">
        <Button
          size="sm"
          icon={TerminalWindow}
          onClick={() => {
            open("terminal");
          }}
        >
          Terminal
        </Button>
        <Button
          size="sm"
          icon={FileText}
          onClick={() => {
            open("report");
          }}
        >
          Report
        </Button>
      </div>
    </section>
  );
}
