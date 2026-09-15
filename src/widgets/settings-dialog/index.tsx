import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderSearchIcon } from "lucide-react";
import { formatBytes } from "@/entities/mod/format";
import { useMods } from "@/features/select-mods/use-mods";
import { launcherApi, type Settings } from "@/shared/api/tauri";
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
  const mods = useMods(draft, setDraft);

  useEffect(() => {
    if (opened) {
      setDraft(settings);
      setFtpMessage(null);
    }
  }, [opened, settings]);

  useEffect(() => {
    if (opened && draft.ftp.host) {
      void mods.refresh(draft);
    }
    // Refresh once when the dialog opens with an FTP host.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [opened]);

  const patch = (partial: Partial<Settings>) => setDraft({ ...draft, ...partial });
  const patchFtp = (partial: Partial<Settings["ftp"]>) =>
    setDraft({ ...draft, ftp: { ...draft.ftp, ...partial } });

  const pickFolder = async (field: "arma3Path" | "modsPath") => {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected === "string") {
      patch({ [field]: selected });
    }
  };

  const detect = async () => {
    const detected = await launcherApi.detectArmaPath();
    if (detected) {
      patch({
        arma3Path: detected,
        modsPath: draft.modsPath || detected,
      });
    } else {
      setFtpMessage("Arma 3 не знайдено автоматично. Оберіть теку вручну.");
    }
  };

  const testFtp = async () => {
    setFtpBusy(true);
    setFtpMessage(null);
    try {
      await onSave(draft);
      await launcherApi.ftpTestConnection();
      await mods.refresh({ ...draft });
      setFtpMessage("Підключення успішне");
    } catch (err) {
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
          <DialogTitle>Налаштування</DialogTitle>
          <DialogDescription>Шлях до гри, FTP і список модів</DialogDescription>
        </DialogHeader>

        <section className="grid gap-3">
          <h3 className="text-sm font-medium text-text-primary">Arma 3</h3>
          <div className="flex items-end gap-2">
            <Input
              label="Шлях до гри"
              value={draft.arma3Path}
              onChange={(e) => patch({ arma3Path: e.target.value })}
            />
            <Button variant="secondary" type="button" onClick={() => void pickFolder("arma3Path")}>
              Огляд
            </Button>
            <Button variant="outline" type="button" onClick={() => void detect()}>
              <FolderSearchIcon />
              Знайти
            </Button>
          </div>
          <div className="flex items-end gap-2">
            <Input
              label="Тека модів"
              value={draft.modsPath}
              onChange={(e) => patch({ modsPath: e.target.value })}
            />
            <Button variant="secondary" type="button" onClick={() => void pickFolder("modsPath")}>
              Огляд
            </Button>
          </div>
        </section>

        <section className="grid gap-3">
          <h3 className="text-sm font-medium text-text-primary">FTP</h3>
          <div className="grid gap-3 sm:grid-cols-[1fr_7rem]">
            <Input
              label="Хост"
              value={draft.ftp.host}
              onChange={(e) => patchFtp({ host: e.target.value })}
            />
            <Input
              label="Порт"
              type="number"
              value={draft.ftp.port}
              onChange={(e) => patchFtp({ port: Number(e.target.value) || 21 })}
            />
          </div>
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
          <Input
            label="Віддалена тека"
            value={draft.ftp.remotePath}
            onChange={(e) => patchFtp({ remotePath: e.target.value })}
          />
          <Checkbox
            checked={draft.ftp.passive}
            onClick={() => patchFtp({ passive: !draft.ftp.passive })}
            label="Пасивний режим"
          />
          <div className="flex items-center gap-3">
            <Button variant="outline" type="button" disabled={ftpBusy} onClick={() => void testFtp()}>
              Перевірити FTP
            </Button>
            {(ftpMessage || mods.error) && (
              <p className="text-sm text-zinc-400">{ftpMessage || mods.error}</p>
            )}
          </div>
        </section>

        <section className="grid gap-3">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-medium text-text-primary">Моди</h3>
            <div className="flex gap-2">
              <Button variant="ghost" size="sm" type="button" onClick={() => mods.selectAll(true)}>
                Всі
              </Button>
              <Button variant="ghost" size="sm" type="button" onClick={() => mods.selectAll(false)}>
                Жодного
              </Button>
            </div>
          </div>
          <div className="max-h-48 space-y-1 overflow-y-auto rounded-md border border-white/10 bg-black/30 p-2">
            {mods.loading && <p className="p-2 text-sm text-zinc-400">Завантаження списку модів...</p>}
            {!mods.loading && mods.mods.length === 0 && (
              <p className="p-2 text-sm text-zinc-500">Немає модів. Перевірте FTP.</p>
            )}
            {mods.mods.map((mod) => (
              <Checkbox
                key={mod.name}
                className="w-full rounded px-2 py-1.5 hover:bg-white/5"
                checked={draft.selectedMods.includes(mod.name)}
                onClick={() => mods.toggle(mod.name)}
                label={`${mod.name} (${formatBytes(mod.remoteBytes)})`}
              />
            ))}
          </div>
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
