import { AnchorSimpleIcon as AnchorSimple } from "@phosphor-icons/react";
import { useApp } from "../state/app";
import { railViews, settingsView, type ViewDef } from "../registry";

function RailItem({ view, shortcut, badge }: { view: ViewDef; shortcut: string; badge: number }) {
  const { area, setArea } = useApp();
  const on = area === view.id;
  const I = view.icon;
  return (
    <button
      type="button"
      className="ri"
      aria-current={on ? "page" : undefined}
      aria-label={`${view.label}${badge ? `, ${badge} need you` : ""}, ${shortcut}`}
      onClick={() => {
        setArea(view.id);
      }}
    >
      <span className="ic">
        <I size={18} weight={on ? "duotone" : "regular"} />
        {badge > 0 && <span className="badge tn">{badge}</span>}
      </span>
      <span className="lb">{view.label}</span>
      <span className="tip" role="tooltip">
        <b>
          {view.label}
          <span className="tk">{shortcut}</span>
        </b>
      </span>
    </button>
  );
}

/** The labelled black rail: brand disc, the views in order, and Settings at the foot. */
export function Rail() {
  const { live } = useApp();
  return (
    <nav className="rail" aria-label="Areas">
      <span className="brand" aria-hidden="true">
        <AnchorSimple size={22} />
      </span>
      {railViews.map((v, i) => (
        <RailItem key={v.id} view={v} shortcut={`⌘${i + 1}`} badge={v.badge?.(live.view) ?? 0} />
      ))}
      <span className="sp" />
      <RailItem view={settingsView} shortcut="⌘," badge={0} />
    </nav>
  );
}
