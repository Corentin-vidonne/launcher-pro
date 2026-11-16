import { memo, useState, type CSSProperties } from "react";

import { hueOf } from "../lib/format";
import type { Game } from "../lib/types";
import { IconHeartFill, IconPlay } from "./Icons";

/** Tuile de repli quand aucune jaquette n'existe : icône du jeu (ou initiales) sur un dégradé. */
export function Tile({ name, icon }: { name: string; icon: string | null }) {
  const initials = name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((word) => word[0])
    .join("")
    .toUpperCase();
  return (
    <div className="tile" style={{ "--hue": hueOf(name) } as CSSProperties}>
      {icon ? <img src={icon} alt="" loading="lazy" /> : <span className="initials">{initials}</span>}
    </div>
  );
}

/** Jaquette avec apparition en fondu ; retombe sur la tuile si l'image manque ou échoue. */
export function Artwork({ name, cover, icon }: { name: string; cover: string | null; icon: string | null }) {
  const [failed, setFailed] = useState(false);
  if (!cover || failed) return <Tile name={name} icon={icon} />;
  return (
    <img
      className="cover"
      src={cover}
      alt=""
      loading="lazy"
      decoding="async"
      // Classe posée directement sur le nœud : pas de rendu React par image chargée.
      ref={(el) => {
        if (el?.complete && el.naturalWidth > 0) el.classList.add("ready");
      }}
      onLoad={(e) => e.currentTarget.classList.add("ready")}
      onError={() => setFailed(true)}
    />
  );
}

interface Props {
  game: Game;
  subtitle: string;
  onOpen: (id: string) => void;
  onPlay: (id: string) => void;
  onMenu: (id: string, x: number, y: number) => void;
  /** Survol ou focus : l'arrière-plan de l'application épouse l'artwork du jeu. */
  onHover: (id: string | null) => void;
}

export const GameCard = memo(function GameCard({ game, subtitle, onOpen, onPlay, onMenu, onHover }: Props) {
  return (
    <div
      className="card"
      role="button"
      tabIndex={0}
      onMouseEnter={() => onHover(game.id)}
      onMouseLeave={() => onHover(null)}
      onFocus={() => onHover(game.id)}
      onClick={() => onOpen(game.id)}
      onKeyDown={(e) => {
        if (e.key === "Enter") onOpen(game.id);
      }}
      onContextMenu={(e) => {
        e.preventDefault();
        onMenu(game.id, e.clientX, e.clientY);
      }}
    >
      <div className="card-art">
        <Artwork key={game.cover ?? "tile"} name={game.name} cover={game.cover} icon={game.icon} />
        {game.favorite && (
          <span className="card-fav">
            <IconHeartFill />
          </span>
        )}
        {game.canLaunch && (
          <button
            className="card-play"
            aria-label={`Jouer à ${game.name}`}
            onClick={(e) => {
              e.stopPropagation();
              onPlay(game.id);
            }}
          >
            <IconPlay />
          </button>
        )}
      </div>
      <div className="card-title">{game.name}</div>
      <div className="card-sub">{subtitle}</div>
    </div>
  );
});
