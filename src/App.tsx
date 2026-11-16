import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import { Ambient } from "./components/Ambient";
import { GameSheet } from "./components/GameSheet";
import {
  IconCheck,
  IconCollection,
  IconEye,
  IconEyeOff,
  IconFolder,
  IconHeart,
  IconLock,
  IconPencil,
  IconPlay,
  IconPlus,
  IconTrash,
} from "./components/Icons";
import { LibraryView } from "./components/LibraryView";
import { PromptSheet, type PromptState } from "./components/PromptSheet";
import { SettingsView } from "./components/SettingsView";
import { ContextMenu, type MenuState } from "./components/Shell";
import { Sidebar } from "./components/Sidebar";
import { StorageView } from "./components/StorageView";
import { StoreSheet } from "./components/StoreSheet";
import { StoreView } from "./components/StoreView";
import { TvMode, type TvRow } from "./components/TvMode";
import { api, inApp, pickExecutable, pickImage, subscribe, windowControls } from "./lib/api";
import { titleKey } from "./lib/format";
import { setLensEnabled } from "./lib/lens";
import type { AppInfo, Collection, Game, Library, Settings, Source, StoreHit, View } from "./lib/types";

const EMPTY: Library = { games: [], sources: [], scannedAt: 0, scanning: true };

const DEFAULT_INFO: AppInfo = {
  settings: {
    theme: "system",
    afterLaunch: "minimize",
    showTools: false,
    showAllStores: false,
    gridSize: "medium",
    liquidGlass: true,
    quickShortcut: "ctrl+alt+space",
    launchAtStartup: true,
  },
  translucent: false,
  country: "FR",
  version: "",
  quickShortcutActive: false,
};

const byName = (a: Game, b: Game) => a.name.localeCompare(b.name, "fr", { sensitivity: "base" });

