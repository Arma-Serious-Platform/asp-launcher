import { useCallback, useEffect, useState } from "react";
import {
  launcherApi,
  type DownloadProgress,
  type Settings,
  type SyncStatus,
} from "@/shared/api/tauri";

const idleProgress: DownloadProgress = {
  phase: "done",
  bytesDone: 0,
  bytesTotal: 0,
  filesDone: 0,
  filesTotal: 0,
};

export function useSync(settings: Settings, ready: boolean) {
  const [status, setStatus] = useState<SyncStatus | null>(null);
  const [progress, setProgress] = useState<DownloadProgress>(idleProgress);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!settings.ftp.host || settings.selectedMods.length === 0) {
      setStatus({ ready: true, bytesMissing: 0, filesMissing: 0 });
      return;
    }
    try {
      setStatus(await launcherApi.syncStatus());
      setError(null);
    } catch (err) {
      setStatus(null);
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [settings.ftp.host, settings.selectedMods]);

  useEffect(() => {
    if (!ready) {
      return;
    }
    void refresh();
  }, [ready, refresh]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void launcherApi.onDownloadProgress((payload) => {
      setProgress(payload);
      if (payload.phase === "error") {
        setError(payload.message ?? "Помилка завантаження");
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, []);

  const start = useCallback(async () => {
    setBusy(true);
    setError(null);
    setProgress({ ...idleProgress, phase: "listing", message: "Сканування FTP..." });
    try {
      await launcherApi.startSync();
      await refresh();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      setProgress({ ...idleProgress, phase: "error", message });
    } finally {
      setBusy(false);
    }
  }, [refresh]);

  const cancel = useCallback(async () => {
    await launcherApi.cancelSync();
  }, []);

  const canPlay = Boolean(status?.ready) && !busy;

  return { status, progress, busy, error, start, cancel, refresh, canPlay };
}
