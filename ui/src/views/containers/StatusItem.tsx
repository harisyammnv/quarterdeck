import { useEffect, useSyncExternalStore } from "react";
import { CubeIcon as Cube } from "@phosphor-icons/react";
import { lastContainers, loadContainers, onContainers } from "./bridge";

let asked = false;

/** The status bar's Docker context, from the containers view's last listing. */
export default function ContainersStatus() {
  const view = useSyncExternalStore(onContainers, lastContainers);
  // One listing per session for the status bar; the view refreshes it from then on.
  useEffect(() => {
    if (asked) return;
    asked = true;
    loadContainers().catch(() => undefined);
  }, []);
  if (!view) return null;
  const ctx = view.context ?? "default";
  return (
    <span className="row">
      <Cube size={14} />
      {view.status === "ready" ? `Docker context ${ctx}` : "Docker not running"}
    </span>
  );
}
