import { CheckIcon as Check, PauseIcon as Pause, QuestionIcon as Question } from "@phosphor-icons/react";
import type { TaskState } from "../bindings";
import { STATE_LOOK } from "../states";
import type { DotKind } from "./StateDot";
import { QuietPill, StatusPill, Tag } from "./Pill";

/** The state dot for a task state, or null for settled states. */
export function stateDot(state: TaskState): DotKind | null {
  return STATE_LOOK[state].dot;
}

/** The chip for a task state. `word` overrides the label (an unknown word stays honest). */
export function StatePill({ state, word, title }: { state: TaskState; word?: string; title?: string }) {
  const look = STATE_LOOK[state];
  const label = word ?? look.word;
  if (look.tone === "quiet") return <QuietPill icon={Check} title={title}>{label}</QuietPill>;
  if (look.tone === "neutral") {
    return (
      <Tag icon={state === "paused" ? Pause : Question} title={title}>
        {label}
      </Tag>
    );
  }
  return (
    <StatusPill tone={look.tone} title={title}>
      {label}
    </StatusPill>
  );
}