export default function App() {
  const [library, setLibrary] = useState<Library>(EMPTY);
  const [info, setInfo] = useState<AppInfo>(DEFAULT_INFO);
  const [collections, setCollections] = useState<Collection[]>([]);
  const [view, setView] = useState<View>({ kind: "library", filter: "all" });
  const [openGame, setOpenGame] = useState<string | null>(null);
  const [storeHit, setStoreHit] = useState<StoreHit | null>(null);
  const [storeRequest, setStoreRequest] = useState<{ query: string; nonce: number } | null>(null);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [prompt, setPrompt] = useState<PromptState | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [tv, setTv] = useState(false);
  const scanning = busy || library.scanning;
  // Survol d'une jaquette : l'arrière-plan épouse son artwork (avec un léger délai, pour ne pas
  // enchaîner les changements quand la souris traverse la grille). En quittant une jaquette, le
  // dernier jeu survolé reste affiché ; on annule seulement un survol trop bref pour compter.
  const [hoverId, setHoverId] = useState<string | null>(null);
  const hoverTimer = useRef(0);
  const onHover = useCallback((id: string | null) => {
    clearTimeout(hoverTimer.current);
    if (id) hoverTimer.current = window.setTimeout(() => setHoverId(id), 240);
  }, []);

  // Coffre des jeux masqués : le mot de passe saisi dans les réglages fait apparaître l'onglet
  // « Masqués » ; verrouillé (ou au redémarrage), ces jeux ne sont même plus envoyés à l'interface.
  const [vaultOpen, setVaultOpen] = useState(false);
  const vaultOpenRef = useRef(vaultOpen);
  vaultOpenRef.current = vaultOpen;
  const lockVault = useCallback(() => {
    api
      .vaultLock()
      .then((lib) => {
        setLibrary(lib);
        setVaultOpen(false);
      })
      .catch((e) => setToast(String(e)));
  }, []);
  const showHidden = useCallback(() => setView({ kind: "library", filter: "hidden" }), []);
  useEffect(() => {
    if (!vaultOpen && view.kind === "library" && view.filter === "hidden") setView({ kind: "library", filter: "all" });
  }, [vaultOpen, view]);

  const settings = info.settings;
  // Références lues par les actions stables (les cartes mémoïsées ne se re-rendent pas pour rien).
  const libraryRef = useRef(library);
  libraryRef.current = library;
  const settingsRef = useRef(settings);
  settingsRef.current = settings;
  const collectionsRef = useRef(collections);
  collectionsRef.current = collections;

  // ── Mode TV (plein écran, manette)
  const enterTv = useCallback(() => {
    setOpenGame(null);
    setStoreHit(null);
    setMenu(null);
    setTv(true);
    void windowControls.setFullscreen(true);
  }, []);
  const exitTv = useCallback(() => {
    setTv(false);
    void windowControls.setFullscreen(false);
  }, []);

  // ── Démarrage : la bibliothèque du dernier lancement s'affiche tout de suite, le rescan suit.
  useEffect(() => {
    let alive = true;
    let unsubscribe = () => {};
    (async () => {
      try {
        unsubscribe = await subscribe({
          library: setLibrary,
          art: (game) => setLibrary((l) => ({ ...l, games: l.games.map((g) => (g.id === game.id ? game : g)) })),
          // Recherche rapide → « Chercher dans le magasin ».
          openStore: (query) => {
            setTv(false);
            setOpenGame(null);
            setStoreRequest({ query, nonce: Date.now() });
            setView({ kind: "store" });
          },
        });
        const [appInfo, lib, cols] = await Promise.all([api.appInfo(), api.library(), api.collections()]);
        if (!alive) return;
        setInfo(appInfo);
        setLibrary(lib);
        setCollections(cols);
        api.vaultStatus().then((s) => alive && setVaultOpen(s.unlocked)).catch(() => {});
      } catch (e) {
        setToast(String(e));
      } finally {
        // La fenêtre n'apparaît qu'une fois l'interface prête (pas de flash blanc), puis le
        // contenu arrive en fondu : le moteur web ne peint sa première image qu'après l'affichage.
        void windowControls.show().finally(() => document.documentElement.classList.add("shown"));
      }
    })();
    // Le menu contextuel du moteur web (Recharger, Inspecter…) n'a pas sa place dans l'application.
    const noMenu = (e: MouseEvent) => e.preventDefault();
    if (inApp) window.addEventListener("contextmenu", noMenu);
    return () => {
      alive = false;
      unsubscribe();
      window.removeEventListener("contextmenu", noMenu);
    };
  }, []);

  // Bouton Start/Menu d'une manette : passe en mode TV. Sondage léger (4 fois par seconde), et
  // seulement quand une manette est branchée et que la fenêtre a le focus.
  useEffect(() => {
    if (tv) return;
    let timer = 0;
    let wasPressed = true; // ignore un bouton déjà enfoncé au moment où l'on commence à écouter
    const pads = () => [...navigator.getGamepads()].filter((p): p is Gamepad => !!p && p.connected);
    const poll = () => {
      if (!document.hasFocus()) return;
      const pressed = pads().some((p) => p.buttons[9]?.pressed);
      if (pressed && !wasPressed) enterTv();
      wasPressed = pressed;
    };
    const start = () => {
      if (!timer) timer = window.setInterval(poll, 250);
    };
    const stop = () => {
      if (pads().length === 0 && timer) {
        clearInterval(timer);
        timer = 0;
      }
    };
    window.addEventListener("gamepadconnected", start);
    window.addEventListener("gamepaddisconnected", stop);
    if (pads().length) start();
    return () => {
      clearInterval(timer);
      window.removeEventListener("gamepadconnected", start);
      window.removeEventListener("gamepaddisconnected", stop);
    };
  }, [tv, enterTv]);

  // ── Apparence
  useEffect(() => {
    const root = document.documentElement;
    const media = matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      root.dataset.theme = settings.theme === "system" ? (media.matches ? "dark" : "light") : settings.theme;
    };
    apply();
    media.addEventListener("change", apply);
    // La fenêtre suit le même thème pour que l'effet Mica s'accorde à l'interface.
    windowControls.setTheme(settings.theme === "system" ? null : settings.theme).catch(() => {});
    return () => media.removeEventListener("change", apply);
  }, [settings.theme]);

  useEffect(() => {
    document.documentElement.dataset.grid = settings.gridSize;
  }, [settings.gridSize]);

  useEffect(() => {
    document.documentElement.classList.toggle("flat", !settings.liquidGlass);
    setLensEnabled(settings.liquidGlass);
  }, [settings.liquidGlass]);

  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(() => setToast(null), 4500);
    return () => clearTimeout(timer);
  }, [toast]);

  // ── Actions
  const patchGame = useCallback((id: string, patch: Partial<Game>) => {
    setLibrary((l) => ({ ...l, games: l.games.map((g) => (g.id === id ? { ...g, ...patch } : g)) }));
  }, []);

  const replaceGame = useCallback((game: Game) => {
    setLibrary((l) => ({ ...l, games: l.games.map((g) => (g.id === game.id ? game : g)) }));
  }, []);

  const play = useCallback(
    (id: string) => {
      api
        .launch(id)
        .then(() => {
          patchGame(id, { lastPlayed: Math.floor(Date.now() / 1000) });
          const after = settingsRef.current.afterLaunch;
          if (after === "minimize") void windowControls.minimize();
          // « Quitter » : on laisse le temps au jeu de démarrer, puis l'application se ferme
          // entièrement (plus aucune mémoire ni processus pendant la partie).
          if (after === "quit") setTimeout(() => void windowControls.close(), 1500);
        })
        .catch((e) => setToast(String(e)));
    },
    [patchGame],
  );

  const toggleFavorite = useCallback(
    (game: Game) => {
      patchGame(game.id, { favorite: !game.favorite });
      api.setFavorite(game.id, !game.favorite).catch((e) => setToast(String(e)));
    },
    [patchGame],
  );

  // Masquer : le jeu quitte la bibliothèque ; il n'est visible que dans l'onglet « Masqués »,
  // lui-même accessible seulement après le mot de passe des réglages. Réafficher : l'inverse.
  const toggleHidden = useCallback(
    (game: Game) => {
      const hide = !game.hidden;
      patchGame(game.id, { hidden: hide });
      if (hide && !vaultOpenRef.current) setOpenGame((open) => (open === game.id ? null : open));
      api
        .setHidden(game.id, hide)
        .then((lib) => {
          setLibrary(lib);
          if (!hide) return;
          setToast(
            vaultOpenRef.current
              ? `« ${game.name} » est passé dans Masqués.`
              : `« ${game.name} » est masqué. Déverrouillez l'onglet « Masqués » dans Réglages.`,
          );
        })
        .catch((e) => setToast(String(e)));
    },
    [patchGame],
  );

  const reveal = useCallback((id: string) => {
    api.reveal(id).catch((e) => setToast(String(e)));
  }, []);

  const openSource = useCallback((id: string) => {
    api.openSource(id).catch((e) => setToast(String(e)));
  }, []);

  const rescan = useCallback(() => {
    setBusy(true);
    api
      .rescan()
      .then(setLibrary)
      .catch((e) => setToast(String(e)))
      .finally(() => setBusy(false));
  }, []);

  const removeCustom = useCallback((id: string) => {
    api
      .removeCustom(id)
      .then(setLibrary)
      .catch((e) => setToast(String(e)));
  }, []);

  const addGame = useCallback(async () => {
    try {
      const path = await pickExecutable();
      if (path) setLibrary(await api.addCustom(path));
    } catch (e) {
      setToast(String(e));
    }
  }, []);

  const changeSettings = useCallback((patch: Partial<Settings>) => {
    const next = { ...settingsRef.current, ...patch };
    settingsRef.current = next;
    setInfo((i) => ({ ...i, settings: next }));
    api
      .setSettings(next)
      .catch((e) => setToast(String(e)))
      // Toujours relire l'état réel : un raccourci peut avoir été refusé (déjà pris ailleurs).
      .finally(() => api.appInfo().then(setInfo).catch(() => {}));
  }, []);

  const searchStore = useCallback((name: string) => {
    setOpenGame(null);
    setStoreRequest({ query: name.replace(/[™®©]/g, ""), nonce: Date.now() });
    setView({ kind: "store" });
  }, []);

  // ── Jaquettes personnalisées
  const changeArt = useCallback(
    async (game: Game, kind: "cover" | "hero") => {
      try {
        const path = await pickImage(kind === "cover" ? "Choisir une jaquette" : "Choisir une image de fond");
        if (path) replaceGame(await api.setCustomArt(game.id, kind, path));
      } catch (e) {
        setToast(String(e));
      }
    },
    [replaceGame],
  );

  const resetArt = useCallback(
    async (game: Game) => {
      try {
        await api.setCustomArt(game.id, "cover", null);
        replaceGame(await api.setCustomArt(game.id, "hero", null));
      } catch (e) {
        setToast(String(e));
      }
    },
    [replaceGame],
  );

  // ── Collections
  const toggleInCollection = useCallback((collectionId: string, gameId: string) => {
    api
      .toggleInCollection(collectionId, gameId)
      .then(setCollections)
      .catch((e) => setToast(String(e)));
  }, []);

  /** Nouvelle collection ; si un jeu est donné, il y est ajouté d'emblée. */
  const newCollection = useCallback((gameId?: string) => {
    setPrompt({
      title: "Nouvelle collection",
      value: "",
      placeholder: "Coop entre potes, Solo, À finir…",
      confirm: "Créer",
      onConfirm: async (name) => {
        let list = await api.saveCollection(null, name);
        const created = list[list.length - 1];
        if (gameId && created) list = await api.toggleInCollection(created.id, gameId);
        setCollections(list);
      },
    });
  }, []);

  const collectionMenu = useCallback((id: string, x: number, y: number) => {
    const collection = collectionsRef.current.find((c) => c.id === id);
    if (!collection) return;
    setMenu({
      x,
      y,
      items: [
        {
          label: "Renommer",
          icon: <IconPencil />,
          action: () =>
            setPrompt({
              title: "Renommer la collection",
              value: collection.name,
              confirm: "Renommer",
              onConfirm: async (name) => setCollections(await api.saveCollection(collection.id, name)),
            }),
        },
        {
          label: "Supprimer",
          icon: <IconTrash />,
          danger: true,
          separatorBefore: true,
          action: () =>
            setPrompt({
              title: `Supprimer « ${collection.name} » ?`,
              message: "Les jeux restent dans votre bibliothèque : seul ce regroupement disparaît.",
              confirm: "Supprimer",
              danger: true,
              onConfirm: async () => {
                setCollections(await api.deleteCollection(collection.id));
                setView((v) => (v.kind === "collection" && v.id === collection.id ? { kind: "library", filter: "all" } : v));
              },
            }),
        },
      ],
    });
  }, []);

  const openMenu = useCallback(
    (id: string, x: number, y: number) => {
      const game = libraryRef.current.games.find((g) => g.id === id);
      if (!game) return;
      const items: MenuState["items"] = [];
      if (game.canLaunch) items.push({ label: "Jouer", icon: <IconPlay />, action: () => play(id) });
      items.push({
        label: game.favorite ? "Retirer des favoris" : "Ajouter aux favoris",
        icon: <IconHeart />,
        action: () => toggleFavorite(game),
      });
      if (game.installDir) {
        items.push({ label: "Ouvrir le dossier", icon: <IconFolder />, action: () => reveal(id) });
      }
      collectionsRef.current.slice(0, 6).forEach((c, i) => {
        const inside = c.games.includes(id);
        items.push({
          label: inside ? `Retirer de « ${c.name} »` : `Ajouter à « ${c.name} »`,
          icon: inside ? <IconCheck /> : <IconCollection />,
          separatorBefore: i === 0,
          action: () => toggleInCollection(c.id, id),
        });
      });
      items.push({
        label: "Nouvelle collection…",
        icon: <IconPlus />,
        separatorBefore: collectionsRef.current.length === 0,
        action: () => newCollection(id),
      });
      items.push({
        label: game.hidden ? "Réafficher" : "Masquer",
        icon: game.hidden ? <IconEye /> : <IconEyeOff />,
        separatorBefore: true,
        action: () => toggleHidden(game),
      });
      if (game.source === "custom") {
        items.push({ label: "Retirer de la bibliothèque", icon: <IconTrash />, danger: true, action: () => removeCustom(id) });
      }
      setMenu({ x, y, items });
    },
    [play, toggleFavorite, toggleHidden, reveal, removeCustom, toggleInCollection, newCollection],
  );

  const closeMenu = useCallback(() => setMenu(null), []);
  const closeGame = useCallback(() => setOpenGame(null), []);
  const closeStore = useCallback(() => setStoreHit(null), []);
  const closePrompt = useCallback(() => setPrompt(null), []);

  // ── Données dérivées
  const sources = useMemo(() => new Map<string, Source>(library.sources.map((s) => [s.id, s])), [library.sources]);

  const visible = useMemo(
    () => library.games.filter((g) => settings.showTools || !g.isTool),
    [library.games, settings.showTools],
  );

  // Jeu mis en avant : le dernier joué (avec artwork), ou celui que la souris survole.
  const featured = useMemo(() => {
    let best: Game | null = null;
    for (const g of visible) {
      if (g.hidden || !(g.hero || g.cover)) continue;
      if (g.lastPlayed && (!best || g.lastPlayed > (best.lastPlayed ?? 0))) best = g;
    }
    return best ?? visible.find((g) => !g.hidden && (g.hero || g.cover)) ?? null;
  }, [visible]);
  const hovered = hoverId ? visible.find((g) => g.id === hoverId) : undefined;
  // Un jeu masqué ne colore le fond que dans l'onglet « Masqués ».
  const viewingHidden = view.kind === "library" && view.filter === "hidden";
  const spotlight = hovered && (hovered.hero || hovered.cover) && (!hovered.hidden || viewingHidden) ? hovered : featured;

  const counts = useMemo(
    () => ({
      all: visible.filter((g) => !g.hidden).length,
      favorites: visible.filter((g) => g.favorite && !g.hidden).length,
      hidden: visible.filter((g) => g.hidden).length,
    }),
    [visible],
  );

  const listed = useMemo(() => {
    if (view.kind === "source") return visible.filter((g) => g.source === view.source && !g.hidden);
    if (view.kind === "collection") {
      const ids = new Set(collections.find((c) => c.id === view.id)?.games ?? []);
      return visible.filter((g) => ids.has(g.id) && !g.hidden);
    }
    if (view.kind !== "library") return [];
    switch (view.filter) {
      case "favorites":
        return visible.filter((g) => g.favorite && !g.hidden);
      case "recent":
        return visible.filter((g) => g.lastPlayed && !g.hidden);
      case "hidden":
        return visible.filter((g) => g.hidden);
      default:
        return visible.filter((g) => !g.hidden);
    }
  }, [visible, view, collections]);

  // Rangées du mode TV : récents, favoris, chaque collection, puis tout.
  const tvRows = useMemo<TvRow[]>(() => {
    if (!tv) return [];
    const base = visible.filter((g) => !g.hidden && g.canLaunch);
    const rows: TvRow[] = [];
    const recent = base
      .filter((g) => g.lastPlayed)
      .sort((a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0))
      .slice(0, 20);
    if (recent.length) rows.push({ title: "Récemment joués", games: recent });
    const favorites = base.filter((g) => g.favorite).sort(byName);
    if (favorites.length) rows.push({ title: "Favoris", games: favorites });
    for (const c of collections) {
      const ids = new Set(c.games);
      const games = base.filter((g) => ids.has(g.id)).sort(byName);
      if (games.length) rows.push({ title: c.name, games });
    }
    rows.push({ title: "Tous les jeux", games: [...base].sort(byName) });
    return rows;
  }, [tv, visible, collections]);

  const selected = openGame ? library.games.find((g) => g.id === openGame) : undefined;
  const owned = useMemo(
    () => (storeHit ? library.games.find((g) => titleKey(g.name) === storeHit.key) : undefined),
    [storeHit, library.games],
  );

  // ── Rendu de la zone principale
  let main: ReactNode;
  if (view.kind === "store") {
    main = <StoreView sources={sources} request={storeRequest} onOpen={setStoreHit} />;
  } else if (view.kind === "storage") {
    main = <StorageView games={visible} sources={sources} onOpen={setOpenGame} onReveal={reveal} />;
  } else if (view.kind === "settings") {
    main = (
      <SettingsView
        info={info}
        library={library}
        scanning={scanning}
        onChange={changeSettings}
        onRescan={rescan}
        onAddGame={() => void addGame()}
        vaultOpen={vaultOpen}
        onVaultOpen={setVaultOpen}
        onShowHidden={showHidden}
        onLibrary={setLibrary}
        onOpenSource={openSource}
      />
    );
  } else {
    const source = view.kind === "source" ? sources.get(view.source) : undefined;
    const collection = view.kind === "collection" ? collections.find((c) => c.id === view.id) : undefined;
    const filter = view.kind === "library" ? view.filter : "all";
    const title =
      view.kind === "source"
        ? (source?.name ?? "Source")
        : view.kind === "collection"
          ? (collection?.name ?? "Collection")
          : { all: "Tous les jeux", favorites: "Favoris", recent: "Récents", hidden: "Masqués" }[filter];
    const empty =
      view.kind === "source"
        ? { title: "Aucun jeu installé", text: `Les jeux installés avec ${source?.name ?? "ce launcher"} apparaîtront ici.` }
        : view.kind === "collection"
          ? { title: "Collection vide", text: "Clic droit sur un jeu, puis « Ajouter à » cette collection." }
          : {
              all: scanning
                ? { title: "Analyse du PC…", text: "Recherche des launchers et des jeux installés." }
                : { title: "Aucun jeu détecté", text: "Installez un jeu ou ajoutez-en un manuellement depuis les réglages." },
              favorites: { title: "Aucun favori", text: "Clic droit sur un jeu, puis « Ajouter aux favoris »." },
              recent: { title: "Rien pour l'instant", text: "Les jeux auxquels vous jouez apparaîtront ici." },
              hidden: { title: "Aucun jeu masqué", text: "Clic droit sur un jeu, puis « Masquer »." },
            }[filter];
    main = (
      <LibraryView
        // Recherche et tri repartent de zéro quand on change de rubrique.
        key={
          view.kind === "source" ? `source:${view.source}` : view.kind === "collection" ? `collection:${view.id}` : `library:${filter}`
        }
        title={title}
        games={listed}
        sources={sources}
        hero={view.kind === "library" && filter === "all"}
        spotlight={spotlight}
        resume={spotlight?.id === featured?.id}
        fixedSort={view.kind === "library" && filter === "recent" ? "recent" : undefined}
        showSource={view.kind !== "source"}
        empty={empty}
        extra={
          source?.canOpen ? (
            <button className="btn" onClick={() => openSource(source.id)}>
              Ouvrir {source.name}
            </button>
          ) : view.kind === "library" && filter === "hidden" ? (
            <button className="btn" onClick={lockVault}>
              <IconLock />
              Verrouiller
            </button>
          ) : undefined
        }
        scanning={scanning}
        onRescan={rescan}
        onOpen={setOpenGame}
        onPlay={play}
        onMenu={openMenu}
        onHover={onHover}
      />
    );
  }

  return (
    <div className="app">
      <Ambient src={spotlight ? (spotlight.hero ?? spotlight.cover) : null} blurred={!!spotlight && !spotlight.hero} />
      <Sidebar
        view={view}
        library={library}
        counts={counts}
        collections={collections}
        vaultOpen={vaultOpen}
        onNavigate={setView}
        onNewCollection={newCollection}
        onCollectionMenu={collectionMenu}
        onTv={enterTv}
      />
      <main className="content">{main}</main>

      {selected && (
        <GameSheet
          game={selected}
          source={sources.get(selected.source)}
          collections={collections}
          onClose={closeGame}
          onPlay={play}
          onFavorite={toggleFavorite}
          onHidden={toggleHidden}
          onReveal={reveal}
          onOpenSource={openSource}
          onStore={searchStore}
          onRemove={removeCustom}
          onToggleCollection={toggleInCollection}
          onNewCollection={newCollection}
          onChangeArt={changeArt}
          onResetArt={resetArt}
        />
      )}
      {storeHit && (
        <StoreSheet
          key={storeHit.key}
          hit={storeHit}
          sources={sources}
          owned={owned}
          onClose={closeStore}
          onPlay={play}
          onOpenSource={openSource}
        />
      )}
      {prompt && <PromptSheet key={prompt.title} prompt={prompt} onClose={closePrompt} />}
      {menu && <ContextMenu menu={menu} onClose={closeMenu} />}
      {tv && <TvMode rows={tvRows} sources={sources} onPlay={play} onFavorite={toggleFavorite} onExit={exitTv} />}
      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}
