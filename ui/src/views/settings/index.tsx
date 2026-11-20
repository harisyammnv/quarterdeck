import { DesktopIcon as Desktop, FolderOpenIcon as FolderOpen, LockSimpleIcon as LockSimple, MoonIcon as Moon, SunIcon as Sun } from "@phosphor-icons/react";
import { inApp } from "../../bridge";
import { Button, Panel, PanelHead, SegmentedTabs, StatusChip } from "../../ui";
import { useApp, type ThemePref } from "../../state/app";

function sourceText(source: string | null | undefined): string {
  switch (source) {
    case "qd-fm-home":
      return "Set by QD_FM_HOME in the environment, which wins over a home chosen here.";
    case "fm-home":
      return "Set by FM_HOME in the environment, which wins over a home chosen here.";
    case "chosen":
      return "Chosen in this window and remembered in the app's own settings. Quarterdeck runs this home's own read-only scripts.";
    default:
      return "No home is chosen yet.";
  }
}

export default function SettingsView() {
  const { info, theme, setTheme, chooseHome, homeBusy, homeError } = useApp();
  const home = info?.home ?? null;
  return (
    <Panel as="main" className="fill" label="Settings">
      <PanelHead title="Settings" />
      <div className="scroll settings">
        <section className="set-sect">
          <h2 className="h2">Firstmate home</h2>
          <div className="set-row">
            <span className="set-ic">
              <FolderOpen size={18} />
            </span>
            <span className="grow">
              <span className="row">
                <b className="mono trunc">{home ?? "Not chosen"}</b>
                {home ? <StatusChip tone="success">Watching</StatusChip> : <StatusChip tone="warning">Needed</StatusChip>}
              </span>
              <span className="mut">{sourceText(info?.home_source)}</span>
            </span>
            <Button variant={home ? "secondary" : "primary"} icon={FolderOpen} disabled={!inApp || homeBusy} onClick={() => void chooseHome()}>
              {homeBusy ? "Choosing" : home ? "Choose another folder" : "Choose folder"}
            </Button>
          </div>
          {homeError && (
            <p className="set-err" role="alert">
              {homeError}
            </p>
          )}
          {!inApp && <p className="mut">The home is chosen from the app window. This preview shows invented data.</p>}
        </section>
        <section className="set-sect">
          <h2 className="h2">Appearance</h2>
          <div className="set-row">
            <span className="grow">Theme</span>
            <SegmentedTabs<ThemePref>
              label="Theme"
              value={theme}
              onChange={setTheme}
              items={[
                { id: "light", label: "Light", icon: Sun },
                { id: "dark", label: "Dark", icon: Moon },
                { id: "system", label: "Match the system", icon: Desktop },
              ]}
            />
          </div>
        </section>
        <section className="set-sect">
          <h2 className="h2">About</h2>
          <div className="set-row">
            <span className="set-ic">
              <LockSimple size={18} />
            </span>
            <span className="grow">
              <b>Read-only companion</b>
              <span className="mut">
                Quarterdeck never dispatches, steers, merges or cleans up work. Its only write is a note to Firstmate's inbox. Version {info?.version ?? ""}.
              </span>
            </span>
          </div>
        </section>
      </div>
    </Panel>
  );
}
