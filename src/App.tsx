import { useCallback, useEffect, useRef, useState } from "react";
import GameStatus from "./components/GameStatus";
import GameToast from "./components/GameToast";
import PermissionGate from "./components/PermissionGate";
import Rail, { type View } from "./components/Rail";
import { switchedAuto, switchedIn, switchesAuto } from "./lib/games";
import { useAppState, type AppState } from "./lib/state";
import GameEditor from "./views/GameEditor";
import Games from "./views/Games";
import Keyboard from "./views/Keyboard";
import Profiles from "./views/Profiles";
import Settings from "./views/Settings";

export default function App() {
  const store = useAppState();
  const [view, setView] = useState<View>("keyboard");
  const [toast, setToast] = useState<AppState["switched"]>(null);
  // a found update (launch, tray or manual check) brings up the banner in Settings
  useEffect(() => {
    if (store.update) setView("settings");
  }, [store.update]);
  // a game detected in front: its notice, while the corner icon pops in
  useEffect(() => {
    if (store.switched?.game) setToast(store.switched);
  }, [store.switched]);
  const clearToast = useCallback(() => setToast(null), []);

  const active = store.activeProfile;
  const game = active?.game ?? null;
  const detected = store.switched?.game && store.switched.id === active?.id ? store.switched.at : undefined;

  // A game's profile switching in, detected or picked by hand, opens its full-screen editor.
  const [editor, setEditor] = useState<{ id: string; auto: boolean; key?: string; back: View } | null>(null);
  const seen = useRef(active?.id);
  useEffect(() => {
    const p = switchedIn(seen.current, active?.id, store.profiles);
    seen.current = active?.id;
    if (p) setEditor({ id: p.id, auto: switchedAuto(store.switched, p.id, Date.now()), back: view });
  }, [active?.id]);
  const editing = editor && store.profiles.some((p) => p.id === editor.id) ? editor : null;
  const close = (to: View) => {
    setView(to);
    setEditor(null);
  };

  return (
    <PermissionGate store={store}>
      {editing ? (
        <GameEditor
          key={`${editing.id}/${editing.key ?? ""}`}
          store={store}
          profileId={editing.id}
          auto={editing.auto}
          initialKey={editing.key}
          onBack={() => close(editing.back)}
          onProfiles={() => close("games")}
          onExit={() => close("keyboard")}
        />
      ) : (
        <div className="app">
          <Rail
            view={view}
            onNav={setView}
            corner={
              <GameStatus
                game={game}
                watching={store.settings.gameAutoSwitch}
                live={!!active && store.liveGame === active.id}
                auto={!!game && switchesAuto(game, store.settings)}
                pop={detected}
                onOpen={() => setView("games")}
              />
            }
          />
          <main className="view">
            {view === "keyboard" && <Keyboard store={store} />}
            {view === "profiles" && <Profiles store={store} onNav={setView} />}
            {view === "settings" && <Settings store={store} />}
            {view === "games" && <Games store={store} onEdit={(id, key) => setEditor({ id, auto: false, key, back: "games" })} />}
          </main>
        </div>
      )}
      {toast?.game && (
        <GameToast
          key={toast.at}
          game={toast.game}
          profileName={toast.name}
          bindings={Object.keys(store.profiles.find((p) => p.id === toast.id)?.bindings ?? {}).length}
          auto
          onDone={clearToast}
        />
      )}
    </PermissionGate>
  );
}
