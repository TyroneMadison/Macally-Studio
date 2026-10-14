import { useState } from "react";
import Rail, { type View } from "./components/Rail";
import { useAppState } from "./lib/state";
import Keyboard from "./views/Keyboard";
import Profiles from "./views/Profiles";

export default function App() {
  const store = useAppState();
  const [view, setView] = useState<View>("keyboard");
  const paused = store.status?.paused ?? false;

  return (
    <div className="app">
      <Rail view={view} onNav={setView} paused={paused} onTogglePause={() => void store.setPaused(!paused)} />
      <main className="view">
        {view === "keyboard" && <Keyboard store={store} />}
        {view === "profiles" && <Profiles store={store} onNav={setView} />}
        {view === "settings" && <section><h1>Settings</h1></section>}
      </main>
    </div>
  );
}
