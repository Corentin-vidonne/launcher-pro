import { fileSize, playtime, timeAgo } from "../lib/format";
import type { Collection, Game, Source } from "../lib/types";
import { Artwork } from "./GameCard";
import {
  IconBag,
  IconCheck,
  IconEye,
  IconEyeOff,
  IconFolder,
  IconHeart,
  IconHeartFill,
  IconImage,
  IconPlay,
  IconPlus,
  IconTrash,
} from "./Icons";
import { Sheet } from "./Shell";

interface Props {
  game: Game;
  source: Source | undefined;
  onClose: () => void;
  onPlay: (id: string) => void;
  onFavorite: (game: Game) => void;
  onHidden: (game: Game) => void;
  onReveal: (id: string) => void;
  onOpenSource: (id: string) => void;
  onStore: (name: string) => void;
  onRemove: (id: string) => void;
  collections: Collection[];
  onToggleCollection: (collectionId: string, gameId: string) => void;
  onNewCollection: (gameId: string) => void;
  onChangeArt: (game: Game, kind: "cover" | "hero") => void;
  onResetArt: (game: Game) => void;
}

export function GameSheet({ game, source, onClose, ...actions }: Props) {
  const backdrop = game.hero ?? game.cover;
  const facts: [string, string][] = [
    ["Source", source?.name ?? game.source],
    ["Dernière session", timeAgo(game.lastPlayed) ?? "Jamais"],
  ];
  const time = playtime(game.playtimeMin);
  if (time) facts.push(["Temps de jeu", time]);
  const size = fileSize(game.sizeBytes);
  if (size) facts.push(["Taille", size]);

  return (
    <Sheet onClose={onClose}>
      <div className="sheet-hero">
        {backdrop && <img className={game.hero ? "bg" : "bg blurred"} src={backdrop} alt="" />}
        <div className="sheet-head">
          <div className="sheet-cover">
            <Artwork key={game.cover ?? "tile"} name={game.name} cover={game.cover} icon={game.icon} />
          </div>
          <div className="sheet-headings">
            <h2 className="sheet-title">{game.name}</h2>
            <div className="sheet-chips">
              <span className="chip">
                {source?.icon && <img src={source.icon} alt="" />}
                {source?.name ?? game.source}
              </span>
              {game.isTool && <span className="chip">Logiciel</span>}
              {game.hidden && <span className="chip">Masqué</span>}
            </div>
          </div>
        </div>
      </div>

      <div className="sheet-body">
        <div className="sheet-actions">
          <button
            className="btn large primary"
            disabled={!game.canLaunch}
            onClick={() => {
              actions.onPlay(game.id);
              onClose();
            }}
          >
            <IconPlay />
            Jouer
          </button>
          <button
            className="btn large round"
            title={game.favorite ? "Retirer des favoris" : "Ajouter aux favoris"}
            aria-label="Favori"
            style={game.favorite ? { color: "var(--pink)" } : undefined}
            onClick={() => actions.onFavorite(game)}
          >
            {game.favorite ? <IconHeartFill /> : <IconHeart />}
          </button>
          <button
            className="btn large round"
            title={game.hidden ? "Réafficher dans la bibliothèque" : "Masquer (protégé par le mot de passe des réglages)"}
            aria-label={game.hidden ? "Réafficher" : "Masquer"}
            onClick={() => actions.onHidden(game)}
          >
            {game.hidden ? <IconEye /> : <IconEyeOff />}
          </button>
          <span style={{ flex: 1 }} />
          {game.source !== "custom" && (
            <button className="btn" onClick={() => actions.onStore(game.name)}>
              <IconBag />
              Voir dans le magasin
            </button>
          )}
          {source?.canOpen && (
            <button className="btn" onClick={() => actions.onOpenSource(source.id)}>
              Ouvrir {source.name}
            </button>
          )}
          {game.source === "custom" && (
            <button
              className="btn"
              style={{ color: "var(--red)" }}
              onClick={() => {
                actions.onRemove(game.id);
                onClose();
              }}
            >
              <IconTrash />
              Retirer
            </button>
          )}
        </div>

        <dl className="facts">
          {facts.map(([label, value]) => (
            <div className="fact" key={label}>
              <dt>{label}</dt>
              <dd>{value}</dd>
            </div>
          ))}
        </dl>

        {game.installDir && (
          <div className="path-row">
            <div className="path">
              <small>Emplacement</small>
              <span title={game.installDir}>{game.installDir}</span>
            </div>
            <button className="btn" onClick={() => actions.onReveal(game.id)}>
              <IconFolder />
              Ouvrir
            </button>
          </div>
        )}

        <h3 className="subhead">Collections</h3>
        <div className="tags">
          {actions.collections.map((c) => {
            const inside = c.games.includes(game.id);
            return (
              <button key={c.id} className={inside ? "tag on" : "tag"} onClick={() => actions.onToggleCollection(c.id, game.id)}>
                {inside && <IconCheck />}
                {c.name}
              </button>
            );
          })}
          <button className="tag add" onClick={() => actions.onNewCollection(game.id)}>
            <IconPlus />
            Nouvelle collection
          </button>
        </div>

        <h3 className="subhead">Images</h3>
        <div className="tags">
          <button className="tag" onClick={() => actions.onChangeArt(game, "cover")}>
            <IconImage />
            Changer la jaquette
          </button>
          <button className="tag" onClick={() => actions.onChangeArt(game, "hero")}>
            <IconImage />
            Changer le fond
          </button>
          {game.customArt && (
            <button className="tag" onClick={() => actions.onResetArt(game)}>
              Rétablir les images d'origine
            </button>
          )}
        </div>
      </div>
    </Sheet>
  );
}
