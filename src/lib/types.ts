export interface Game {
  id: string;
  source: string;
  name: string;
  installDir: string | null;
  sizeBytes: number | null;
  lastPlayed: number | null;
  playtimeMin: number | null;
  isTool: boolean;
  favorite: boolean;
  hidden: boolean;
  canLaunch: boolean;
  cover: string | null;
  hero: string | null;
  logo: string | null;
  icon: string | null;
  /** Jaquette ou fond choisi par l'utilisateur. */
  customArt: boolean;
}

export interface Source {
  id: string;
  name: string;
  installed: boolean;
  canOpen: boolean;
  count: number;
  icon: string | null;
}

export interface Library {
  games: Game[];
  sources: Source[];
  scannedAt: number;
  /** Une analyse du PC est en cours. */
  scanning: boolean;
}

export interface Settings {
  theme: "system" | "dark" | "light";
  /** Que devient Launcher Pro une fois un jeu lancé. */
  afterLaunch: "stay" | "minimize" | "quit";
  showTools: boolean;
  showAllStores: boolean;
  gridSize: "small" | "medium" | "large";
  /** Verre translucide avec réfraction (coupé = panneaux opaques, sans coût GPU). */
  liquidGlass: boolean;
  /** Raccourci global de la recherche rapide (« ctrl+alt+space »), vide = désactivé. */
  quickShortcut: string;
  /** Lancer Launcher Pro (agrandi) à l'ouverture de session Windows. */
  launchAtStartup: boolean;
}

export interface AppInfo {
  settings: Settings;
  translucent: boolean;
  country: string;
  version: string;
  /** Le raccourci de recherche rapide est réellement actif (pas pris par une autre application). */
  quickShortcutActive: boolean;
}

export interface Offer {
  store: string;
  storeName: string;
  title: string;
  edition: string | null;
  price: number | null;
  regular: number | null;
  discount: number | null;
  currency: string;
  free: boolean;
  url: string;
  note: string | null;
}

export interface StoreHit {
  key: string;
  title: string;
  image: string | null;
  cover: string | null;
  hero: string | null;
  description: string | null;
  steamAppid: number | null;
  offers: Offer[];
}

export interface SearchResult {
  hits: StoreHit[];
  failed: string[];
}

export interface Screenshot {
  thumb: string;
  full: string;
}

export interface Details {
  appid: number;
  name: string;
  summary: string;
  about: string;
  header: string | null;
  screenshots: Screenshot[];
  genres: string[];
  features: string[];
  developers: string[];
  publishers: string[];
  releaseDate: string | null;
  comingSoon: boolean;
  metacritic: number | null;
  recommendations: number | null;
  website: string | null;
  requirements: string | null;
}

export interface LinkStore {
  source: string;
  name: string;
  /** Page de recherche de la boutique ; absente = ouvrir le launcher. */
  url: string | null;
}

export interface OffersResult {
  offers: Offer[];
  links: LinkStore[];
  failed: string[];
}

export interface FeaturedItem {
  appid: number;
  title: string;
  image: string | null;
  price: number | null;
  regular: number | null;
  discount: number;
  currency: string;
}

export interface Featured {
  specials: FeaturedItem[];
  topSellers: FeaturedItem[];
  newReleases: FeaturedItem[];
}

/** Ce que la zone principale affiche. */
export type View =
  | { kind: "library"; filter: "all" | "favorites" | "recent" | "hidden" }
  | { kind: "source"; source: string }
  | { kind: "collection"; id: string }
  | { kind: "storage" }
  | { kind: "store" }
  | { kind: "settings" };

export interface VaultStatus {
  hasPassword: boolean;
  unlocked: boolean;
}

export interface Collection {
  id: string;
  name: string;
  games: string[];
}

export interface StorageInfo {
  disks: { root: string; label: string; total: number; free: number }[];
  games: { id: string; root: string; size: number; measured: boolean }[];
}
