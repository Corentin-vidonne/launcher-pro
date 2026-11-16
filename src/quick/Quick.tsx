// Recherche rapide : fenêtre flottante ouverte par le raccourci global. Taper, ↑↓, Entrée : le jeu
// se lance et la fenêtre disparaît. Page volontairement légère (pas de chargement de l'app entière).

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { LogicalSize, getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties } from "react";

import { api, inApp } from "../lib/api";
import { hueOf, timeAgo } from "../lib/format";
import type { Game, Library } from "../lib/types";

const MAX = 7;

const fold = (s: string) =>
  s
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ™®©]/g, "");

/** Pertinence : début du titre > début d'un mot > initiales (« rdr ») > contenu > lettres dans l'ordre. */
function score(name: string, q: string): number {
  if (name.startsWith(q)) return 100;
  const words = name.split(/[^\p{L}\p{N}]+/u).filter(Boolean);
  if (words.some((w) => w.startsWith(q))) return 80;
  if (words.map((w) => w[0]).join("").startsWith(q)) return 60;
  if (name.includes(q)) return 50;
  let i = 0;
  for (const c of name) if (c === q[i]) i++;
  return i === q.length ? 20 : 0;
}

function hide() {
  if (inApp) void invoke("quick_hide");
}

function Thumb({ game }: { game: Game }) {
  const src = game.cover ?? game.icon;
  return (
    <span className="thumb" style={{ "--hue": hueOf(game.name) } as CSSProperties}>
      {src ? <img src={src} alt="" /> : game.name.charAt(0)}
    </span>
  );
}

export function Quick() {
  const [library, setLibrary] = useState<Library | null>(null);
  const [query, setQuery] = useState("");
  const [index, setIndex] = useState(0);
  const [shortcut, setShortcut] = useState("");
  const input = useRef<HTMLInputElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const openedAt = useRef(0);

  const load = useCallback(() => {
    api.library().then(setLibrary).catch(() => {});
    api
      .appInfo()
      .then((info) => {
        const t = info.settings.theme;
        document.documentElement.dataset.theme =
          t === "system" ? (matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light") : t;
        setShortcut(info.settings.quickShortcut);
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    load();
    openedAt.current = Date.now();
    input.current?.focus();
    if (!inApp) return;
    const win = getCurrentWindow();
    void win.show().then(() => win.setFocus());
    const offs = [
      listen("quick-open", () => {
        openedAt.current = Date.now();
        setQuery("");
        setIndex(0);
        load();
        requestAnimationFrame(() => input.current?.focus());
      }),
      // Clic ailleurs : la fenêtre s'efface (sauf le bref instant de son apparition).
      win.onFocusChanged(({ payload }) => {
        if (!payload && Date.now() - openedAt.current > 400) hide();
      }),
    ];
    return () => offs.forEach((off) => void off.then((f) => f()));
  }, [load]);

  const results = useMemo(() => {
    const games = (library?.games ?? []).filter((g) => !g.hidden && !g.isTool && g.canLaunch);
    const q = fold(query.trim());
    if (!q) return [...games].filter((g) => g.lastPlayed).sort((a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0)).slice(0, MAX);
    return games
      .map((g) => ({ g, s: score(fold(g.name), q) }))
      .filter((x) => x.s > 0)
      .sort((a, b) => b.s - a.s || a.g.name.localeCompare(b.g.name, "fr"))
      .slice(0, MAX)
      .map((x) => x.g);
  }, [library, query]);

  const rows = results.length + (query.trim() ? 1 : 0);
  const sources = useMemo(() => new Map(library?.sources.map((s) => [s.id, s.name]) ?? []), [library]);

  useEffect(() => setIndex(0), [query]);

  // La fenêtre épouse la hauteur du contenu (pas de grand vide sous trois résultats).
  useLayoutEffect(() => {
    if (!inApp || !panel.current) return;
    const height = Math.ceil(panel.current.getBoundingClientRect().height);
    void getCurrentWindow().setSize(new LogicalSize(680, height));
  }, [rows, library]);

  const activate = (i: number) => {
    const game = results[i];
    if (game) {
      api
        .launch(game.id)
        .then(hide)
        .catch(() => {});
    } else if (query.trim() && inApp) {
      void invoke("quick_store", { query: query.trim() });
    }
  };

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setIndex((i) => (rows ? (i + 1) % rows : 0));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setIndex((i) => (rows ? (i - 1 + rows) % rows : 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      activate(index);
    } else if (e.key === "Escape") {
      e.preventDefault();
      if (query) setQuery("");
      else hide();
    }
  };

  const label = shortcut
    .split("+")
    .map((k) => ({ ctrl: "Ctrl", alt: "Alt", shift: "Maj", space: "Espace", super: "Win" })[k.toLowerCase()] ?? k.toUpperCase())
    .join(" + ");

  return (
    <div className="q" ref={panel}>
      <div className="q-search">
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <circle cx="10.8" cy="10.8" r="6.3" />
          <path d="m15.6 15.6 4.4 4.4" />
        </svg>
        <input
          ref={input}
          value={query}
          placeholder="Lancer un jeu…"
          spellCheck={false}
          autoFocus
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={onKey}
        />
        <kbd>Échap</kbd>
      </div>

      {rows > 0 && (
        <div className="q-list" role="listbox">
          {!query.trim() && <div className="q-section">Récemment joués</div>}
          {results.map((game, i) => (
            <button
              key={game.id}
              role="option"
              aria-selected={i === index}
              className={i === index ? "q-row on" : "q-row"}
              onMouseMove={() => setIndex(i)}
              onClick={() => activate(i)}
            >
              <Thumb game={game} />
              <span className="q-name">
                <b>{game.name}</b>
                <small>
                  {[sources.get(game.source), timeAgo(game.lastPlayed) && `joué ${timeAgo(game.lastPlayed)}`]
                    .filter(Boolean)
                    .join(" · ")}
                </small>
              </span>
              <span className="q-hint">Jouer ↵</span>
            </button>
          ))}
          {query.trim() && (
            <button
              className={index === results.length ? "q-row on" : "q-row"}
              onMouseMove={() => setIndex(results.length)}
              onClick={() => activate(results.length)}
            >
              <span className="thumb store">
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M5 8h14l-1 11.2a2 2 0 0 1-2 1.8H8a2 2 0 0 1-2-1.8L5 8Z" />
                  <path d="M9 8V6.5a3 3 0 0 1 6 0V8" />
                </svg>
              </span>
              <span className="q-name">
                <b>Chercher « {query.trim()} » dans le magasin</b>
                <small>Comparer les prix des boutiques</small>
              </span>
              <span className="q-hint">↵</span>
            </button>
          )}
        </div>
      )}

      <div className="q-foot">
        <span>↑↓ choisir</span>
        <span>↵ lancer</span>
        {label && <span className="q-shortcut">{label}</span>}
      </div>
    </div>
  );
}
