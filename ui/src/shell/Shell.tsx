import { Suspense } from "react";
import { useHotkey } from "../ui";
import { useApp } from "../state/app";
import { inspectorSections, railViews, settingsView, viewById } from "../registry";
import { NoHome } from "./NoHome";
import { ProjectsPane } from "./ProjectsPane";
import { Rail } from "./Rail";
import { StatusBar } from "./StatusBar";
import { TitleBar } from "./TitleBar";

function Loading() {
  return <div className="panel fill" aria-busy="true" />;
}

/** One rail shortcut, ⌘n for rail position n. */
function RailKey({ n }: { n: number }) {
  const { setArea } = useApp();
  const view = railViews[n - 1];
  useHotkey(`mod+${String(n)}`, () => {
    if (view) setArea(view.id);
  }, view !== undefined);
  return null;
}

/** The global shortcuts: ⌘1 to ⌘9 for the rail, ⌘, for Settings, ⌘\\ for the inspector. */
function Shortcuts() {
  const { setArea, toggleInspector } = useApp();
  useHotkey("mod+,", () => {
    setArea(settingsView.id);
  });
  useHotkey("mod+\\", toggleInspector);
  return (
    <>
      {[1, 2, 3, 4, 5, 6, 7, 8, 9].map((n) => (
        <RailKey key={n} n={n} />
      ))}
    </>
  );
}

const sections = [...inspectorSections].sort((a, b) => a.order - b.order);

/**
 * The window frame. Each view decides, through its registry entry, whether the project list
 * and the inspector show around it; the shell draws them and the areas fill them.
 */
export function Shell() {
  const { area, info, inspectorOpen } = useApp();
  const def = viewById(area);
  const View = def.component;
  const homeless = info !== null && info.home === null;
  const needsHome = def.needsHome ?? true;
  const showFrames = !(homeless && needsHome);
  return (
    <div className="app">
      <Shortcuts />
      <TitleBar />
      <div className="shell">
        <Rail />
        {showFrames && def.layout.projects && <ProjectsPane />}
        <div className="main">
          {homeless && needsHome ? (
            <NoHome />
          ) : (
            <Suspense fallback={<Loading />}>
              <View />
            </Suspense>
          )}
        </div>
        {showFrames && def.layout.inspector && inspectorOpen && (
          <aside className="insp panel" aria-label="Inspector">
            <Suspense fallback={null}>
              {sections.map((s) => {
                const S = s.component;
                return <S key={s.id} />;
              })}
            </Suspense>
          </aside>
        )}
      </div>
      <StatusBar />
    </div>
  );
}
