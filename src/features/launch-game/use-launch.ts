import { useCallback, useState } from "react";
import { launcherApi } from "@/shared/api/tauri";

export function useLaunch() {
  const [error, setError] = useState<string | null>(null);
  const [launching, setLaunching] = useState(false);

  const launch = useCallback(async () => {
    setLaunching(true);
    setError(null);
    try {
      await launcherApi.launchGame();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLaunching(false);
    }
  }, []);

  return { launch, launching, error };
}
