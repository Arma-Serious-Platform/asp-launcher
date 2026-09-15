import { useEffect, useState } from "react";
import { launcherApi } from "@/shared/api/tauri";
import { SERVERS_API, SERVERS_DEV_API } from "@/shared/config";
import { firstActiveServer, parseServersPayload, type GameServer } from "./model";

async function loadServers(): Promise<unknown> {
  try {
    return await launcherApi.fetchServers();
  } catch {
    const urls = [SERVERS_API, SERVERS_DEV_API];
    let lastError: unknown;
    for (const url of urls) {
      try {
        const response = await fetch(url);
        if (!response.ok) {
          throw new Error(`API серверів повернуло ${response.status}`);
        }
        return await response.json();
      } catch (err) {
        lastError = err;
      }
    }
    throw lastError instanceof Error ? lastError : new Error("Не вдалося отримати статус серверів");
  }
}

export function useServers() {
  const [server, setServer] = useState<GameServer | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const payload = await loadServers();
        if (!cancelled) {
          setServer(firstActiveServer(parseServersPayload(payload)));
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

  return { server, loading, error };
}
