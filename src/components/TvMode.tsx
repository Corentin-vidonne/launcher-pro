// Mode TV : plein écran, grandes jaquettes en rangées, navigation à la manette (ou au clavier).
// Ⓐ jouer · Ⓑ quitter · Ⓨ favori · LB/RB rangée précédente/suivante · croix ou stick pour se déplacer.

import { useEffect, useRef, useState } from "react";

import { playtime, timeAgo } from "../lib/format";
import type { Game, Source } from "../lib/types";
import { Ambient } from "./Ambient";
import { Artwork } from "./GameCard";

export interface TvRow {
  title: string;
  games: Game[];
}

interface Props {
  rows: TvRow[];
  sources: Map<string, Source>;
  onPlay: (id: string) => void;
  onFavorite: (game: Game) => void;
  onExit: () => void;
}

type Dir = "up" | "down" | "left" | "right";

const REPEAT_DELAY = 380;
const REPEAT_EVERY = 110;
const DEADZONE = 0.55;

export function TvMode({ rows, sources, onPlay, onFavorite, onExit }: Props) {
  const [pos, setPos] = useState({ r: 0, c: 0 });
  const [clock, setClock] = useState(() => new Date());

  // Position bornée : la liste des rangées peut changer (favori ajouté, rescan…).
  const r = Math.max(0, Math.min(pos.r, rows.length - 1));
  const row = rows[r];
  const c = row ? Math.max(0, Math.min(pos.c, row.games.length - 1)) : 0;
  const game = row?.games[c];

  // Les écouteurs (clavier, manette) sont posés une seule fois et lisent l'état courant ici.
  const live = useRef({ rows, game, onPlay, onFavorite, onExit });
  live.current = { rows, game, onPlay, onFavorite, onExit };

  const move = (dir: Dir) =>
    setPos((p) => {
      const rs = live.current.rows;
      if (!rs.length) return p;
      const pr = Math.max(0, Math.min(p.r, rs.length - 1));
      const pc = Math.max(0, Math.min(p.c, rs[pr].games.length - 1));
      if (dir === "left") return { r: pr, c: Math.max(0, pc - 1) };
      if (dir === "right") return { r: pr, c: Math.min(rs[pr].games.length - 1, pc + 1) };
      const nr = dir === "up" ? Math.max(0, pr - 1) : Math.min(rs.length - 1, pr + 1);
      return { r: nr, c: nr === pr ? pc : 0 };
    });

  const play = () => live.current.game && live.current.onPlay(live.current.game.id);
  const favorite = () => live.current.game && live.current.onFavorite(live.current.game);

  useEffect(() => {
    const timer = setInterval(() => setClock(new Date()), 15000);
    return () => clearInterval(timer);
  }, []);

  // Clavier (capture : rien d'autre ne doit réagir pendant le mode TV).
  useEffect(() => {
    const keys: Record<string, Dir> = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right" };
    const onKey = (e: KeyboardEvent) => {
      if (keys[e.key]) move(keys[e.key]);
      else if (e.key === "Enter" || e.key === " ") play();
      else if (e.key === "Escape" || e.key === "Backspace") live.current.onExit();
      else if (e.key.toLowerCase() === "f") favorite();
      else return;
      e.preventDefault();
      e.stopPropagation();
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Manette : lecture à chaque image tant que le mode TV est ouvert et la fenêtre active
  // (un jeu lancé passe au premier plan : la manette ne pilote alors plus le launcher).
  useEffect(() => {
    let frame = 0;
    const down = new Set<number>();
    const next: Partial<Record<Dir, number>> = {};
    let primed = false;
    const loop = (t: number) => {
      frame = requestAnimationFrame(loop);
      if (!document.hasFocus()) return;
      const pad = [...navigator.getGamepads()].find((p): p is Gamepad => !!p && p.connected);
      if (!pad) return;
      const held = (i: number) => !!pad.buttons[i]?.pressed;
      const pressed = (i: number) => {
        const on = held(i);
        const was = down.has(i);
        if (on) down.add(i);
        else down.delete(i);
        return on && !was;
      };
      if (!primed) {
        // Le bouton qui a ouvert le mode TV (Start) ne doit rien déclencher ici.
        [0, 1, 3, 4, 5].forEach((i) => held(i) && down.add(i));
        primed = true;
        return;
      }
      if (pressed(0)) play();
      if (pressed(1)) live.current.onExit();
      if (pressed(3)) favorite();
      if (pressed(4)) move("up");
      if (pressed(5)) move("down");
      const ax = pad.axes[0] ?? 0;
      const ay = pad.axes[1] ?? 0;
      const dirs: Record<Dir, boolean> = {
        up: held(12) || ay < -DEADZONE,
        down: held(13) || ay > DEADZONE,
        left: held(14) || ax < -DEADZONE,
        right: held(15) || ax > DEADZONE,
      };
      for (const d of Object.keys(dirs) as Dir[]) {
        if (!dirs[d]) {
          delete next[d];
        } else if (next[d] === undefined) {
          move(d);
          next[d] = t + REPEAT_DELAY;
        } else if (t >= next[d]!) {
          move(d);
          next[d] = t + REPEAT_EVERY;
        }
      }
    };
    frame = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(frame);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const time = clock.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" });

  return (
    <div className="tv" role="application" aria-label="Mode TV">
      <Ambient src={game ? (game.hero ?? game.cover) : null} blurred={!!game && !game.hero} />
      <div className="tv-shade" />

      <header className="tv-top">
        <span className="tv-brand">Launcher Pro</span>
        <span className="tv-clock tabular">{time}</span>
      </header>

      {game ? (
        <section className="tv-hero" key={game.id}>
          <div className="tv-eyebrow">{row?.title}</div>
          {game.logo ? <img className="tv-logo" src={game.logo} alt={game.name} /> : <h1 className="tv-title">{game.name}</h1>}
          <p className="tv-meta">
            {[
              sources.get(game.source)?.name,
              timeAgo(game.lastPlayed) && `joué ${timeAgo(game.lastPlayed)}`,
              playtime(game.playtimeMin) && `${playtime(game.playtimeMin)} de jeu`,
              game.favorite && "♥ Favori",
            ]
              .filter(Boolean)
              .join("  ·  ")}
          </p>
        </section>
      ) : (
        <section className="tv-hero">
          <h1 className="tv-title">Aucun jeu à lancer</h1>
        </section>
      )}

      <div className="tv-rows">
        <div className="tv-rows-inner" style={{ transform: `translateY(calc(${-r} * var(--tv-row)))` }}>
          {rows.map((rw, ri) => (
            <div key={rw.title} className={ri === r ? "tv-row on" : ri < r ? "tv-row gone" : "tv-row"}>
              <h3>{rw.title}</h3>
              <div
                className="tv-track"
                style={{
                  transform: `translateX(calc(${-(ri === r ? Math.max(0, c - 1) : 0)} * (var(--tv-card) + var(--tv-gap))))`,
                }}
              >
                {rw.games.map((g, gi) => (
                  <button
                    key={g.id}
                    className={ri === r && gi === c ? "tv-card on" : "tv-card"}
                    tabIndex={-1}
                    onClick={() => (ri === r && gi === c ? onPlay(g.id) : setPos({ r: ri, c: gi }))}
                  >
                    <Artwork key={g.cover ?? "tile"} name={g.name} cover={g.cover} icon={g.icon} />
                  </button>
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>

      <footer className="tv-hints">
        <span>
          <b className="pad a">A</b> Jouer
        </span>
        <span>
          <b className="pad y">Y</b> Favori
        </span>
        <span>
          <b className="pad lb">LB</b>
          <b className="pad lb">RB</b> Rangée
        </span>
        <span>
          <b className="pad b">B</b> Quitter
        </span>
      </footer>
    </div>
  );
}
