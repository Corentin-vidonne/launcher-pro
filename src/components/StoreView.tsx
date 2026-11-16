import { useEffect, useRef, useState } from "react";

import { api } from "../lib/api";
import { money, titleKey } from "../lib/format";
import type { Featured, FeaturedItem, Offer, SearchResult, Source, StoreHit } from "../lib/types";
import { IconBag, IconClose, IconSearch } from "./Icons";
import { Page } from "./Shell";

/** Meilleure offre d'une fiche : l'édition de base la moins chère (les offres arrivent déjà triées). */
export function bestOffer(offers: Offer[]): Offer | null {
  return offers.find((o) => !o.edition) ?? offers[0] ?? null;
}

export function priceLabel(offer: Offer): string {
  return offer.free || offer.price === null ? "Gratuit" : money(offer.price, offer.currency);
}

/** Pictogramme d'une boutique : icône réelle du launcher installé, sinon pastille. */
export function StoreIcon({ store, sources }: { store: string; sources: Map<string, Source> }) {
  const source = sources.get(store === "microsoft" ? "xbox" : store);
  if (source?.icon) {
    return (
      <span className="store-icon">
        <img src={source.icon} alt="" />
      </span>
    );
  }
  const [cls, text] =
    store === "instantgaming"
      ? ["store-icon ig", "IG"]
      : store === "microsoft"
        ? ["store-icon ms", "MS"]
        : store === "gog"
          ? ["store-icon gog", "GOG"]
          : ["store-icon", (source?.name ?? store).charAt(0).toUpperCase()];
  return <span className={cls}>{text}</span>;
}

// Conservé entre deux visites de l'onglet : on retrouve sa recherche telle qu'on l'a laissée.
const memory: { query: string; searched: string; result: SearchResult | null; featured: Featured | null } = {
  query: "",
  searched: "", // requête à laquelle correspond `result`
  result: null,
  featured: null,
};

interface Props {
  sources: Map<string, Source>;
  /** Recherche demandée depuis une fiche de la bibliothèque (le numéro force la prise en compte). */
  request: { query: string; nonce: number } | null;
  onOpen: (hit: StoreHit) => void;
}

export function StoreView({ sources, request, onOpen }: Props) {
  const [query, setQuery] = useState(memory.query);
  const [result, setResult] = useState<SearchResult | null>(memory.result);
  const [featured, setFeatured] = useState<Featured | null>(memory.featured);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const ticket = useRef(0);
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    input.current?.focus();
    if (!memory.featured) {
      api
        .storeFeatured()
        .then((f) => {
          memory.featured = f;
          setFeatured(f);
        })
        .catch(() => {});
    }
  }, []);

  useEffect(() => {
    if (request) setQuery(request.query);
  }, [request]);

  // Recherche différée : une seule requête une fois la frappe terminée.
  useEffect(() => {
    memory.query = query;
    const q = query.trim();
    const mine = ++ticket.current;
    if (q.length < 2) {
      memory.result = null;
      memory.searched = "";
      setResult(null);
      setLoading(false);
      setError(null);
      return;
    }
    if (memory.result && memory.searched === q.toLowerCase()) {
      // Résultat déjà en main (retour sur l'onglet, ou frappe annulée) : rien à refaire.
      setResult(memory.result);
      setLoading(false);
      return;
    }
    setLoading(true);
    const timer = setTimeout(() => {
      api
        .storeSearch(q)
        .then((r) => {
          if (mine !== ticket.current) return; // réponse d'une frappe précédente
          memory.searched = q.toLowerCase();
          memory.result = r;
          setResult(r);
          setError(null);
        })
        .catch((e) => mine === ticket.current && setError(String(e)))
        .finally(() => mine === ticket.current && setLoading(false));
    }, 380);
    return () => clearTimeout(timer);
  }, [query]);

  const openFeatured = (item: FeaturedItem) =>
    onOpen({
      key: titleKey(item.title),
      title: item.title,
      image: item.image,
      cover: null,
      hero: null,
      description: null,
      steamAppid: item.appid,
      offers: [],
    });

  const searching = query.trim().length >= 2;

  return (
    <Page>
      <div className="page-head">
        <div className="grow">
          <h1 className="page-title">Magasin</h1>
          <p className="page-sub">Cherchez un jeu et comparez les prix des boutiques installées sur ce PC.</p>
        </div>
      </div>

      <div className="store-search">
        <label className="search big">
          <IconSearch />
          <input
            ref={input}
            value={query}
            placeholder="Rechercher un jeu…"
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
      </div>

      {searching ? (
        <SearchResults
          result={result}
          loading={loading}
          error={error}
          query={query}
          sources={sources}
          onOpen={onOpen}
        />
      ) : (
        <>
          <Shelf title="Promotions du moment" items={featured?.specials} onOpen={openFeatured} />
          <Shelf title="Meilleures ventes" items={featured?.topSellers} onOpen={openFeatured} />
          <Shelf title="Nouveautés" items={featured?.newReleases} onOpen={openFeatured} />
        </>
      )}
    </Page>
  );
}

