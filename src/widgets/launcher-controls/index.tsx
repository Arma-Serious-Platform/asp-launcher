import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderSearchIcon } from "lucide-react";
import { formatBytes } from "@/entities/mod/format";
import { useMods } from "@/features/select-mods/use-mods";
import { launcherApi, type Settings } from "@/shared/api/tauri";
import { Button } from "@/shared/ui/atoms/button";
import { Checkbox } from "@/shared/ui/atoms/checkbox";

export const LauncherControls = ({
  settings,
  loaded,
  onChange,
}: {
  settings: Settings;
  loaded: boolean;
  onChange: (next: Settings) => Promise<void> | void;
}) => {
  const mods = useMods(settings, onChange);
  const [arma3Path, setArma3Path] = useState(settings.arma3Path);
  const [modsPath, setModsPath] = useState(settings.modsPath);
  const saveTimer = useRef<number | null>(null);

  useEffect(() => {
    setArma3Path(settings.arma3Path);
    setModsPath(settings.modsPath);
  }, [settings.arma3Path, settings.modsPath]);

  useEffect(() => {
    if (loaded) {
      void mods.refresh(settings);
    }
    // List once the source URL is available.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loaded, settings.ftp.sourceUrl, settings.ftp.host]);

  const persistPaths = (nextArma: string, nextMods: string, immediate = false) => {
    const apply = () => {
      if (nextArma === settings.arma3Path && nextMods === settings.modsPath) {
        return;
      }
      void onChange({ ...settings, arma3Path: nextArma, modsPath: nextMods });
    };
    if (saveTimer.current) {
      window.clearTimeout(saveTimer.current);
      saveTimer.current = null;
    }
    if (immediate) {
      apply();
      return;
    }
    saveTimer.current = window.setTimeout(apply, 400);
  };

  const pickFolder = async (field: "arma3Path" | "modsPath") => {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected !== "string") {
      return;
    }
    const nextArma = field === "arma3Path" ? selected : arma3Path;
    const nextMods = field === "modsPath" ? selected : modsPath;
    setArma3Path(nextArma);
    setModsPath(nextMods);
    await onChange({ ...settings, arma3Path: nextArma, modsPath: nextMods });
  };

  const detect = async () => {
    const detected = await launcherApi.detectArmaPath();
    if (detected) {
      setArma3Path(detected);
      await onChange({
        ...settings,
        arma3Path: detected,
      });
    }
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden p-3">
      <div className="grid shrink-0 gap-1.5">
        <div className="flex min-w-0 items-center gap-2">
          <span className="w-28 shrink-0 text-xs text-zinc-400">Шлях до гри</span>
          <input
            aria-label="Шлях до гри"
            className="h-8 min-w-0 flex-1 rounded-md border border-neutral-700 bg-black/70 px-2 text-sm text-zinc-100 focus-visible:border-lime-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lime-500/40"
            value={arma3Path}
            onChange={(e) => {
              setArma3Path(e.target.value);
              persistPaths(e.target.value, modsPath);
            }}
            onBlur={() => persistPaths(arma3Path, modsPath, true)}
          />
          <Button
            className="shrink-0"
            size="sm"
            variant="secondary"
            type="button"
            onClick={() => void pickFolder("arma3Path")}
          >
            Огляд
          </Button>
          <Button
            className="shrink-0"
            size="sm"
            variant="outline"
            type="button"
            onClick={() => void detect()}
          >
            <FolderSearchIcon />
            Знайти
          </Button>
        </div>
        <div className="flex min-w-0 items-center gap-2">
          <span className="w-28 shrink-0 text-xs text-zinc-400">Тека аддонів</span>
          <input
            aria-label="Тека аддонів"
            className="h-8 min-w-0 flex-1 rounded-md border border-neutral-700 bg-black/70 px-2 text-sm text-zinc-100 focus-visible:border-lime-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lime-500/40"
            value={modsPath}
            onChange={(e) => {
              setModsPath(e.target.value);
              persistPaths(arma3Path, e.target.value);
            }}
            onBlur={() => persistPaths(arma3Path, modsPath, true)}
          />
          <Button
            className="shrink-0"
            size="sm"
            variant="secondary"
            type="button"
            onClick={() => void pickFolder("modsPath")}
          >
            Огляд
          </Button>
        </div>
      </div>

      <div className="mt-3 flex min-h-0 flex-1 flex-col">
        <div className="mb-2 flex items-center justify-between">
          <h3 className="text-sm font-medium text-text-primary">Аддони</h3>
          <div className="flex gap-2">
            <Button variant="ghost" size="sm" type="button" onClick={() => mods.selectAll(true)}>
              Всі
            </Button>
            <Button variant="ghost" size="sm" type="button" onClick={() => mods.selectAll(false)}>
              Жодного
            </Button>
          </div>
        </div>
        <div className="flex min-h-0 flex-1 flex-col overflow-y-auto rounded-md border border-white/10 bg-black/30 p-1">
          {mods.loading && <p className="p-2 text-sm text-zinc-400">Завантаження списку аддонів...</p>}
          {!mods.loading && mods.mods.length === 0 && (
            <p className="p-2 text-sm text-zinc-500">
              {mods.error || "Немає аддонів. Перевірте FTP у налаштуваннях."}
            </p>
          )}
          {mods.mods.map((mod) => (
            <Checkbox
              key={mod.name}
              className="w-full shrink-0 rounded px-2 py-1.5 hover:bg-white/5"
              checked={settings.selectedMods.includes(mod.name)}
              onClick={() => mods.toggle(mod.name)}
              label={
                <>
                  <span className="min-w-0 flex-1 truncate">{mod.name}</span>
                  <span className="shrink-0 text-zinc-500">{formatBytes(mod.remoteBytes)}</span>
                </>
              }
            />
          ))}
        </div>
        {mods.error && mods.mods.length > 0 && (
          <p className="mt-2 text-sm text-destructive">{mods.error}</p>
        )}
      </div>
    </div>
  );
};
