import { useState } from "react";
import { useWeekend } from "@/entities/weekend/use-weekend";
import { useSettings } from "@/features/configure-settings/use-settings";
import { useLaunch } from "@/features/launch-game/use-launch";
import { useSync } from "@/features/sync-mods/use-sync";
import { Header } from "@/widgets/header";
import { LauncherControls } from "@/widgets/launcher-controls";
import { PlayBar } from "@/widgets/play-bar";
import { ServerStatus } from "@/widgets/server-status";
import { SettingsDialog } from "@/widgets/settings-dialog";
import { WeekendAnnouncement } from "@/widgets/weekend-announcement";

export default function App() {
  const { settings, save, loaded, saving, error: settingsError } = useSettings();
  const weekend = useWeekend();
  const sync = useSync(settings, loaded);
  const launch = useLaunch();
  const [settingsOpen, setSettingsOpen] = useState(false);

  const armaReady = Boolean(settings.arma3Path);
  const canInstall = Boolean(settings.modsPath.trim());
  const actionError = launch.error || sync.error;

  const persist = async (next: typeof settings) => {
    await save(next);
    await sync.refresh();
  };

  return (
    <div className="flex h-screen flex-col bg-[#0a0a0a] text-foreground">
      <Header onOpenSettings={() => setSettingsOpen(true)} />
      <main className="grid min-h-0 flex-1 grid-cols-[1fr_minmax(22rem,28rem)] gap-4 p-5">
        <section className="flex min-h-0 flex-col gap-4">
          <ServerStatus />
          <WeekendAnnouncement
            weekend={weekend.weekend}
            loading={weekend.loading}
            error={weekend.error}
          />
        </section>
        <section className="flex min-h-0 flex-col overflow-hidden rounded-xl border border-white/10 bg-black/40">
          <LauncherControls settings={settings} loaded={loaded} onChange={persist} />
          <PlayBar
            canPlay={sync.canPlay}
            canInstall={canInstall}
            busy={sync.busy}
            status={sync.status}
            progress={sync.progress}
            armaReady={armaReady}
            error={actionError}
            onInstall={() => void sync.start()}
            onPlay={() => void launch.launch()}
            onCancel={() => void sync.cancel()}
          />
        </section>
      </main>
      <SettingsDialog
        open={settingsOpen}
        settings={settings}
        saving={saving}
        error={settingsError}
        onOpenChange={setSettingsOpen}
        onSave={persist}
      />
    </div>
  );
}
