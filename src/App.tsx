import { useState } from "react";
import Rail, { type View } from "./components/Rail";
import { useAppState } from "./lib/state";
import Keyboard from "./views/Keyboard";

export default function App() {
  const store = useAppState();
  const [view, setView] = useState<View>("keyboard");
  const paused = store.status?.paused ?? false;

  return (
    <div className="app">
      <Rail view={view} onNav={setView} paused={paused} onTogglePause={() => void store.setPaused(!paused)} />
      <main className="view">
        {view === "keyboard" && <Keyboard store={store} />}
        {view === "profiles" && <section><h1>Profiles</h1></section>}
        {view === "settings" && <section><h1>Settings</h1></section>}
      </main>
    </div>
  );
}
