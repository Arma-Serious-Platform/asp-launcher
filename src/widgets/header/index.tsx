import { SettingsIcon } from "lucide-react";
import { LOGO_URL } from "@/shared/config";
import { Button } from "@/shared/ui/atoms/button";

export const Header = ({ onOpenSettings }: { onOpenSettings: () => void }) => (
  <header className="flex items-center justify-between border-b border-white/10 bg-black/40 px-5 py-3 backdrop-blur">
    <div className="flex items-center gap-3">
      <img src={LOGO_URL} alt="VTG" className="h-10 w-10 object-contain" />
      <div>
        <h1 className="text-lg font-semibold leading-none">ASP Launcher</h1>
        <p className="mt-1 text-xs text-zinc-400">Virtual Tactical Games</p>
      </div>
    </div>
    <Button variant="ghost" onClick={onOpenSettings}>
      <SettingsIcon />
      Налаштування
    </Button>
  </header>
);
