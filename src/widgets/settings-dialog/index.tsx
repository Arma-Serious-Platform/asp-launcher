import { useEffect, useState } from "react";
import { launcherApi, type FtpProbe, type Settings } from "@/shared/api/tauri";
import { Button } from "@/shared/ui/atoms/button";
import { Checkbox } from "@/shared/ui/atoms/checkbox";
import { Input } from "@/shared/ui/atoms/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/organisms/dialog";

export const SettingsDialog = ({
  open: opened,
  settings,
  saving,
  error,
  onOpenChange,
  onSave,
}: {
  open: boolean;
  settings: Settings;
  saving: boolean;
  error: string | null;
  onOpenChange: (open: boolean) => void;
  onSave: (settings: Settings) => Promise<void>;
}) => {
  const [draft, setDraft] = useState(settings);
  const [ftpMessage, setFtpMessage] = useState<string | null>(null);
  const [ftpBusy, setFtpBusy] = useState(false);
  const [probe, setProbe] = useState<FtpProbe | null>(null);

  useEffect(() => {
    if (opened) {
      setDraft(settings);
      setFtpMessage(null);
      setProbe(null);
    }
  }, [opened, settings]);

  const patchFtp = (partial: Partial<Settings["ftp"]>) =>
    setDraft({ ...draft, ftp: { ...draft.ftp, ...partial } });

  const testFtp = async () => {
    setFtpBusy(true);
    setFtpMessage(null);
    try {
      await onSave(draft);
      const result = await launcherApi.ftpTestConnection();
      setProbe(result);
      setFtpMessage(result.hasSync ? "Підключення успішне (A3S sync)" : "Підключення успішне");
    } catch (err) {
      setProbe(null);
      setFtpMessage(err instanceof Error ? err.message : String(err));
    } finally {
      setFtpBusy(false);
    }
  };

  const save = async () => {
    await onSave(draft);
    onOpenChange(false);
  };

  return (
    <Dialog open={opened} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[88vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Налаштування FTP</DialogTitle>
          <DialogDescription>Джерело аддонів: простий FTP або Arma3Sync autoconfig</DialogDescription>
        </DialogHeader>

        <section className="grid gap-3">
          <Input
            label="FTP / autoconfig URL"
            value={draft.ftp.sourceUrl}
            onChange={(e) => patchFtp({ sourceUrl: e.target.value })}
            placeholder="ftp://host/path/.a3s/autoconfig"
          />
          <div className="grid gap-3 sm:grid-cols-2">
            <Input
              label="Логін"
              value={draft.ftp.username}
              onChange={(e) => patchFtp({ username: e.target.value })}
            />
            <Input
              label="Пароль"
              type="password"
              value={draft.ftp.password}
              onChange={(e) => patchFtp({ password: e.target.value })}
            />
          </div>
          <div className="grid gap-3 sm:grid-cols-[7rem_1fr]">
            <Input
              label="Порт"
              type="number"
              value={draft.ftp.port}
              onChange={(e) => patchFtp({ port: Number(e.target.value) || 21 })}
            />
            <div className="flex items-end pb-1">
              <Checkbox
                checked={draft.ftp.passive}
                onClick={() => patchFtp({ passive: !draft.ftp.passive })}
                label="Пасивний режим"
              />
            </div>
          </div>
          <div className="flex items-center gap-3">
            <Button variant="outline" type="button" disabled={ftpBusy} onClick={() => void testFtp()}>
              Перевірити
            </Button>
            {ftpMessage && <p className="text-sm text-zinc-400">{ftpMessage}</p>}
          </div>
          {probe && (
            <p className="text-sm text-zinc-400">
              {probe.repositoryName ? `${probe.repositoryName}: ` : ""}
              {probe.host}:{probe.port}
              {probe.remotePath}
              {probe.mode === "a3s" ? " (A3S)" : " (FTP)"}
            </p>
          )}
        </section>

        {error && <p className="text-sm text-destructive">{error}</p>}

        <DialogFooter>
          <Button variant="secondary" type="button" onClick={() => onOpenChange(false)}>
            Скасувати
          </Button>
          <Button type="button" disabled={saving} onClick={() => void save()}>
            Зберегти
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};
