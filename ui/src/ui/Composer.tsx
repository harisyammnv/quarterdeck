import { useRef, useState } from "react";
import { PaperPlaneTiltIcon as PaperPlaneTilt } from "@phosphor-icons/react";
import { inApp, sendNote } from "../bridge";

function newRequestId(): string {
  return `qd-${crypto.randomUUID()}`;
}

type Hint = { tone: "" | "ok" | "err"; text: string };

/**
 * The one write path: a note to Firstmate through its inbox. `about` names a task, which is
 * prefixed to the note so Firstmate knows what it refers to. One request id per draft, so a
 * retried send replays the same note instead of creating a second one.
 */
export function Composer({ enabled, about, placeholder, initialHint }: { enabled: boolean; about?: string; placeholder?: string; initialHint?: string }) {
  const [text, setText] = useState("");
  const [busy, setBusy] = useState(false);
  const [hint, setHint] = useState<Hint>({ tone: "", text: initialHint ?? "One note goes to Firstmate's inbox. It is read at Firstmate's next check." });
  const requestId = useRef<string | null>(null);
  const field = useRef<HTMLInputElement>(null);

  const send = async () => {
    const body = text.trim();
    if (!enabled || busy || !body) return;
    requestId.current ??= newRequestId();
    setBusy(true);
    try {
      const r = await sendNote(about ? `About task ${about}: ${body}` : body, requestId.current);
      if (r.announced || r.acknowledged) {
        setHint({
          tone: "ok",
          text: r.outcome === "replay" ? `Already sent as note ${r.id}.` : `Sent as note ${r.id}. Firstmate will read it at its next check.`,
        });
        setText("");
        requestId.current = null;
      } else {
        setHint({ tone: "err", text: `Saved as note ${r.id}, but Firstmate was not woken. Send again to retry the wake; it will not create a second note.` });
      }
    } catch (e) {
      // The draft and its request id are kept: sending again replays the same note if the
      // intake saved it, and can never create a second one.
      setHint({
        tone: "err",
        text: `Firstmate's intake reported a problem: ${e instanceof Error ? e.message : String(e)}. Sending again is safe; it cannot create a second note.`,
      });
    } finally {
      setBusy(false);
    }
  };

  return (
    <form
      className="composer"
      onSubmit={(e) => {
        e.preventDefault();
        void send();
      }}
    >
      <label className="input">
        <span className="sr-only">Note for Firstmate</span>
        <input
          ref={field}
          value={text}
          disabled={!enabled}
          placeholder={enabled ? (placeholder ?? "Leave a note for Firstmate") : inApp ? "Notes need a Firstmate home to go to" : "Notes are sent from the app window"}
          maxLength={4000}
          onChange={(e) => {
            setText(e.target.value);
            requestId.current = null;
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.metaKey) e.preventDefault();
            if (e.key === "Enter" && e.metaKey) {
              e.preventDefault();
              void send();
            }
          }}
        />
        <button type="submit" className="btn sm primary" disabled={!enabled || busy || !text.trim()}>
          <PaperPlaneTilt size={14} weight="bold" />
          {busy ? "Sending" : "Send note"}
          <span className="kbd" aria-hidden="true">
            ⌘↵
          </span>
        </button>
      </label>
      <span className={`hint ${hint.tone}`} role="status">
        {hint.text}
      </span>
    </form>
  );
}
