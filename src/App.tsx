import { useState } from "react";
import Rail, { type View } from "./components/Rail";

export default function App() {
  const [view, setView] = useState<View>("keyboard");
  const [paused, setPaused] = useState(false);

  return (
    <div className="app">
      <Rail view={view} onNav={setView} paused={paused} onTogglePause={() => setPaused((p) => !p)} />
      <main className="view">
        {view === "keyboard" && <section><h1>Keyboard</h1></section>}
        {view === "profiles" && <section><h1>Profiles</h1></section>}
        {view === "settings" && <section><h1>Settings</h1></section>}
      </main>
    </div>
  );
}
