import { AnchorSimpleIcon as AnchorSimple, FolderOpenIcon as FolderOpen } from "@phosphor-icons/react";
import { inApp } from "../bridge";
import { Button, Panel } from "../ui";
import { useApp } from "../state/app";

/** First launch, or a home that went away: ask for the folder, once, and remember it. */
export function NoHome() {
  const { info, chooseHome, homeBusy, homeError } = useApp();
  const reason = info?.home_error ?? "";
  const notChosen = reason === "" || reason.startsWith("no Firstmate home");
  return (
    <Panel as="main" className="fill" label="Choose the Firstmate home">
      <div className="placeholder">
        <div className="ph-card">
          <span className="ph-ic brandish">
            <AnchorSimple size={22} />
          </span>
          <h2>{notChosen ? "Choose your Firstmate home" : "The Firstmate home cannot be read"}</h2>
          <p>
            {notChosen
              ? "Quarterdeck watches one Firstmate home: the folder that holds its bin and state folders. Pick it once and the app remembers it, even when opened from Finder."
              : `${reason.charAt(0).toUpperCase()}${reason.slice(1)}. Pick the home folder again.`}
          </p>
          <Button variant="primary" icon={FolderOpen} disabled={!inApp || homeBusy} onClick={() => void chooseHome()}>
            {homeBusy ? "Choosing" : "Choose folder"}
          </Button>
          {homeError && (
            <p className="set-err" role="alert">
              {homeError}
            </p>
          )}
          {!inApp && <p className="mut t12">The folder is chosen from the app window.</p>}
        </div>
      </div>
    </Panel>
  );
}
