/**
 * The terminal area's door to the core. In the app window it calls the generated command;
 * in a plain browser it serves invented preview data.
 */
import { inApp } from "../../bridge";
import { commands, type Span, type TerminalTail, type Tone } from "../../bindings";
import { previewTail } from "./preview";

export type { Span, TerminalTail, Tone };

/** Reads the last lines of a task's terminal; rejects with a readable message. */
export function terminalTail(taskId: string): Promise<TerminalTail> {
  return inApp ? commands.terminalTail(taskId) : previewTail(taskId);
}
