import { useCallback, useEffect, useState } from "react";
import GameStatus from "./components/GameStatus";
import GameToast from "./components/GameToast";
import PermissionGate from "./components/PermissionGate";
import Rail, { type View } from "./components/Rail";
import { switchesAuto } from "./lib/games";
import { useAppState, type AppState } from "./lib/state";
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
  // The keyboard view edits the active profile, so a profile opened from Game profiles becomes active.
  const edit = (id: string) => void store.setActiveProfile(id).then(() => setView("keyboard"));

  return (
    <PermissionGate store={store}>
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
          {view === "games" && <Games store={store} onEdit={edit} />}
        </main>
      </div>
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
