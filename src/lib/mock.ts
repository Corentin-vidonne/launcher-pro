// Données de démonstration pour prévisualiser l'interface dans un navigateur (`npm run dev`).
// Jamais embarqué dans l'application : voir `api.ts`.

import type { AppInfo, Collection, Details, Featured, Game, Library, OffersResult, SearchResult, StorageInfo, StoreHit } from "./types";

const cdn = "https://shared.steamstatic.com/store_item_assets/steam/apps";
const now = Math.floor(Date.now() / 1000);

function steam(appid: number, name: string, daysAgo: number, minutes: number, gb: number): Game {
  return {
    id: `steam:${appid}`,
    source: "steam",
    name,
    installDir: `D:\\SteamLibrary\\steamapps\\common\\${name}`,
    sizeBytes: gb * 1024 ** 3,
    lastPlayed: now - daysAgo * 86400,
    playtimeMin: minutes,
    isTool: false,
    favorite: false,
    hidden: false,
    canLaunch: true,
    cover: `${cdn}/${appid}/library_600x900.jpg`,
    hero: `${cdn}/${appid}/library_hero.jpg`,
    logo: `${cdn}/${appid}/logo.png`,
    icon: null,
    customArt: false,
  };
}

function other(source: string, name: string, daysAgo: number | null, cover: string | null = null): Game {
  return {
    id: `${source}:${name}`,
    source,
    name,
    installDir: `D:\\${name}`,
    sizeBytes: null,
    lastPlayed: daysAgo === null ? null : now - daysAgo * 86400,
    playtimeMin: null,
    isTool: false,
    favorite: false,
    hidden: false,
    canLaunch: true,
    cover,
    hero: null,
    logo: null,
    icon: null,
    customArt: false,
  };
}

const games: Game[] = [
  { ...steam(1245620, "ELDEN RING", 0.3, 5210, 66), favorite: true },
  steam(1091500, "Cyberpunk 2077", 12, 3120, 82),
  steam(2050650, "Resident Evil 4", 2, 940, 68),
  { ...steam(1172470, "Apex Legends", 1, 12400, 71), favorite: true },
  steam(805550, "Assetto Corsa Competizione", 240, 310, 19),
  steam(678950, "DRAGON BALL FighterZ", 35, 420, 6.5),
  steam(285900, "Gang Beasts", 320, 95, 1.2),
  steam(448510, "Overcooked", 130, 60, 0.8),
  steam(1490890, "Demon Slayer -Kimetsu no Yaiba- The Hinokami Chronicles", 128, 700, 24),
  other("epic", "Fortnite", 4, "https://cdn1.epicgames.com/item/fn/FNBR_43-00_MSEE_KeyArt_FN_EGS_Launcher_Blade_1200x1600_1200x1600-6c88e2d63466451ba341e67e08c33dbc?h=900&resize=1&w=600"),
  other("epic", "Rocket League®", 20, `${cdn}/252950/library_600x900.jpg`),
  other("ea", "EA SPORTS FC 26", 6, `${cdn}/3405690/library_600x900.jpg`),
  other("hoyoplay", "Genshin Impact", 60),
  other("wargaming", "World of Tanks EU", 15, `${cdn}/1407200/library_600x900.jpg`),
  other("xbox", "Minecraft for Windows", null),
  other("ankama", "Dofus", null),
];

const library: Library = {
  games,
  sources: [
    ["steam", "Steam"],
    ["epic", "Epic Games"],
    ["ea", "EA app"],
    ["xbox", "Xbox · Microsoft Store"],
    ["battlenet", "Battle.net"],
    ["hoyoplay", "HoYoPlay"],
    ["wargaming", "Wargaming"],
    ["ankama", "Ankama"],
  ].map(([id, name]) => ({
    id,
    name,
    installed: true,
    canOpen: true,
    count: games.filter((g) => g.source === id).length,
    icon: null,
  })),
  scannedAt: now,
  scanning: false,
};

const info: AppInfo = {
  settings: { theme: "system", afterLaunch: "minimize", showTools: false, showAllStores: false, gridSize: "medium", liquidGlass: true, quickShortcut: "ctrl+alt+space" },
  translucent: false,
  country: "FR",
  version: "0.1.0",
  quickShortcutActive: true,
};

