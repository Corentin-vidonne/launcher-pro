// Éléments de structure : page (barre du haut + panneau de verre), feuille modale, menu contextuel.

import { useEffect, useRef, useState, type ReactNode } from "react";

import { windowControls } from "../lib/api";
import { useLens } from "../lib/lens";
import { IconClose, IconMinus, IconSquare } from "./Icons";

function WindowControls() {
  return (
    <div className="window-controls">
      <button aria-label="Réduire" onClick={() => void windowControls.minimize()}>
        <IconMinus />
      </button>
      <button aria-label="Agrandir" onClick={() => void windowControls.toggleMaximize()}>
        <IconSquare />
      </button>
      <button className="close" aria-label="Fermer" onClick={() => void windowControls.close()}>
        <IconClose />
      </button>
    </div>
  );
}

/**
 * Disposition d'une rubrique : capsules de verre en haut (`actions`), mise en avant facultative
 * (`hero`, texte posé directement sur l'artwork), puis un grand panneau de verre dont seul le
 * contenu défile — le panneau lui-même ne bouge pas, donc son flou n'est jamais recalculé.
 */
export function Page({ actions, hero, children }: { actions?: ReactNode; hero?: ReactNode; children: ReactNode }) {
  const panel = useRef<HTMLElement>(null);
  useLens(panel, { r: 30, bezel: 22 });
  return (
    <>
      <header className="topbar" data-tauri-drag-region>
        <div className="topbar-actions" data-tauri-drag-region>
          {actions}
        </div>
        <WindowControls />
      </header>
      {hero}
      <section ref={panel} className="panel glass">
        <div className="panel-scroll">{children}</div>
      </section>
    </>
  );
}

/** Feuille modale : fermeture par Échap, clic sur le fond ou bouton. */
export function Sheet({
  onClose,
  narrow,
  children,
}: {
  onClose: () => void;
  narrow?: boolean;
  children: ReactNode;
}) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div
      className="backdrop"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className={narrow ? "sheet narrow" : "sheet"} role="dialog" aria-modal="true">
        <button className="sheet-close" aria-label="Fermer" onClick={onClose}>
          <IconClose />
        </button>
        <div className="sheet-scroll">{children}</div>
      </div>
    </div>
  );
}

export interface MenuItem {
  label: string;
  icon?: ReactNode;
  danger?: boolean;
  separatorBefore?: boolean;
  action: () => void;
}

export interface MenuState {
  x: number;
  y: number;
  items: MenuItem[];
}

export function ContextMenu({ menu, onClose }: { menu: MenuState; onClose: () => void }) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ left: menu.x, top: menu.y });

  // Recale le menu s'il dépasse de la fenêtre.
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const { width, height } = el.getBoundingClientRect();
    setPos({
      left: Math.min(menu.x, window.innerWidth - width - 8),
      top: Math.min(menu.y, window.innerHeight - height - 8),
    });
  }, [menu]);

  useEffect(() => {
    const close = () => onClose();
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("mousedown", close);
    window.addEventListener("blur", close);
    window.addEventListener("resize", close);
    window.addEventListener("wheel", close, { passive: true });
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("blur", close);
      window.removeEventListener("resize", close);
      window.removeEventListener("wheel", close);
      window.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  return (
    <div ref={ref} className="menu" style={pos} onMouseDown={(e) => e.stopPropagation()}>
      {menu.items.map((item) => (
        <div key={item.label}>
          {item.separatorBefore && <hr />}
          <button
            className={item.danger ? "danger" : undefined}
            onClick={() => {
              onClose();
              item.action();
            }}
          >
            {item.icon}
            {item.label}
          </button>
        </div>
      ))}
    </div>
  );
}
