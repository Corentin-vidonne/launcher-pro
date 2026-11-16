import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import type {
  AppInfo,
  Details,
  Featured,
  Game,
  Library,
  OffersResult,
  SearchResult,
  Settings,
  VaultStatus,
  Collection,
  StorageInfo,
} from "./types";

/** Vrai dans la fenêtre Tauri ; faux dans un simple navigateur (aperçu de développement). */
export const inApp = "__TAURI_INTERNALS__" in window;

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (inApp) return invoke<T>(command, args);
  // Hors application (vite dev dans un navigateur), des données factices remplacent le backend.
  // Branche éliminée du build final : le module de démonstration n'y est pas embarqué.
  if (import.meta.env.DEV) {
    const { handle } = await import("./mock");
    return handle(command, args) as T;
  }
  throw new Error("Backend indisponible");
}

export const api = {
  library: () => call<Library>("get_library"),
  rescan: () => call<Library>("rescan_library"),
  launch: (id: string) => call<void>("launch_game", { id }),
  openSource: (id: string) => call<void>("open_source", { id }),
  reveal: (id: string) => call<void>("reveal_game", { id }),
  setFavorite: (id: string, value: boolean) => call<void>("set_favorite", { id, value }),
  setHidden: (id: string, value: boolean) => call<Library>("set_hidden", { id, value }),
  vaultStatus: () => call<VaultStatus>("vault_status"),
  vaultSetPassword: (password: string, current?: string) =>
    call<Library>("vault_set_password", { password, current: current ?? null }),
  vaultUnlock: (password: string) => call<Library>("vault_unlock", { password }),
  vaultLock: () => call<Library>("vault_lock"),
  setCustomArt: (id: string, kind: "cover" | "hero", path: string | null) =>
    call<Game>("set_custom_art", { id, kind, path }),
  collections: () => call<Collection[]>("get_collections"),
  saveCollection: (id: string | null, name: string) => call<Collection[]>("save_collection", { id, name }),
  deleteCollection: (id: string) => call<Collection[]>("delete_collection", { id }),
  toggleInCollection: (id: string, gameId: string) => call<Collection[]>("toggle_in_collection", { id, gameId }),
  storage: () => call<StorageInfo>("storage_info"),
  quickOpen: () => call<void>("quick_open"),
  addCustom: (path: string, name?: string) => call<Library>("add_custom_game", { path, name }),
  removeCustom: (id: string) => call<Library>("remove_custom_game", { id }),
  appInfo: () => call<AppInfo>("get_app_info"),
  setSettings: (settings: Settings) => call<void>("set_settings", { settings }),
  storeSearch: (query: string) => call<SearchResult>("store_search", { query }),
  storeDetails: (appid: number) => call<Details>("store_details", { appid }),
  storeOffers: (title: string, publishers: string[]) =>
    call<OffersResult>("store_offers", { title, publishers }),
  storeFeatured: () => call<Featured>("store_featured"),
  openUrl: (url: string) => call<void>("open_url", { url }),
};

/**
 * Abonnement aux mises à jour poussées par le backend (fin d'analyse, jaquette trouvée).
 * La promesse se résout une fois l'écoute active : on charge la bibliothèque après, pour ne
 * manquer aucun événement entre les deux.
 */
export async function subscribe(handlers: {
  library: (library: Library) => void;
  art: (game: Game) => void;
  openStore: (query: string) => void;
}): Promise<() => void> {
  if (!inApp) return () => {};
  const offs = await Promise.all([
    listen<Library>("library-updated", (e) => handlers.library(e.payload)),
    listen<Game>("art-updated", (e) => handlers.art(e.payload)),
    listen<string>("open-store", (e) => handlers.openStore(e.payload)),
  ]);
  return () => offs.forEach((off) => off());
}

export const windowControls = {
  show: () => (inApp ? getCurrentWindow().show() : Promise.resolve()),
  minimize: () => (inApp ? getCurrentWindow().minimize() : Promise.resolve()),
  toggleMaximize: () => (inApp ? getCurrentWindow().toggleMaximize() : Promise.resolve()),
  close: () => (inApp ? getCurrentWindow().close() : Promise.resolve()),
  setFullscreen: (on: boolean) => (inApp ? getCurrentWindow().setFullscreen(on) : Promise.resolve()),
  setTheme: (theme: "dark" | "light" | null) =>
    inApp ? getCurrentWindow().setTheme(theme) : Promise.resolve(),
};

/** Sélecteur de fichier natif pour ajouter un jeu à la main. */
export async function pickExecutable(): Promise<string | null> {
  if (!inApp) return null;
  const picked = await openDialog({
    title: "Choisir l'exécutable du jeu",
    multiple: false,
    directory: false,
    filters: [{ name: "Applications", extensions: ["exe", "lnk", "bat", "url"] }],
  });
  return typeof picked === "string" ? picked : null;
}

/** Sélecteur d'image pour une jaquette ou un fond personnalisé. */
export async function pickImage(title: string): Promise<string | null> {
  if (!inApp) return null;
  const picked = await openDialog({
    title,
    multiple: false,
    directory: false,
    filters: [{ name: "Images", extensions: ["jpg", "jpeg", "png", "webp", "avif"] }],
  });
  return typeof picked === "string" ? picked : null;
}
