import { useEffect, useState, useSyncExternalStore } from "react";
import { CubeIcon as Cube } from "@phosphor-icons/react";
import { containersStatus, lastContainers, onContainers, type EngineSummary } from "./bridge";

/**
 * The Docker context in the status bar: one cheap call (discovery and `/version`, no listing)
 * when the window starts, then whatever the Containers view last listed.
 */
export default function EngineStatus() {
  const [summary, setSummary] = useState<EngineSummary | null>(null);
  const listed = useSyncExternalStore(onContainers, lastContainers);
  useEffect(() => {
    let alive = true;
    containersStatus()
      .then((s) => {
        if (alive) setSummary(s);
      })
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, []);
  const now = listed ?? summary;
  if (!now) return null;
  return (
    <span className="row">
      <Cube size={14} />
      {now.status === "ready" ? `Docker context ${now.context ?? "default"}` : "Docker not running"}
    </span>
  );
}
