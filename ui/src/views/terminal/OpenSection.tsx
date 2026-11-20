import { FileTextIcon as FileText, TerminalWindowIcon as TerminalWindow } from "@phosphor-icons/react";
import { Button, InspectorSection } from "../../ui";
import { useApp } from "../../state/app";
import { TASK_AREA, useOpenTask } from "./open";

/** An inspector section that opens the task in focus on its terminal or its report. */
export default function OpenSection() {
  const { area, selectedTask } = useApp();
  const open = useOpenTask();
  if (!selectedTask || area === TASK_AREA) return null;
  return (
    <InspectorSection title="Worker">
      <div className="qd-row terminal-open">
        <Button
          size="sm"
          icon={TerminalWindow}
          onClick={() => {
            open(selectedTask, "terminal");
          }}
        >
          Terminal
        </Button>
        <Button
          size="sm"
          icon={FileText}
          onClick={() => {
            open(selectedTask, "report");
          }}
        >
          Report
        </Button>
      </div>
    </InspectorSection>
  );
}
