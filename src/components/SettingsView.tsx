import type { AppInfo, Library, Settings } from "../lib/types";
import { api } from "../lib/api";
import { HiddenVault } from "./HiddenVault";
import { IconPlus, IconRefresh } from "./Icons";
import { Page } from "./Shell";

interface Props {
  info: AppInfo;
  library: Library;
  scanning: boolean;
  onChange: (patch: Partial<Settings>) => void;
  onRescan: () => void;
  onAddGame: () => void;
  vaultOpen: boolean;
  onVaultOpen: (open: boolean) => void;
  onShowHidden: () => void;
  onLibrary: (library: Library) => void;
  onOpenSource: (id: string) => void;
}

function Segmented<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: readonly (readonly [T, string])[];
  onChange: (value: T) => void;
}) {
  return (
    <div className="segmented">
      {options.map(([v, label]) => (
        <button key={v} className={v === value ? "on" : undefined} onClick={() => onChange(v)}>
          {label}
        </button>
      ))}
    </div>
  );
}

function Toggle({ on, onChange, label }: { on: boolean; onChange: (on: boolean) => void; label: string }) {
  return (
    <button
      className={on ? "switch on" : "switch"}
      role="switch"
      aria-checked={on}
      aria-label={label}
      onClick={() => onChange(!on)}
    />
  );
}

