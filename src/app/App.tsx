import { useEffect, useState } from "react";
import { useWeekend } from "@/entities/weekend/use-weekend";
import { useSettings } from "@/features/configure-settings/use-settings";
import { useLaunch } from "@/features/launch-game/use-launch";
import { useSync } from "@/features/sync-mods/use-sync";
import { Header } from "@/widgets/header";
import { PlayBar } from "@/widgets/play-bar";
import { SettingsDialog } from "@/widgets/settings-dialog";
import { WeekendAnnouncement } from "@/widgets/weekend-announcement";

export default function App() {
  const { settings, save, loaded, saving, error: settingsError } = useSettings();
  const weekend = useWeekend();
  const sync = useSync(settings, loaded);
  const launch = useLaunch();
  const [settingsOpen, setSettingsOpen] = useState(false);

  useEffect(() => {
    if (loaded && !settings.arma3Path) {
      setSettingsOpen(true);
    }
  }, [loaded, settings.arma3Path]);

  const armaReady = Boolean(settings.arma3Path);
  const actionError = launch.error || sync.error;

  return (
    <div className="flex h-screen flex-col bg-[#0a0a0a] text-foreground">
      <Header onOpenSettings={() => setSettingsOpen(true)} />
      <main className="flex min-h-0 flex-1 flex-col p-5">
        <WeekendAnnouncement
          weekend={weekend.weekend}
          loading={weekend.loading}
          error={weekend.error}
        />
      </main>
      <PlayBar
        canPlay={sync.canPlay}
        busy={sync.busy}
        status={sync.status}
        progress={sync.progress}
        armaReady={armaReady}
        error={actionError}
        onInstall={() => void sync.start()}
        onPlay={() => void launch.launch()}
        onCancel={() => void sync.cancel()}
      />
      <SettingsDialog
        open={settingsOpen}
        settings={settings}
        saving={saving}
        error={settingsError}
        onOpenChange={setSettingsOpen}
        onSave={async (next) => {
          await save(next);
          await sync.refresh();
        }}
      />
    </div>
  );
}
