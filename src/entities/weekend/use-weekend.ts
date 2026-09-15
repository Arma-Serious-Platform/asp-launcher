import { useEffect, useState } from "react";
import { launcherApi } from "@/shared/api/tauri";
import { WEEKENDS_API, WEEKENDS_DEV_API } from "@/shared/config";
import { parseWeekendsPayload, type Weekend } from "./model";

async function loadWeekends(): Promise<unknown> {
  try {
    return await launcherApi.fetchWeekends();
  } catch {
    const urls = [WEEKENDS_API, WEEKENDS_DEV_API];
    let lastError: unknown;
    for (const url of urls) {
      try {
        const response = await fetch(url);
        if (!response.ok) {
          throw new Error(`API анонсів повернуло ${response.status}`);
        }
        return await response.json();
      } catch (err) {
        lastError = err;
      }
    }
    throw lastError instanceof Error ? lastError : new Error("Не вдалося отримати анонси");
  }
}

export function useWeekend() {
  const [weekend, setWeekend] = useState<Weekend | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const payload = await loadWeekends();
        if (!cancelled) {
          setWeekend(parseWeekendsPayload(payload));
        }
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return { weekend, loading, error };
}