export function SettingsView({ info, library, scanning, onChange, onRescan, onAddGame, vaultOpen, onVaultOpen, onShowHidden, onLibrary, onOpenSource }: Props) {
  const { settings } = info;
  const scanned = library.scannedAt
    ? new Date(library.scannedAt * 1000).toLocaleString("fr-FR", { dateStyle: "medium", timeStyle: "short" })
    : "jamais";

  return (
    <Page>
      <div className="page-head">
        <div className="grow">
          <h1 className="page-title">Réglages</h1>
        </div>
      </div>

      <div className="settings">
        <div className="group-title" style={{ marginTop: 0 }}>
          Apparence
        </div>
        <div className="group">
          <div className="row">
            <div className="text">
              <strong>Thème</strong>
            </div>
            <Segmented
              value={settings.theme}
              options={[
                ["system", "Système"],
                ["light", "Clair"],
                ["dark", "Sombre"],
              ]}
              onChange={(theme) => onChange({ theme })}
            />
          </div>
          <div className="row">
            <div className="text">
              <strong>Taille des jaquettes</strong>
            </div>
            <Segmented
              value={settings.gridSize}
              options={[
                ["small", "Petite"],
                ["medium", "Moyenne"],
                ["large", "Grande"],
              ]}
              onChange={(gridSize) => onChange({ gridSize })}
            />
          </div>
          <div className="row">
            <div className="text">
              <strong>Effet Liquid Glass</strong>
              <small>Panneaux de verre translucide qui réfractent l'artwork. À couper pour économiser le GPU.</small>
            </div>
            <Toggle
              on={settings.liquidGlass}
              label="Effet Liquid Glass"
              onChange={(liquidGlass) => onChange({ liquidGlass })}
            />
          </div>
        </div>

        <div className="group-title">Bibliothèque</div>
        <div className="group">
          <div className="row">
            <div className="text">
              <strong>Lancer au démarrage de Windows</strong>
              <small>Launcher Pro s'ouvre agrandi à l'ouverture de session, prêt à jouer.</small>
            </div>
            <Toggle
              on={settings.launchAtStartup}
              label="Lancer au démarrage de Windows"
              onChange={(launchAtStartup) => onChange({ launchAtStartup })}
            />
          </div>
          <div className="row">
            <div className="text">
              <strong>Après le lancement d'un jeu</strong>
              <small>
                {settings.afterLaunch === "quit"
                  ? "Launcher Pro se ferme : plus aucune mémoire utilisée pendant la partie."
                  : settings.afterLaunch === "minimize"
                    ? "La fenêtre se réduit et libère la mémoire de son affichage."
                    : "La fenêtre reste ouverte."}
              </small>
            </div>
            <Segmented
              value={settings.afterLaunch}
              options={[
                ["stay", "Rester ouvert"],
                ["minimize", "Réduire"],
                ["quit", "Quitter"],
              ]}
              onChange={(afterLaunch) => onChange({ afterLaunch })}
            />
          </div>
          <div className="row">
            <div className="text">
              <strong>Afficher les logiciels</strong>
              <small>SteamVR, Wallpaper Engine et autres outils installés via un launcher.</small>
            </div>
            <Toggle on={settings.showTools} label="Afficher les logiciels" onChange={(showTools) => onChange({ showTools })} />
          </div>
          <div className="row">
            <div className="text">
              <strong>Analyse du PC</strong>
              <small>
                {library.games.length} jeux · {library.sources.length} sources · dernière analyse : {scanned}
              </small>
            </div>
            <button className="btn" disabled={scanning} onClick={onRescan}>
              <IconRefresh />
              {scanning ? "Analyse…" : "Rescanner"}
            </button>
          </div>
          <div className="row">
            <div className="text">
              <strong>Ajouter un jeu manuellement</strong>
              <small>Pour un jeu qui n'appartient à aucun launcher : choisissez son exécutable.</small>
            </div>
            <button className="btn" onClick={onAddGame}>
              <IconPlus />
              Ajouter
            </button>
          </div>
        </div>

        <div className="group-title">Recherche rapide</div>
        <div className="group">
          <div className="row">
            <div className="text">
              <strong>Raccourci global</strong>
              <small>
                {settings.quickShortcut && !info.quickShortcutActive
                  ? "Ce raccourci est déjà utilisé par une autre application : choisissez-en un autre."
                  : "Ouvre une barre pour lancer un jeu depuis n'importe où dans Windows, même Launcher Pro réduit."}
              </small>
            </div>
            <Segmented
              value={settings.quickShortcut}
              options={[
                ["ctrl+alt+space", "Ctrl+Alt+Espace"],
                ["alt+shift+space", "Alt+Maj+Espace"],
                ["ctrl+shift+space", "Ctrl+Maj+Espace"],
                ["", "Désactivée"],
              ]}
              onChange={(quickShortcut) => onChange({ quickShortcut })}
            />
            <button className="btn" onClick={() => void api.quickOpen().catch(() => {})}>
              Essayer
            </button>
          </div>
        </div>

        <div className="group-title">Magasin</div>
        <div className="group">
          <div className="row">
            <div className="text">
              <strong>Inclure les boutiques non installées</strong>
              <small>
                Par défaut, seuls les prix des boutiques présentes sur ce PC et d'Instant Gaming sont affichés.
              </small>
            </div>
            <Toggle
              on={settings.showAllStores}
              label="Inclure les boutiques non installées"
              onChange={(showAllStores) => onChange({ showAllStores })}
            />
          </div>
        </div>

        <div className="group-title">Sources détectées</div>
        <div className="group">
          {library.sources.map((source) => (
            <div className="row" key={source.id}>
              {source.icon ? (
                <img className="src" src={source.icon} alt="" />
              ) : (
                <span className="store-icon" style={{ width: 22, height: 22, borderRadius: 5, fontSize: 10 }}>
                  {source.name.charAt(0)}
                </span>
              )}
              <div className="text">
                <strong>{source.name}</strong>
                <small>{source.count === 0 ? "Aucun jeu installé" : source.count === 1 ? "1 jeu" : `${source.count} jeux`}</small>
              </div>
              {source.canOpen && (
                <button className="btn" onClick={() => onOpenSource(source.id)}>
                  Ouvrir
                </button>
              )}
            </div>
          ))}
          {library.sources.length === 0 && (
            <div className="row">
              <div className="text">
                <small>Aucun launcher détecté pour l'instant.</small>
              </div>
            </div>
          )}
        </div>

        <div className="group-title">Jeux masqués</div>
        <HiddenVault open={vaultOpen} onOpenChange={onVaultOpen} onShow={onShowHidden} onLibrary={onLibrary} />

        <p className="note" style={{ marginTop: 22 }}>
          Launcher Pro {info.version} · prix affichés pour la région {info.country}
        </p>
      </div>
    </Page>
  );
}
