import { useCallback, useEffect, useState } from "react";
import { defaultSettings, launcherApi, type Settings } from "@/shared/api/tauri";

function isDesktopUnavailable(err: unknown): boolean {
  const message = err instanceof Error ? err.message : String(err);
  return message.includes("invoke") || message.includes("Tauri") || message.includes("not allowed");
}

export function useSettings() {
  const [settings, setSettings] = useState<Settings>(defaultSettings);
  const [loaded, setLoaded] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        let next = await launcherApi.getSettings();
        if (!next.arma3Path) {
          const detected = await launcherApi.detectArmaPath();
          if (detected) {
            next = await launcherApi.saveSettings({
              ...next,
              arma3Path: detected,
            });
          }
        }
        if (!cancelled) {
          setSettings(next);
        }
      } catch (err) {
        if (!cancelled && !isDesktopUnavailable(err)) {
          setError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (!cancelled) {
          setLoaded(true);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const save = useCallback(async (next: Settings) => {
    setSaving(true);
    setError(null);
    try {
      const stored = await launcherApi.saveSettings(next);
      setSettings(stored);
      return stored;
    } catch (err) {
      const message = isDesktopUnavailable(err)
        ? "Збереження доступне лише в десктопному застосунку"
        : err instanceof Error
          ? err.message
          : String(err);
      setError(message);
      throw new Error(message);
    } finally {
      setSaving(false);
    }
  }, []);

  return { settings, setSettings, save, loaded, saving, error, setError };
}