const rdr2: StoreHit = {
  key: "red dead redemption 2",
  title: "Red Dead Redemption 2",
  image: `${cdn}/1174180/header.jpg`,
  cover: `${cdn}/1174180/library_600x900.jpg`,
  hero: `${cdn}/1174180/library_hero.jpg`,
  description: "Arthur Morgan et la bande de Van der Linde fuient à travers l'Amérique à l'aube de l'ère moderne.",
  steamAppid: 1174180,
  offers: [
    { store: "instantgaming", storeName: "Instant Gaming", title: "Red Dead Redemption 2", edition: null, price: 12.99, regular: 59.99, discount: 78, currency: "EUR", free: false, url: "https://www.instant-gaming.com", note: "Clé Rockstar" },
    { store: "steam", storeName: "Steam", title: "Red Dead Redemption 2", edition: null, price: 59.99, regular: null, discount: null, currency: "EUR", free: false, url: "https://store.steampowered.com/app/1174180/", note: null },
    { store: "epic", storeName: "Epic Games", title: "Red Dead Redemption 2", edition: null, price: 59.99, regular: null, discount: null, currency: "EUR", free: false, url: "https://store.epicgames.com", note: null },
    { store: "instantgaming", storeName: "Instant Gaming", title: "Red Dead Redemption 2: Ultimate Edition", edition: "Ultimate Edition", price: 17.39, regular: 99.99, discount: 83, currency: "EUR", free: false, url: "https://www.instant-gaming.com", note: "Clé Rockstar" },
  ],
};

const cyberpunk: StoreHit = {
  key: "cyberpunk 2077",
  title: "Cyberpunk 2077",
  image: `${cdn}/1091500/header.jpg`,
  cover: `${cdn}/1091500/library_600x900.jpg`,
  hero: `${cdn}/1091500/library_hero.jpg`,
  description: "Un RPG d'action-aventure en monde ouvert dans la mégalopole de Night City.",
  steamAppid: 1091500,
  offers: [
    { store: "steam", storeName: "Steam", title: "Cyberpunk 2077", edition: null, price: 20.99, regular: 59.99, discount: 65, currency: "EUR", free: false, url: "https://store.steampowered.com/app/1091500/", note: null },
    { store: "instantgaming", storeName: "Instant Gaming", title: "Cyberpunk 2077", edition: null, price: 22.49, regular: 59.99, discount: 63, currency: "EUR", free: false, url: "https://www.instant-gaming.com", note: "Clé GOG.com" },
  ],
};

const details: Details = {
  appid: 1174180,
  name: "Red Dead Redemption 2",
  summary: "Arthur Morgan et la bande de Van der Linde sont des hors-la-loi en fuite. Avec les agents fédéraux et les meilleurs chasseurs de primes à leurs trousses, la bande doit voler, braquer et se battre pour survivre.",
  about: "Amérique, 1899.\n\nAprès un braquage qui a mal tourné dans la ville de Blackwater, Arthur Morgan et la bande de Van der Linde sont contraints de fuir.\n\n• Un monde ouvert vivant\n• Une histoire épique\n• Red Dead Online inclus",
  header: `${cdn}/1174180/header.jpg`,
  screenshots: [1, 2, 3, 4].map((n) => ({
    thumb: `${cdn}/1174180/header.jpg?n=${n}`,
    full: `${cdn}/1174180/header.jpg?n=${n}`,
  })),
  genres: ["Action", "Aventure"],
  features: ["Solo", "Multijoueur", "Succès Steam"],
  developers: ["Rockstar Games"],
  publishers: ["Rockstar Games"],
  releaseDate: "5 déc. 2019",
  comingSoon: false,
  metacritic: 93,
  recommendations: 640000,
  website: "https://www.rockstargames.com/reddeadredemption2/",
  requirements: null,
};

const featured: Featured = {
  specials: [1091500, 1245620, 2050650, 1174180, 805550, 678950].map((appid, i) => ({
    appid,
    title: games.find((g) => g.id === `steam:${appid}`)?.name ?? "Red Dead Redemption 2",
    image: `${cdn}/${appid}/header.jpg`,
    price: 19.99 + i * 3,
    regular: 59.99,
    discount: 60 - i * 5,
    currency: "EUR",
  })),
  topSellers: [1172470, 1245620, 1091500, 2050650].map((appid) => ({
    appid,
    title: games.find((g) => g.id === `steam:${appid}`)?.name ?? "",
    image: `${cdn}/${appid}/header.jpg`,
    price: appid === 1172470 ? 0 : 59.99,
    regular: null,
    discount: 0,
    currency: "EUR",
  })),
  newReleases: [],
};

