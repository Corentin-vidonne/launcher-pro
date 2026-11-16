import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";

import { playtime, timeAgo } from "../lib/format";
import type { Game, Source } from "../lib/types";
import { GameCard } from "./GameCard";
import { IconClose, IconController, IconPlay, IconRefresh, IconSearch } from "./Icons";
import { Page } from "./Shell";

type Sort = "name" | "recent" | "playtime";

interface Props {
  title: string;
  games: Game[];
  sources: Map<string, Source>;
  /** Afficher la mise en avant au-dessus du panneau (vue d'ensemble uniquement). */
  hero: boolean;
  /** Jeu mis en avant : le dernier joué, ou celui que la souris survole. */
  spotlight: Game | null;
  /** Le jeu mis en avant est bien le dernier joué (« Reprendre »). */
  resume: boolean;
  /** Tri imposé (vue « Récents ») : masque le sélecteur. */
  fixedSort?: Sort;
  /** Dans la vue d'une source, le sous-titre des cartes n'a pas à la répéter. */
  showSource: boolean;
  empty: { title: string; text: string };
  extra?: ReactNode;
  scanning: boolean;
  onRescan: () => void;
  onOpen: (id: string) => void;
  onPlay: (id: string) => void;
  onMenu: (id: string, x: number, y: number) => void;
  onHover: (id: string | null) => void;
}

const fold = (s: string) =>
  s
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "");

export function LibraryView(props: Props) {
  const { title, games, sources, hero, spotlight, resume, fixedSort, showSource, empty, extra, scanning } = props;
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<Sort>("name");
  const input = useRef<HTMLInputElement>(null);
  const activeSort = fixedSort ?? sort;

  // Ctrl+F : recherche dans la bibliothèque.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        input.current?.focus();
        input.current?.select();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const shown = useMemo(() => {
    const q = fold(query.trim());
    const list = q ? games.filter((g) => fold(g.name).includes(q)) : [...games];
    const byName = (a: Game, b: Game) => a.name.localeCompare(b.name, "fr", { sensitivity: "base" });
    if (activeSort === "recent") {
      list.sort((a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0) || byName(a, b));
    } else if (activeSort === "playtime") {
      list.sort((a, b) => (b.playtimeMin ?? 0) - (a.playtimeMin ?? 0) || byName(a, b));
    } else {
      list.sort(byName);
    }
    return list;
  }, [games, query, activeSort]);

  const subtitle = (g: Game) => {
    const ago = timeAgo(g.lastPlayed);
    if (!showSource) return ago ? `Joué ${ago}` : "Jamais lancé";
    const name = sources.get(g.source)?.name ?? g.source;
    return activeSort === "playtime" && g.playtimeMin ? `${name} · ${playtime(g.playtimeMin)}` : name;
  };

  const actions = (
    <>
      {extra}
      {!fixedSort && games.length > 1 && (
        <div className="segmented" role="tablist" aria-label="Tri">
          {(
            [
              ["name", "Nom"],
              ["recent", "Récents"],
              ["playtime", "Temps de jeu"],
            ] as const
          ).map(([value, label]) => (
            <button key={value} className={sort === value ? "on" : undefined} onClick={() => setSort(value)}>
              {label}
            </button>
          ))}
        </div>
      )}
      <label className="search">
        <IconSearch />
        <input
          ref={input}
          value={query}
          placeholder="Rechercher"
          spellCheck={false}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Escape" && setQuery("")}
        />
        {query && (
          <button className="clear" aria-label="Effacer" onClick={() => setQuery("")}>
            <IconClose />
          </button>
        )}
      </label>
      <button
        className={scanning ? "icon-btn spinning" : "icon-btn"}
        title="Rescanner le PC"
        aria-label="Rescanner le PC"
        onClick={props.onRescan}
      >
        <IconRefresh />
      </button>
    </>
  );

  const heroNode =
    hero && spotlight && !query ? (
      <section className="spot" key={spotlight.id}>
        <div className="spot-eyebrow">{resume ? "Reprendre" : (sources.get(spotlight.source)?.name ?? "Bibliothèque")}</div>
        {spotlight.logo ? (
          <img className="spot-logo" src={spotlight.logo} alt={spotlight.name} />
        ) : (
          <h2 className="spot-title">{spotlight.name}</h2>
        )}
        <div className="spot-meta">
          {[
            sources.get(spotlight.source)?.name,
            timeAgo(spotlight.lastPlayed) && `joué ${timeAgo(spotlight.lastPlayed)}`,
            playtime(spotlight.playtimeMin) && `${playtime(spotlight.playtimeMin)} de jeu`,
          ]
            .filter(Boolean)
            .join(" · ")}
        </div>
        <div className="spot-actions">
          {spotlight.canLaunch && (
            <button className="btn large white" onClick={() => props.onPlay(spotlight.id)}>
              <IconPlay />
              Jouer
            </button>
          )}
          <button className="btn large frost" onClick={() => props.onOpen(spotlight.id)}>
            Détails
          </button>
        </div>
      </section>
    ) : null;

  return (
    <Page actions={actions} hero={heroNode}>
      <div className="page-head">
        <div className="grow">
          <h1 className="page-title">{title}</h1>
          <p className="page-sub">
            {games.length === 0 ? "Aucun jeu" : games.length === 1 ? "1 jeu" : `${games.length} jeux`}
            {scanning && " · analyse du PC…"}
          </p>
        </div>
      </div>

      {shown.length > 0 ? (
        <div className="grid">
          {shown.map((game) => (
            <GameCard
              key={game.id}
              game={game}
              subtitle={subtitle(game)}
              onOpen={props.onOpen}
              onPlay={props.onPlay}
              onMenu={props.onMenu}
              onHover={props.onHover}
            />
          ))}
        </div>
      ) : (
        <div className="empty">
          <IconController />
          <strong>{query ? "Aucun résultat" : empty.title}</strong>
          <p>{query ? `Aucun jeu ne correspond à « ${query} ».` : empty.text}</p>
        </div>
      )}
    </Page>
  );
}
