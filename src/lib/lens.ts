// Réfraction « lentille » du Liquid Glass : un filtre SVG (feDisplacementMap) appliqué au fond
// de l'élément via `backdrop-filter: url(#…)`. La carte de déplacement est calculée une fois par
// taille d'élément (à demi-résolution pour les grands panneaux), jamais pendant le défilement.

import { useEffect, useSyncExternalStore, type RefObject } from "react";

const NS = "http://www.w3.org/2000/svg";

let defs: SVGDefsElement | null = null;
let counter = 0;
let enabled = true;
const listeners = new Set<() => void>();

/** Active ou coupe l'effet pour toute l'application (réglage « Effet Liquid Glass »). */
export function setLensEnabled(value: boolean) {
  if (enabled === value) return;
  enabled = value;
  listeners.forEach((l) => l());
}

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => void listeners.delete(listener);
};

function getDefs(): SVGDefsElement {
  if (!defs) {
    const svg = document.createElementNS(NS, "svg");
    svg.setAttribute("width", "0");
    svg.setAttribute("height", "0");
    svg.setAttribute("aria-hidden", "true");
    svg.style.position = "absolute";
    defs = document.createElementNS(NS, "defs");
    svg.append(defs);
    document.body.append(svg);
  }
  return defs;
}

export interface LensOptions {
  /** Rayon des coins de l'élément (px). */
  r: number;
  /** Largeur de la bordure qui réfracte (px). */
  bezel: number;
  /** Déplacement maximal ≈ scale / 2 (px). */
  scale?: number;
  blur?: number;
}

/** Image PNG (data URI) : rouge/vert = direction et intensité du déplacement, 128 = neutre. */
function displacementMap(w: number, h: number, r: number, bezel: number): string | null {
  const q = Math.max(w, h) > 500 ? 0.5 : 1;
  const cw = Math.max(1, Math.ceil(w * q));
  const ch = Math.max(1, Math.ceil(h * q));
  const canvas = document.createElement("canvas");
  canvas.width = cw;
  canvas.height = ch;
  const ctx = canvas.getContext("2d");
  if (!ctx) return null;
  const image = ctx.createImageData(cw, ch);
  const data = image.data;
  const hw = w / 2;
  const hh = h / 2;
  const rr = Math.min(r, hw, hh);

  for (let j = 0; j < ch; j++) {
    for (let i = 0; i < cw; i++) {
      const px = (i + 0.5) / q - hw;
      const py = (j + 0.5) / q - hh;
      const sx = px < 0 ? -1 : 1;
      const sy = py < 0 ? -1 : 1;
      const qx = Math.abs(px) - (hw - rr);
      const qy = Math.abs(py) - (hh - rr);
      let dist: number;
      let nx = 0;
      let ny = 0;
      if (qx > 0 && qy > 0) {
        const l = Math.hypot(qx, qy);
        dist = rr - l;
        nx = (qx / l) * sx;
        ny = (qy / l) * sy;
      } else if (qx > qy) {
        dist = rr - qx;
        nx = sx;
      } else {
        dist = rr - qy;
        ny = sy;
      }
      let m = 0;
      if (dist >= 0 && dist < bezel) {
        const t = 1 - dist / bezel; // 1 au bord, 0 à l'intérieur
        m = t * t * (3 - 2 * t);
      }
      const k = (j * cw + i) * 4;
      data[k] = 128 - nx * m * 127; // on prélève vers l'intérieur : le bord « plonge » sur le fond
      data[k + 1] = 128 - ny * m * 127;
      data[k + 2] = 128;
      data[k + 3] = 255;
    }
  }
  ctx.putImageData(image, 0, 0);
  return canvas.toDataURL();
}

function mount(el: HTMLElement, o: LensOptions): () => void {
  const w = el.offsetWidth;
  const h = el.offsetHeight;
  if (!w || !h) return () => {};
  const href = displacementMap(w, h, o.r, o.bezel);
  if (!href) return () => {};

  const id = `lens-${++counter}`;
  const filter = document.createElementNS(NS, "filter");
  filter.id = id;
  filter.setAttribute("filterUnits", "userSpaceOnUse");
  filter.setAttribute("x", "0");
  filter.setAttribute("y", "0");
  filter.setAttribute("width", String(w));
  filter.setAttribute("height", String(h));
  filter.setAttribute("color-interpolation-filters", "sRGB");

  const map = document.createElementNS(NS, "feImage");
  map.setAttribute("href", href);
  map.setAttribute("x", "0");
  map.setAttribute("y", "0");
  map.setAttribute("width", String(w));
  map.setAttribute("height", String(h));
  map.setAttribute("result", "map");

  const displace = document.createElementNS(NS, "feDisplacementMap");
  displace.setAttribute("in", "SourceGraphic");
  displace.setAttribute("in2", "map");
  displace.setAttribute("scale", String(o.scale ?? 70));
  displace.setAttribute("xChannelSelector", "R");
  displace.setAttribute("yChannelSelector", "G");

  filter.append(map, displace);
  getDefs().append(filter);

  const value = `url(#${id}) blur(${o.blur ?? 7}px) saturate(1.8) brightness(1.07)`;
  el.style.backdropFilter = value;
  el.style.setProperty("-webkit-backdrop-filter", value);
  return () => {
    filter.remove();
    el.style.backdropFilter = "";
    el.style.removeProperty("-webkit-backdrop-filter");
  };
}

/** Applique la réfraction à un élément `.glass` ; recalculée quand sa taille change. */
export function useLens(ref: RefObject<HTMLElement | null>, options: LensOptions) {
  const on = useSyncExternalStore(subscribe, () => enabled);
  const { r, bezel } = options;

  useEffect(() => {
    const el = ref.current;
    if (!el || !on) return;
    let cleanup = mount(el, { r, bezel });
    let last = `${el.offsetWidth}x${el.offsetHeight}`;
    let timer = 0;
    const observer = new ResizeObserver(() => {
      const size = `${el.offsetWidth}x${el.offsetHeight}`;
      if (size === last) return;
      last = size;
      clearTimeout(timer);
      timer = window.setTimeout(() => {
        cleanup();
        cleanup = mount(el, { r, bezel });
      }, 140);
    });
    observer.observe(el);
    return () => {
      clearTimeout(timer);
      observer.disconnect();
      cleanup();
    };
  }, [ref, on, r, bezel]);
}
