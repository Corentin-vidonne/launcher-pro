import { useCallback, useEffect, useMemo, useState } from "react";

import { api } from "../lib/api";
import { fileSize } from "../lib/format";
import type { Game, Source, StorageInfo } from "../lib/types";
import { Artwork } from "./GameCard";
import { IconDisk, IconFolder, IconRefresh } from "./Icons";
import { Page } from "./Shell";

interface Props {
  games: Game[];
  sources: Map<string, Source>;
  onOpen: (id: string) => void;
  onReveal: (id: string) => void;
}

const pct = (part: number, total: number) => `${total > 0 ? Math.max(0, Math.min(100, (part / total) * 100)) : 0}%`;

/** Place prise par les jeux, disque par disque, du plus gros au plus petit. */
export function StorageView({ games, sources, onOpen, onReveal }: Props) {
  const [info, setInfo] = useState<StorageInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    setInfo(null);
    setError(null);
    api
      .storage()
      .then(setInfo)
      .catch((e) => setError(String(e)));
  }, []);
  useEffect(load, [load]);

  const byId = useMemo(() => new Map(games.map((g) => [g.id, g])), [games]);

  const actions = (
    <button className="icon-btn" title="Recalculer" aria-label="Recalculer" onClick={load}>
      <IconRefresh />
    </button>
  );

  return (
    <Page actions={actions}>
      <div className="page-head">
        <div className="grow">
          <h1 className="page-title">Stockage</h1>
          <p className="page-sub">Place occupée par vos jeux sur chaque disque.</p>
        </div>
      </div>

      {error && (
        <div className="empty">
          <IconDisk />
          <strong>Analyse impossible</strong>
          <p>{error}</p>
        </div>
      )}

      {!info && !error && (
        <div className="storage">
          {[0, 1].map((i) => (
            <div className="disk" key={i}>
              <div className="skeleton" style={{ height: 22, width: "30%" }} />
              <div className="skeleton" style={{ height: 12, marginTop: 16 }} />
              <div className="skeleton" style={{ height: 120, marginTop: 18 }} />
            </div>
          ))}
          <p className="note">Mesure des dossiers des jeux dont le launcher ne donne pas la taille…</p>
        </div>
      )}

      {info && (
        <div className="storage">
          {info.disks.map((disk) => {
            const list = info.games.filter((g) => g.root === disk.root && byId.has(g.id));
            const gamesSize = list.reduce((sum, g) => sum + g.size, 0);
            const used = disk.total - disk.free;
            const other = Math.max(0, used - gamesSize);
            const biggest = list[0]?.size ?? 1;
            return (
              <section className="disk" key={disk.root}>
                <div className="disk-head">
                  <span className="disk-icon">
                    <IconDisk />
                  </span>
                  <div className="grow">
                    <h2>
                      {disk.label || "Disque local"} <span>({disk.root.replace("\\", "")})</span>
                    </h2>
                    <p>
                      {fileSize(disk.free)} libres sur {fileSize(disk.total)}
                    </p>
                  </div>
                </div>
                <div className="disk-bar" role="img" aria-label={`Jeux ${fileSize(gamesSize)}, autres fichiers ${fileSize(other)}`}>
                  <i className="games" style={{ width: pct(gamesSize, disk.total) }} />
                  <i className="other" style={{ width: pct(other, disk.total) }} />
                </div>
                <div className="disk-legend">
                  <span>
                    <i className="games" />
                    Jeux · {fileSize(gamesSize) ?? "0 Mo"}
                  </span>
                  <span>
                    <i className="other" />
                    Autres fichiers · {fileSize(other) ?? "0 Mo"}
                  </span>
                  <span>
                    <i className="free" />
                    Libre · {fileSize(disk.free)}
                  </span>
                </div>

                <div className="disk-games">
                  {list.map((entry) => {
                    const game = byId.get(entry.id)!;
                    return (
                      <div className="disk-game" key={entry.id}>
                        <button className="mini" onClick={() => onOpen(game.id)} aria-label={`Ouvrir ${game.name}`}>
                          <Artwork key={game.cover ?? "tile"} name={game.name} cover={game.cover} icon={game.icon} />
                        </button>
                        <div className="text">
                          <strong>{game.name}</strong>
                          <small>{sources.get(game.source)?.name ?? game.source}</small>
                        </div>
                        <div className="size-bar">
                          <i style={{ width: pct(entry.size, biggest) }} />
                        </div>
                        <span className="size tabular">{fileSize(entry.size) ?? "—"}</span>
                        {game.installDir && (
                          <button className="icon-btn" title="Ouvrir le dossier" aria-label="Ouvrir le dossier" onClick={() => onReveal(game.id)}>
                            <IconFolder />
                          </button>
                        )}
                      </div>
                    );
                  })}
                </div>
              </section>
            );
          })}
          {info.disks.length === 0 && (
            <div className="empty">
              <IconDisk />
              <strong>Aucun jeu localisé</strong>
              <p>Les jeux dont le dossier d'installation est connu apparaîtront ici.</p>
            </div>
          )}
        </div>
      )}
    </Page>
  );
}
