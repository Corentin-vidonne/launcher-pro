import { useEffect, useState } from "react";

interface Layer {
  src: string;
  blurred: boolean;
}

/**
 * Fond de toute l'interface : l'artwork du jeu mis en avant, en fondu enchaîné quand il change.
 * Sans artwork, un dégradé « aurore » prend le relais (voir `--ambient-base`).
 */
export function Ambient({ src, blurred }: { src: string | null; blurred: boolean }) {
  const [layers, setLayers] = useState<Layer[]>([]);

  useEffect(() => {
    if (!src) return;
    setLayers((l) => (l.at(-1)?.src === src ? l : [...l.slice(-2), { src, blurred }]));
    // Une fois le fondu terminé, seule la dernière image reste dans le DOM.
    const timer = setTimeout(() => setLayers((l) => l.slice(-1)), 1100);
    return () => clearTimeout(timer);
  }, [src, blurred]);

  return (
    <div className="ambient" aria-hidden="true">
      {layers.map((layer) => (
        <img
          key={layer.src}
          className={layer.blurred ? "blurred" : undefined}
          src={layer.src}
          alt=""
          decoding="async"
        />
      ))}
      <div className="scrim" />
    </div>
  );
}
