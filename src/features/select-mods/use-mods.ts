import { useCallback, useState } from "react";
import { launcherApi, type RemoteMod, type Settings } from "@/shared/api/tauri";

export function useMods(settings: Settings, onChange: (next: Settings) => void) {
  const [mods, setMods] = useState<RemoteMod[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async (current = settings) => {
    if (!current.ftp.host) {
      setMods([]);
      return [];
    }
    setLoading(true);
    setError(null);
    try {
      const listed = await launcherApi.ftpListMods();
      setMods(listed);
      if (current.selectedMods.length === 0 && listed.length > 0) {
        onChange({ ...current, selectedMods: listed.map((mod) => mod.name) });
      }
      return listed;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      setMods([]);
      return [];
    } finally {
      setLoading(false);
    }
  }, [onChange, settings]);

  const toggle = useCallback(
    (name: string) => {
      const selected = settings.selectedMods.includes(name)
        ? settings.selectedMods.filter((mod) => mod !== name)
        : [...settings.selectedMods, name];
      onChange({ ...settings, selectedMods: selected });
    },
    [onChange, settings],
  );

  const selectAll = useCallback(
    (all: boolean) => {
      onChange({
        ...settings,
        selectedMods: all ? mods.map((mod) => mod.name) : [],
      });
    },
    [mods, onChange, settings],
  );

  return { mods, loading, error, refresh, toggle, selectAll };
}
