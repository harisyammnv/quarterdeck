import { AppProvider } from "./state/app";
import { Shell } from "./shell/Shell";

export function App() {
  return (
    <AppProvider>
      <Shell />
    </AppProvider>
  );
}
