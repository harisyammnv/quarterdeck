import { useApp } from "../../state/app";

/**
 * Changes when the home is switched and when its first snapshot arrives, so memory and
 * skills re-list then: project guides and project skills come from the snapshot.
 */
export function useHomeKey(): string {
  const { info, live } = useApp();
  const hasSnapshot = live.view?.snapshot.generated_at != null;
  return `${info?.home ?? ""}|${hasSnapshot ? "snapshot" : "none"}`;
}
