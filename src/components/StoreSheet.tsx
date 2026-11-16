import { useEffect, useState } from "react";

import { api } from "../lib/api";
import { money } from "../lib/format";
import type { Details, Game, LinkStore, Offer, Source, StoreHit } from "../lib/types";
import { IconArrowOut, IconPlay } from "./Icons";
import { Sheet } from "./Shell";
import { bestOffer, priceLabel, StoreIcon } from "./StoreView";

interface Props {
  hit: StoreHit;
  sources: Map<string, Source>;
  /** Jeu déjà installé correspondant à cette fiche, s'il y en a un. */
  owned: Game | undefined;
  onClose: () => void;
  onPlay: (id: string) => void;
  onOpenSource: (id: string) => void;
}

export function StoreSheet({ hit, sources, owned, onClose, onPlay, onOpenSource }: Props) {
  const [details, setDetails] = useState<Details | null>(null);
  const [offers, setOffers] = useState<Offer[]>(hit.offers);
  const [links, setLinks] = useState<LinkStore[]>([]);
  const [loadingOffers, setLoadingOffers] = useState(true);
  const [expanded, setExpanded] = useState(false);
  const [zoom, setZoom] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    // Prix : recherche ciblée sur le titre exact (plus complète que la liste de résultats).
    const loadOffers = (publishers: string[]) =>
      api
        .storeOffers(hit.title, publishers)
        .then((r) => {
          if (!alive) return;
          if (r.offers.length > 0) setOffers(r.offers);
          setLinks(r.links);
        })
        .catch(() => {})
        .finally(() => alive && setLoadingOffers(false));

    if (hit.steamAppid) {
      void loadOffers([]);
      api
        .storeDetails(hit.steamAppid)
        .then((d) => {
          if (!alive) return;
          setDetails(d);
          // Les éditeurs affinent la liste des launchers où le jeu peut se trouver.
          if (d.publishers.length > 0) void loadOffers([...d.publishers, ...d.developers]);
        })
        .catch(() => {});
    } else {
      void loadOffers([]);
    }
    return () => {
      alive = false;
    };
  }, [hit]);

  const best = bestOffer(offers);
  const backdrop = hit.hero ?? details?.header ?? hit.image;
  const summary = details?.summary || hit.description;
  const score = details?.metacritic ?? null;
  const facts: [string, string][] = [];
  if (details?.releaseDate) facts.push([details.comingSoon ? "Sortie prévue" : "Sortie", details.releaseDate]);
  if (details?.developers.length) facts.push(["Développeur", details.developers.join(", ")]);
  if (details?.publishers.length) facts.push(["Éditeur", details.publishers.join(", ")]);
  if (details?.recommendations) {
    facts.push(["Recommandations Steam", details.recommendations.toLocaleString("fr-FR")]);
  }

  return (
    <Sheet onClose={onClose}>
      <div className="sheet-hero">
        {backdrop && <img className="bg" src={backdrop} alt="" />}
        <div className="sheet-head">
          {hit.cover && (
            <div className="sheet-cover">
              <img className="cover" src={hit.cover} alt="" />
            </div>
          )}
          <div className="sheet-headings">
            <h2 className="sheet-title">{details?.name || hit.title}</h2>
            <div className="sheet-chips">
              {score !== null && (
                <span className={score >= 75 ? "score" : score >= 50 ? "score mid" : "score low"} title="Metacritic">
                  {score}
                </span>
              )}
              {details?.genres.slice(0, 4).map((genre) => (
                <span className="chip" key={genre}>
                  {genre}
                </span>
              ))}
            </div>
          </div>
        </div>
      </div>

      <div className="sheet-body">
        {owned && (
          <div className="owned">
            <span>Déjà dans votre bibliothèque ({sources.get(owned.source)?.name ?? owned.source})</span>
            <button
              className="btn primary"
              onClick={() => {
                onPlay(owned.id);
                onClose();
              }}
            >
              <IconPlay />
              Jouer
            </button>
          </div>
        )}

        {summary && <p className="prose">{summary}</p>}

        <h3 className="subhead">Où l'acheter</h3>
        <div className="offers">
          {offers.map((offer) => (
            <button
              key={`${offer.store}|${offer.title}|${offer.note ?? ""}`}
              className={offer === best && offers.length > 1 ? "offer best" : "offer"}
              onClick={() => void api.openUrl(offer.url)}
            >
              <StoreIcon store={offer.store} sources={sources} />
              <div style={{ minWidth: 0 }}>
                <div className="offer-name">
                  {offer.storeName}
                  {offer === best && offers.length > 1 && <span className="tag-best">Meilleur prix</span>}
                </div>
                <div className="offer-sub">
                  {[offer.edition ?? "Édition standard", offer.note].filter(Boolean).join(" · ")}
                </div>
              </div>
              <div className="offer-price">
                <strong>{priceLabel(offer)}</strong>
                {offer.discount && offer.regular !== null ? (
                  <div className="was">
                    <span className="strike">{money(offer.regular, offer.currency)}</span>
                    <span className="discount">−{offer.discount} %</span>
                  </div>
                ) : null}
              </div>
              <span className="offer-go">
                <IconArrowOut />
              </span>
            </button>
          ))}

          {links.map((link) => (
            <button
              key={link.source}
              className="offer"
              onClick={() => (link.url ? void api.openUrl(link.url) : onOpenSource(link.source))}
            >
              <StoreIcon store={link.source} sources={sources} />
              <div style={{ minWidth: 0 }}>
                <div className="offer-name">{link.name}</div>
                <div className="offer-sub">
                  {link.url ? "Prix à consulter sur la boutique" : "Prix à consulter dans le launcher"}
                </div>
              </div>
              <div className="offer-price">
                <span className="link">{link.url ? "Rechercher" : "Ouvrir"}</span>
              </div>
              <span className="offer-go">
                <IconArrowOut />
              </span>
            </button>
          ))}

          {loadingOffers && offers.length === 0 && <div className="skeleton" style={{ height: 54 }} />}
          {!loadingOffers && offers.length === 0 && links.length === 0 && (
            <p className="prose">Aucune offre trouvée dans les boutiques de ce PC ni chez Instant Gaming.</p>
          )}
        </div>

        {details && details.screenshots.length > 0 && (
          <>
            <h3 className="subhead">Aperçu</h3>
            <div className="shots">
              {details.screenshots.map((shot) => (
                <button key={shot.full} onClick={() => setZoom(shot.full)} aria-label="Agrandir la capture">
                  <img src={shot.thumb} alt="" loading="lazy" decoding="async" />
                </button>
              ))}
            </div>
          </>
        )}

        {facts.length > 0 && (
          <>
            <h3 className="subhead">Informations</h3>
            <dl className="facts">
              {facts.map(([label, value]) => (
                <div className="fact" key={label}>
                  <dt>{label}</dt>
                  <dd title={value}>{value}</dd>
                </div>
              ))}
            </dl>
          </>
        )}

        {details?.about && details.about !== summary && (
          <>
            <h3 className="subhead">À propos</h3>
            <p className={expanded ? "prose" : "prose clamped"}>{details.about}</p>
            {!expanded && details.about.length > 500 && (
              <button className="link" style={{ marginTop: 6 }} onClick={() => setExpanded(true)}>
                Lire la suite
              </button>
            )}
          </>
        )}

        {details?.requirements && (
          <>
            <h3 className="subhead">Configuration minimale</h3>
            <p className="prose">{details.requirements}</p>
          </>
        )}
      </div>

      {zoom && (
        <div className="lightbox" onClick={() => setZoom(null)}>
          <img src={zoom} alt="" />
        </div>
      )}
    </Sheet>
  );
}