function SearchResults({
  result,
  loading,
  error,
  query,
  sources,
  onOpen,
}: {
  result: SearchResult | null;
  loading: boolean;
  error: string | null;
  query: string;
  sources: Map<string, Source>;
  onOpen: (hit: StoreHit) => void;
}) {
  if (loading && !result) {
    return (
      <div className="hits">
        {[0, 1, 2, 3].map((i) => (
          <div className="hit" key={i}>
            <div className="hit-art skeleton" />
            <div>
              <div className="skeleton" style={{ height: 16, width: "42%" }} />
              <div className="skeleton" style={{ height: 12, width: "78%", marginTop: 10 }} />
            </div>
          </div>
        ))}
      </div>
    );
  }
  if (error) {
    return (
      <div className="empty">
        <IconBag />
        <strong>Recherche impossible</strong>
        <p>Vérifiez votre connexion Internet, puis réessayez.</p>
      </div>
    );
  }
  if (!result || result.hits.length === 0) {
    return (
      <div className="empty">
        <IconBag />
        <strong>Aucun résultat</strong>
        <p>Aucun jeu trouvé pour « {query.trim()} ».</p>
      </div>
    );
  }
  return (
    <>
      <div className="hits" style={loading ? { opacity: 0.55 } : undefined}>
        {result.hits.map((hit) => {
          const best = bestOffer(hit.offers);
          const stores = [...new Set(hit.offers.map((o) => o.storeName))];
          return (
            <button className="hit" key={hit.key} onClick={() => onOpen(hit)}>
              <div className="hit-art">
                {hit.image && <img src={hit.image} alt="" loading="lazy" decoding="async" />}
              </div>
              <div>
                <div className="hit-title">{hit.title}</div>
                {hit.description && <div className="hit-desc">{hit.description}</div>}
                <div className="hit-stores">
                  {stores.map((name) => (
                    <span className="chip" key={name}>
                      {name}
                    </span>
                  ))}
                </div>
              </div>
              <div className="hit-price">
                {best ? (
                  <>
                    <small>{hit.offers.length > 1 ? "à partir de" : "prix"}</small>
                    <strong>{priceLabel(best)}</strong>
                    <div className="where">
                      {best.discount ? <span className="discount">−{best.discount} %</span> : null} {best.storeName}
                    </div>
                  </>
                ) : (
                  <small>Voir la fiche</small>
                )}
              </div>
            </button>
          );
        })}
      </div>
      {result.failed.length > 0 && (
        <p className="note">Sans réponse pour cette recherche : {result.failed.join(", ")}.</p>
      )}
      <StoreNote sources={sources} />
    </>
  );
}

/** Rappel honnête de ce qui est comparé (et de ce qui ne peut pas l'être). */
function StoreNote({ sources }: { sources: Map<string, Source> }) {
  const unpriced = ["epic", "ea", "ubisoft", "battlenet", "rockstar"]
    .map((id) => sources.get(id)?.name)
    .filter(Boolean);
  if (unpriced.length === 0) return null;
  return (
    <p className="note">
      {unpriced.join(", ")} : ces boutiques ne publient pas leurs prix ; un lien direct est proposé dans la fiche du jeu.
    </p>
  );
}

function Shelf({
  title,
  items,
  onOpen,
}: {
  title: string;
  items: FeaturedItem[] | undefined;
  onOpen: (item: FeaturedItem) => void;
}) {
  if (items && items.length === 0) return null;
  return (
    <section className="shelf">
      <h2 className="section-title">{title}</h2>
      <div className="shelf-row">
        {items
          ? items.map((item) => (
              <button className="promo" key={item.appid} onClick={() => onOpen(item)}>
                <div className="promo-art">
                  {item.image && <img src={item.image} alt="" loading="lazy" decoding="async" />}
                </div>
                <div className="promo-title">{item.title}</div>
                <div className="promo-price">
                  {item.discount > 0 && <span className="discount">−{item.discount} %</span>}
                  {item.discount > 0 && item.regular !== null && (
                    <span className="strike">{money(item.regular, item.currency)}</span>
                  )}
                  <span>{item.price ? money(item.price, item.currency) : "Gratuit"}</span>
                </div>
              </button>
            ))
          : [0, 1, 2, 3].map((i) => (
              <div key={i}>
                <div className="promo-art skeleton" />
                <div className="skeleton" style={{ height: 13, width: "70%", marginTop: 10 }} />
              </div>
            ))}
      </div>
    </section>
  );
}
