import { PlayIcon, DownloadIcon, SquareIcon } from "lucide-react";
import { formatBytes } from "@/entities/mod/format";
import type { DownloadProgress, SyncStatus } from "@/shared/api/tauri";
import { Button } from "@/shared/ui/atoms/button";
import { Progress } from "@/shared/ui/organisms/progress";

export const PlayBar = ({
  canPlay,
  canInstall,
  busy,
  status,
  progress,
  armaReady,
  error,
  onInstall,
  onPlay,
  onCancel,
}: {
  canPlay: boolean;
  canInstall: boolean;
  busy: boolean;
  status: SyncStatus | null;
  progress: DownloadProgress;
  armaReady: boolean;
  error: string | null;
  onInstall: () => void;
  onPlay: () => void;
  onCancel: () => void;
}) => {
  const pct =
    busy && progress.bytesTotal > 0 ? (progress.bytesDone / progress.bytesTotal) * 100 : 0;
  const indeterminate = busy && (progress.phase === "listing" || progress.bytesTotal === 0);
  const label = canPlay ? "Грати" : "Встановити";

  return (
    <div className="border-t border-white/10 bg-black/50 px-3 py-3 backdrop-blur">
      <p className="mb-2 min-w-0 truncate text-sm text-zinc-400">
        {error ? (
          <span className="text-destructive">{error}</span>
        ) : busy ? (
          progress.currentFile
            ? `${progress.modName ?? ""} / ${progress.currentFile}`
            : progress.message || "Завантаження..."
        ) : !canPlay && !canInstall ? (
          "Вкажіть теку аддонів"
        ) : status && !status.ready ? (
          `Потрібно завантажити ${status.filesMissing} файлів (${formatBytes(status.bytesMissing)})`
        ) : armaReady ? (
          "Готово до гри"
        ) : (
          "Вкажіть шлях до Arma 3"
        )}
      </p>
      <div className="flex items-center gap-3">
        <Progress className="flex-1" value={pct} indeterminate={indeterminate} />
        {busy && progress.bytesTotal > 0 && (
          <span className="shrink-0 text-xs text-zinc-500">
            {formatBytes(progress.bytesDone)} / {formatBytes(progress.bytesTotal)}
          </span>
        )}
        {busy ? (
          <Button variant="secondary" size="lg" onClick={onCancel}>
            <SquareIcon />
            Скасувати
          </Button>
        ) : (
          <Button
            size="xl"
            className="min-w-0 shrink-0 px-6 w-full"
            disabled={canPlay ? !armaReady : !canInstall}
            onClick={canPlay ? onPlay : onInstall}
          >
            {canPlay ? <PlayIcon /> : <DownloadIcon />}
            {label}
          </Button>
        )}
      </div>
    </div>
  );
};
