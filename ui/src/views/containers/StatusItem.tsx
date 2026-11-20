import { Suspense, lazy } from "react";

const EngineStatus = lazy(() => import("./EngineStatus"));

/**
 * The status bar's Docker context. The registry imports only this wrapper; the code that
 * talks to the engine loads on its own, after the window is up.
 */
export default function ContainersStatus() {
  return (
    <Suspense fallback={null}>
      <EngineStatus />
    </Suspense>
  );
}
