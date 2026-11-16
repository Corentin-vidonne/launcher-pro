import { useEffect, useRef, useState, type FormEvent } from "react";

import { api } from "../lib/api";
import type { Library } from "../lib/types";
import { IconEyeOff, IconLock } from "./Icons";

type Mode = "loading" | "create" | "locked" | "open" | "change";

interface Props {
  /** Coffre déverrouillé : l'onglet « Masqués » est visible dans la barre latérale. */
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Aller à l'onglet « Masqués ». */
  onShow: () => void;
  onLibrary: (library: Library) => void;
}

const TEXTS = {
  create: {
    title: "Protéger les jeux masqués",
    text: "Choisissez un mot de passe : l'onglet « Masqués » n'apparaîtra qu'après l'avoir saisi ici.",
    action: "Protéger",
  },
  locked: {
    title: "Jeux masqués verrouillés",
    text: "Saisissez le mot de passe pour faire apparaître l'onglet « Masqués ».",
    action: "Déverrouiller",
  },
  change: {
    title: "Changer le mot de passe",
    text: "Saisissez le mot de passe actuel, puis le nouveau.",
    action: "Enregistrer",
  },
};

/**
 * Accès aux jeux masqués : tant que le coffre est verrouillé, le backend ne les envoie pas à
 * l'interface. Le bon mot de passe fait apparaître l'onglet « Masqués », jusqu'au verrouillage
 * ou à la fermeture de l'application.
 */
export function HiddenVault({ open, onOpenChange, onShow, onLibrary }: Props) {
  const [hasPassword, setHasPassword] = useState<boolean | null>(null);
  const [changing, setChanging] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [current, setCurrent] = useState("");
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const form = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let alive = true;
    api
      .vaultStatus()
      .then((status) => {
        if (!alive) return;
        setHasPassword(status.hasPassword);
        onOpenChange(status.unlocked);
      })
      .catch((e) => alive && setError(String(e)));
    return () => {
      alive = false;
    };
  }, [onOpenChange]);

  const mode: Mode =
    hasPassword === null ? "loading" : changing ? "change" : open ? "open" : hasPassword ? "locked" : "create";

  // Les mots de passe ne restent jamais en mémoire plus que nécessaire.
  const clearFields = () => {
    setCurrent("");
    setPassword("");
    setConfirm("");
  };

  const fail = (message: string) => {
    setError(message);
    form.current?.animate(
      [
        { transform: "translateX(0)" },
        { transform: "translateX(-8px)" },
        { transform: "translateX(7px)" },
        { transform: "translateX(-4px)" },
        { transform: "translateX(0)" },
      ],
      { duration: 380, easing: "ease-out" },
    );
  };

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (busy || mode === "loading" || mode === "open") return;
    setError(null);
    if (mode !== "locked") {
      if (password.length < 4) return fail("Le mot de passe doit contenir au moins 4 caractères.");
      if (password !== confirm) return fail("Les deux mots de passe ne correspondent pas.");
    }
    setBusy(true);
    try {
      const library =
        mode === "locked"
          ? await api.vaultUnlock(password)
          : await api.vaultSetPassword(password, mode === "change" ? current : undefined);
      onLibrary(library);
      onOpenChange(true);
      setHasPassword(true);
      clearFields();
      if (mode === "change") setChanging(false);
      else onShow(); // déverrouillé : direction l'onglet « Masqués »
    } catch (err) {
      clearFields();
      fail(String(err));
    } finally {
      setBusy(false);
    }
  };

  const lock = () => {
    api
      .vaultLock()
      .then((library) => {
        onLibrary(library);
        onOpenChange(false);
        setError(null);
      })
      .catch((e) => setError(String(e)));
  };

  if (mode === "loading") {
    return (
      <div className="group">
        <div className="row">
          <div className="text">
            <small>{error ?? "Chargement…"}</small>
          </div>
        </div>
      </div>
    );
  }

  if (mode === "open") {
    return (
      <div className="group">
        <div className="vault-lock">
          <span className="lock">
            <IconEyeOff />
          </span>
          <strong>Jeux masqués déverrouillés</strong>
          <small>L'onglet « Masqués » est visible dans la barre latérale jusqu'au verrouillage ou à la fermeture de l'application.</small>
          <div className="vault-form">
            {error && <span className="vault-error">{error}</span>}
            <div className="vault-actions">
              <button className="btn" onClick={onShow}>
                Ouvrir l'onglet
              </button>
              <button className="btn primary" onClick={lock}>
                <IconLock />
                Verrouiller
              </button>
            </div>
            <button
              className="link"
              onClick={() => {
                clearFields();
                setError(null);
                setChanging(true);
              }}
            >
              Changer le mot de passe
            </button>
          </div>
        </div>
      </div>
    );
  }

  const texts = TEXTS[mode];
  return (
    <div className="group">
      <form className="vault-lock" onSubmit={submit}>
        <span className="lock">
          <IconLock />
        </span>
        <strong>{texts.title}</strong>
        <small>{texts.text}</small>
        <div ref={form} className="vault-form">
          {mode === "change" && (
            <input
              className="field"
              type="password"
              placeholder="Mot de passe actuel"
              autoComplete="current-password"
              autoFocus
              value={current}
              onChange={(e) => setCurrent(e.target.value)}
            />
          )}
          <input
            className="field"
            type="password"
            placeholder={mode === "locked" ? "Mot de passe" : "Nouveau mot de passe"}
            autoComplete={mode === "locked" ? "current-password" : "new-password"}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          {mode !== "locked" && (
            <input
              className="field"
              type="password"
              placeholder="Confirmer le mot de passe"
              autoComplete="new-password"
              value={confirm}
              onChange={(e) => setConfirm(e.target.value)}
            />
          )}
          {error && <span className="vault-error">{error}</span>}
          <div className="vault-actions">
            {mode === "change" && (
              <button
                type="button"
                className="btn"
                onClick={() => {
                  clearFields();
                  setError(null);
                  setChanging(false);
                }}
              >
                Annuler
              </button>
            )}
            <button type="submit" className="btn primary" disabled={busy || !password}>
              {busy ? "Vérification…" : texts.action}
            </button>
          </div>
        </div>
      </form>
    </div>
  );
}
