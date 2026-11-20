import { useEffect, useRef, useState } from "react";
import { ArrowSquareOutIcon as ArrowSquareOut, WarningCircleIcon as WarningCircle } from "@phosphor-icons/react";
import { urlOpen } from "../bridge";
import { Button } from "./Button";

/**
 * The confirmation every link from untrusted text passes before it opens: it shows the full
 * address, and only "Open in browser" hands it to the core, which checks it again.
 */
export function ConfirmLink({ url, onClose }: { url: string; onClose: () => void }) {
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const dialog = useRef<HTMLDialogElement>(null);
  const https = /^https:\/\//i.test(url);

  useEffect(() => {
    const d = dialog.current;
    if (d && !d.open) d.showModal();
  }, []);

  const open = async () => {
    setBusy(true);
    setError(null);
    try {
      await urlOpen(url);
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <dialog ref={dialog} className="confirm" aria-labelledby="confirm-title" onClose={onClose} onCancel={onClose}>
      <h2 id="confirm-title" className="h2">
        Open this link?
      </h2>
      <p className="mut">It comes from a document or a worker, not from Quarterdeck. Check the address before you open it.</p>
      <code className="confirm-url mono">{url}</code>
      {!https && (
        <p className="confirm-err" role="alert">
          <WarningCircle size={14} /> Only https links can be opened.
        </p>
      )}
      {error && (
        <p className="confirm-err" role="alert">
          <WarningCircle size={14} /> {error}
        </p>
      )}
      <div className="confirm-actions">
        <Button onClick={onClose}>Cancel</Button>
        <Button variant="primary" icon={ArrowSquareOut} disabled={!https || busy} onClick={() => void open()}>
          Open in browser
        </Button>
      </div>
    </dialog>
  );
}
