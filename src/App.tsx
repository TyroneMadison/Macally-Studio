import { useEffect, useState } from "react";
import PermissionGate from "./components/PermissionGate";
import Rail, { type View } from "./components/Rail";
import { useAppState } from "./lib/state";
import Keyboard from "./views/Keyboard";
import Profiles from "./views/Profiles";
import Settings from "./views/Settings";

export default function App() {
  const store = useAppState();
  const [view, setView] = useState<View>("keyboard");
  const paused = store.status?.paused ?? false;
  // a found update (launch, tray or manual check) brings up the banner in Settings
  useEffect(() => {
    if (store.update) setView("settings");
  }, [store.update]);

  return (
    <PermissionGate store={store}>
      <div className="app">
        <Rail view={view} onNav={setView} paused={paused} onTogglePause={() => void store.setPaused(!paused)} />
        <main className="view">
          {view === "keyboard" && <Keyboard store={store} />}
          {view === "profiles" && <Profiles store={store} onNav={setView} />}
          {view === "settings" && <Settings store={store} />}
        </main>
      </div>
    </PermissionGate>
  );
}