let collections: Collection[] = [{ id: "c1", name: "Coop entre potes", games: ["steam:285900", "steam:448510", "epic:Rocket League®"] }];

// Coffre de démonstration (le vrai backend ne stocke qu'une empreinte Argon2).
const hiddenIds = new Set<string>(["steam:448510", "steam:285900"]);
let vaultPassword: string | null = null;
let vaultOpen = false;
const shown = (): Library => ({
  ...library,
  games: library.games
    .filter((g) => vaultOpen || !hiddenIds.has(g.id))
    .map((g) => ({ ...g, hidden: hiddenIds.has(g.id) })),
});

export async function handle(command: string, args?: Record<string, unknown>): Promise<unknown> {
  await new Promise((r) => setTimeout(r, command.startsWith("store_") ? 350 : 20));
  switch (command) {
    case "get_library":
    case "rescan_library":
    case "add_custom_game":
    case "remove_custom_game":
      return shown();
    case "set_hidden":
      if (!args?.value && !vaultOpen) throw "Déverrouillez les jeux masqués dans les réglages pour réafficher un jeu.";
      if (args?.value) hiddenIds.add(String(args.id));
      else hiddenIds.delete(String(args?.id));
      return shown();
    case "vault_status":
      return { hasPassword: vaultPassword !== null, unlocked: vaultOpen };
    case "vault_set_password":
      if (vaultPassword !== null && args?.current !== vaultPassword) {
        await new Promise((r) => setTimeout(r, 700));
        throw "Mot de passe actuel incorrect.";
      }
      vaultPassword = String(args?.password);
      vaultOpen = true;
      return shown();
    case "vault_unlock":
      if (args?.password !== vaultPassword) {
        await new Promise((r) => setTimeout(r, 700));
        throw "Mot de passe incorrect.";
      }
      vaultOpen = true;
      return shown();
    case "vault_lock":
      vaultOpen = false;
      return shown();
    case "get_app_info":
      return info;
    case "set_settings":
      Object.assign(info.settings, args?.settings);
      return undefined;
    case "store_search": {
      const q = String(args?.query ?? "").toLowerCase();
      const hits = [rdr2, cyberpunk].filter((h) => h.key.includes(q.trim()) || q.includes(h.key.split(" ")[0]));
      return { hits, failed: [] } satisfies SearchResult;
    }
    case "store_details":
      return { ...details, appid: Number(args?.appid) };
    case "store_offers":
      return { offers: rdr2.offers, links: [{ source: "rockstar", name: "Rockstar Games", url: null }], failed: [] } satisfies OffersResult;
    case "store_featured":
      return featured;
    case "get_collections":
      return collections;
    case "save_collection": {
      const name = String(args?.name ?? "").trim();
      if (!name) throw "Donnez un nom à la collection.";
      if (args?.id) collections = collections.map((c) => (c.id === args.id ? { ...c, name } : c));
      else collections = [...collections, { id: `c${Date.now()}`, name, games: [] }];
      return collections;
    }
    case "delete_collection":
      collections = collections.filter((c) => c.id !== args?.id);
      return collections;
    case "toggle_in_collection": {
      const gameId = String(args?.gameId);
      collections = collections.map((c) =>
        c.id !== args?.id ? c : { ...c, games: c.games.includes(gameId) ? c.games.filter((g) => g !== gameId) : [...c.games, gameId] },
      );
      return collections;
    }
    case "set_custom_art": {
      const game = library.games.find((g) => g.id === args?.id)!;
      return { ...game, customArt: args?.path != null };
    }
    case "storage_info": {
      const games = shown().games.filter((g) => g.installDir);
      return {
        disks: [{ root: "D:\\", label: "Jeux", total: 2000e9, free: 74e9 }],
        games: games
          .map((g, i) => ({ id: g.id, root: "D:\\", size: g.sizeBytes ?? (8 + i * 3) * 1024 ** 3, measured: !g.sizeBytes }))
          .sort((a, b) => b.size - a.size),
      } satisfies StorageInfo;
    }
    default:
      return undefined;
  }
}
