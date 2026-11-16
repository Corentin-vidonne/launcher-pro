const relative = new Intl.RelativeTimeFormat("fr", { numeric: "auto" });

/** « hier », « il y a 3 jours », « il y a 2 mois »… à partir d'un horodatage Unix (secondes). */
export function timeAgo(unixSeconds: number | null): string | null {
  if (!unixSeconds) return null;
  const diff = unixSeconds - Date.now() / 1000;
  const abs = Math.abs(diff);
  if (abs < 3600) return abs < 90 ? "à l'instant" : relative.format(Math.round(diff / 60), "minute");
  if (abs < 86400) return relative.format(Math.round(diff / 3600), "hour");
  if (abs < 86400 * 30) return relative.format(Math.round(diff / 86400), "day");
  if (abs < 86400 * 365) return relative.format(Math.round(diff / (86400 * 30)), "month");
  return relative.format(Math.round(diff / (86400 * 365)), "year");
}

export function playtime(minutes: number | null): string | null {
  if (!minutes) return null;
  if (minutes < 60) return `${minutes} min`;
  const hours = minutes / 60;
  return `${hours < 10 ? hours.toFixed(1).replace(".", ",").replace(",0", "") : Math.round(hours)} h`;
}

export function fileSize(bytes: number | null): string | null {
  if (!bytes) return null;
  const gb = bytes / 1024 ** 3;
  if (gb >= 1) return `${gb >= 100 ? Math.round(gb) : gb.toFixed(1).replace(".", ",")} Go`;
  return `${Math.max(1, Math.round(bytes / 1024 ** 2))} Mo`;
}

const formatters = new Map<string, Intl.NumberFormat>();

export function money(amount: number, currency: string): string {
  let f = formatters.get(currency);
  if (!f) {
    try {
      f = new Intl.NumberFormat("fr-FR", { style: "currency", currency });
    } catch {
      f = new Intl.NumberFormat("fr-FR", { style: "currency", currency: "EUR" });
    }
    formatters.set(currency, f);
  }
  return f.format(amount);
}

/** Même normalisation que le backend (`match_key`) pour relier magasin et bibliothèque. */
export function titleKey(title: string): string {
  const editions = new Set([
    "edition", "editions", "standard", "deluxe", "ultimate", "gold", "premium", "special",
    "digital", "collector", "collectors", "complete", "goty", "legendary", "ultime", "bundle", "launch",
  ]);
  const platforms = new Set(["pc", "windows", "win10", "eu", "na", "asia", "for", "steam"]);
  const tokens = title
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ™®©'’`]/g, "")
    .replace(/game of the year/g, "goty")
    .replace(/[^\p{L}\p{N}]+/gu, " ")
    .trim()
    .split(" ");
  while (tokens.length > 1 && editions.has(tokens[tokens.length - 1])) tokens.pop();
  while (tokens.length > 1 && platforms.has(tokens[tokens.length - 1])) tokens.pop();
  return tokens.join(" ");
}

/** Teinte stable dérivée d'une chaîne (tuiles sans jaquette). */
export function hueOf(text: string): number {
  let h = 0;
  for (let i = 0; i < text.length; i++) h = (h * 31 + text.charCodeAt(i)) >>> 0;
  return h % 360;
}
