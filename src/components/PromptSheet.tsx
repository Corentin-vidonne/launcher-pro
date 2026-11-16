import { useState, type FormEvent } from "react";

import { Sheet } from "./Shell";

export interface PromptState {
  title: string;
  message?: string;
  /** Présent = un champ de saisie, prérempli avec cette valeur. */
  value?: string;
  placeholder?: string;
  confirm: string;
  danger?: boolean;
  onConfirm: (value: string) => Promise<void> | void;
}

/** Petite feuille de saisie ou de confirmation (nom de collection, suppression…). */
export function PromptSheet({ prompt, onClose }: { prompt: PromptState; onClose: () => void }) {
  const [value, setValue] = useState(prompt.value ?? "");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const withInput = prompt.value !== undefined;

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await prompt.onConfirm(value.trim());
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Sheet narrow onClose={onClose}>
      <form className="prompt" onSubmit={submit}>
        <h2>{prompt.title}</h2>
        {prompt.message && <p>{prompt.message}</p>}
        {withInput && (
          <input
            className="field"
            autoFocus
            value={value}
            maxLength={40}
            placeholder={prompt.placeholder}
            onChange={(e) => setValue(e.target.value)}
          />
        )}
        {error && <span className="vault-error">{error}</span>}
        <div className="prompt-actions">
          <button type="button" className="btn" onClick={onClose}>
            Annuler
          </button>
          <button
            type="submit"
            className={prompt.danger ? "btn danger" : "btn primary"}
            disabled={busy || (withInput && !value.trim())}
          >
            {prompt.confirm}
          </button>
        </div>
      </form>
    </Sheet>
  );
}
