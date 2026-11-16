import { memo, useMemo, useRef, type ReactNode } from "react";

import { useLens } from "../lib/lens";
import type { Collection, Library, View } from "../lib/types";
import {
  AppLogo,
  IconBag,
  IconClock,
  IconCollection,
  IconDisk,
  IconEyeOff,
  IconGear,
  IconGrid,
  IconHeart,
  IconPlus,
  IconTv,
} from "./Icons";

interface Props {
  view: View;
  library: Library;
  counts: { all: number; favorites: number; hidden: number };
  collections: Collection[];
  /** Coffre déverrouillé dans les réglages : l'onglet « Masqués » apparaît. */
  vaultOpen: boolean;
  onNavigate: (view: View) => void;
  onNewCollection: () => void;
  onCollectionMenu: (id: string, x: number, y: number) => void;
  onTv: () => void;
}

function sameView(a: View, b: View): boolean {
  if (a.kind !== b.kind) return false;
  if (a.kind === "library" && b.kind === "library") return a.filter === b.filter;
  if (a.kind === "source" && b.kind === "source") return a.source === b.source;
  if (a.kind === "collection" && b.kind === "collection") return a.id === b.id;
  return true;
}

export const Sidebar = memo(function Sidebar({
  view,
  library,
  counts,
  collections,
  vaultOpen,
  onNavigate,
  onNewCollection,
  onCollectionMenu,
  onTv,
}: Props) {
  const panel = useRef<HTMLElement>(null);
  useLens(panel, { r: 30, bezel: 24 });

  // Jeux présents (et non masqués) : une collection ne compte que ce qui est encore installé.
  const present = useMemo(() => new Set(library.games.filter((g) => !g.hidden).map((g) => g.id)), [library.games]);

  const item = (target: View, icon: ReactNode, label: string, count?: number) => (
    <button
      key={label}
      className={sameView(view, target) ? "nav-item active" : "nav-item"}
      onClick={() => onNavigate(target)}
    >
      {icon}
      <span className="name">{label}</span>
      {count !== undefined && count > 0 && <span className="count">{count}</span>}
    </button>
  );

  return (
    <aside ref={panel} className="sidebar glass">
      <div className="brand" data-tauri-drag-region>
        <AppLogo />
        Launcher Pro
      </div>

      <nav className="sidebar-scroll">
        <div className="nav-title">Bibliothèque</div>
        {item({ kind: "library", filter: "all" }, <IconGrid />, "Tous les jeux", counts.all)}
        {item({ kind: "library", filter: "favorites" }, <IconHeart />, "Favoris", counts.favorites)}
        {item({ kind: "library", filter: "recent" }, <IconClock />, "Récents")}
        {vaultOpen && item({ kind: "library", filter: "hidden" }, <IconEyeOff />, "Masqués", counts.hidden)}
        {item({ kind: "storage" }, <IconDisk />, "Stockage")}

        <div className="nav-title">Collections</div>
        {collections.map((c) => (
          <button
            key={c.id}
            className={sameView(view, { kind: "collection", id: c.id }) ? "nav-item active" : "nav-item"}
            onClick={() => onNavigate({ kind: "collection", id: c.id })}
            onContextMenu={(e) => {
              e.preventDefault();
              onCollectionMenu(c.id, e.clientX, e.clientY);
            }}
          >
            <IconCollection />
            <span className="name">{c.name}</span>
            <span className="count">{c.games.filter((id) => present.has(id)).length || ""}</span>
          </button>
        ))}
        <button className="nav-item subtle" onClick={() => onNewCollection()}>
          <IconPlus />
          <span className="name">Nouvelle collection</span>
        </button>

        <div className="nav-title">Magasin</div>
        {item({ kind: "store" }, <IconBag />, "Découvrir")}

        {library.sources.length > 0 && <div className="nav-title">Sources</div>}
        {library.sources.map((source) =>
          item(
            { kind: "source", source: source.id },
            source.icon ? (
              <img src={source.icon} alt="" />
            ) : (
              <span className="glyph">{source.name.charAt(0)}</span>
            ),
            source.name,
            source.count,
          ),
        )}
      </nav>

      <div className="sidebar-foot">
        <button className="nav-item" onClick={onTv} title="Plein écran, navigation à la manette (ou bouton Start)">
          <IconTv />
          <span className="name">Mode TV</span>
        </button>
        {item({ kind: "settings" }, <IconGear />, "Réglages")}
      </div>
    </aside>
  );
});
