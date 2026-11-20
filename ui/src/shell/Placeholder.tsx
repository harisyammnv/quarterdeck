import type { ComponentType } from "react";
import type { Icon } from "@phosphor-icons/react";
import { Panel, PanelHead } from "../ui";

/**
 * The intentional placeholder for an area that is not built yet: the area's header and one
 * centred card that says what will live here and when. It is a promise, not an error.
 */
function Placeholder({ title, icon: I, phase, children }: { title: string; icon: Icon; phase: string; children: string }) {
  return (
    <Panel as="main" className="fill" label={title}>
      <PanelHead title={title} />
      <div className="placeholder">
        <div className="ph-card">
          <span className="ph-ic">
            <I size={22} />
          </span>
          <span className="ph-phase">{phase}</span>
          <h2>{title} is on its way</h2>
          <p>{children}</p>
        </div>
      </div>
    </Panel>
  );
}

/** A view component that shows the placeholder for an area that is not built yet. */
export function placeholder(title: string, icon: Icon, text: string, phase = "Phase 1"): ComponentType {
  const View = () => (
    <Placeholder title={title} icon={icon} phase={phase}>
      {text}
    </Placeholder>
  );
  View.displayName = `Placeholder(${title})`;
  return View;
}
