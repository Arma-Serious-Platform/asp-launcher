import { PlayIcon, DownloadIcon, SquareIcon } from "lucide-react";
import { formatBytes } from "@/entities/mod/format";
import type { DownloadProgress, SyncStatus } from "@/shared/api/tauri";
import { Button } from "@/shared/ui/atoms/button";
import { Progress } from "@/shared/ui/organisms/progress";

export const PlayBar = ({
  canPlay,
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
    <footer className="border-t border-white/10 bg-black/50 px-5 py-4 backdrop-blur">
      <div className="mb-3 flex items-center justify-between gap-4 text-sm text-zinc-400">
        <p className="min-w-0 truncate">
          {error ? (
            <span className="text-destructive">{error}</span>
          ) : busy ? (
            progress.currentFile
              ? `${progress.modName ?? ""} / ${progress.currentFile}`
              : progress.message || "Завантаження..."
          ) : status && !status.ready ? (
            `Потрібно завантажити ${status.filesMissing} файлів (${formatBytes(status.bytesMissing)})`
          ) : armaReady ? (
            "Готово до гри"
          ) : (
            "Вкажіть шлях до Arma 3 у налаштуваннях"
          )}
        </p>
        {busy && progress.bytesTotal > 0 && (
          <span>
            {formatBytes(progress.bytesDone)} / {formatBytes(progress.bytesTotal)}
          </span>
        )}
      </div>
      <div className="flex items-center gap-4">
        <Progress className="flex-1" value={pct} indeterminate={indeterminate} />
        {busy ? (
          <Button variant="secondary" onClick={onCancel}>
            <SquareIcon />
            Скасувати
          </Button>
        ) : (
          <Button
            size="lg"
            disabled={canPlay && !armaReady}
            onClick={canPlay ? onPlay : onInstall}
          >
            {canPlay ? <PlayIcon /> : <DownloadIcon />}
            {label}
          </Button>
        )}
      </div>
    </footer>
  );
};
