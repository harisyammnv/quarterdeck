import type { KeyboardEvent, Ref } from "react";
import { PaperPlaneTiltIcon as PaperPlaneTilt } from "@phosphor-icons/react";
import { Keycap } from "./Keycap";

/**
 * NoteField (design system): a full-pill field with its primary action 3px inside, `qd-input`.
 * The placeholder names who receives the text. ⌘↵ sends; a plain Return does nothing.
 */
export function NoteField({
  value,
  onChange,
  onSend,
  placeholder,
  disabled,
  busy,
  label = "Note for Firstmate",
  sendLabel = "Send note",
  inputRef,
}: {
  value: string;
  onChange: (v: string) => void;
  onSend: () => void;
  placeholder: string;
  disabled?: boolean;
  busy?: boolean;
  label?: string;
  sendLabel?: string;
  inputRef?: Ref<HTMLInputElement>;
}) {
  const onKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key !== "Enter") return;
    e.preventDefault();
    if (e.metaKey) onSend();
  };
  return (
    <label className="qd-input">
      <span className="sr-only">{label}</span>
      <input
        ref={inputRef}
        value={value}
        disabled={disabled}
        placeholder={placeholder}
        maxLength={4000}
        onChange={(e) => {
          onChange(e.target.value);
        }}
        onKeyDown={onKeyDown}
      />
      <button type="submit" className="qd-btn qd-btn--primary qd-btn--sm" disabled={disabled || busy || !value.trim()}>
        <PaperPlaneTilt size={14} weight="bold" />
        {busy ? "Sending" : sendLabel}
        <Keycap>⌘↵</Keycap>
      </button>
    </label>
  );
}
