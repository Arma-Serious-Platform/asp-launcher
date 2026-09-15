import { useMemo, useState } from "react";
import { MapIcon, UsersIcon } from "lucide-react";
import { sideColor } from "@/entities/mission/side";
import { formatGameDate, sortedGames, type Weekend } from "@/entities/weekend/model";
import { Card } from "@/shared/ui/atoms/card";
import { Tab } from "@/shared/ui/moleculas/tab";

export const WeekendAnnouncement = ({
  weekend,
  loading,
  error,
}: {
  weekend: Weekend | null;
  loading: boolean;
  error: string | null;
}) => {
  const games = useMemo(() => sortedGames(weekend), [weekend]);
  const [activeIndex, setActiveIndex] = useState(0);
  const active = games[activeIndex] ?? games[0];

  if (loading) {
    return (
      <Card className="flex min-h-72 items-center justify-center text-zinc-400">
        Завантаження анонсів...
      </Card>
    );
  }

  if (error) {
    return (
      <Card className="flex min-h-72 items-center justify-center text-destructive">{error}</Card>
    );
  }

  if (!weekend || !active) {
    return (
      <Card className="flex min-h-72 items-center justify-center text-zinc-400">
        Немає опублікованих ігор
      </Card>
    );
  }

  const image = active.mission?.image?.url;
  const attackSlots = active.missionVersion?.attackSideSlots ?? 0;
  const defenseSlots = active.missionVersion?.defenseSideSlots ?? 0;

  return (
    <Card className="flex min-h-0 flex-1 flex-col overflow-hidden p-0">
      <div className="border-b border-white/10 px-5 py-4">
        <p className="text-xs uppercase tracking-wide text-text-primary">Анонси</p>
        <h2 className="mt-1 text-xl font-semibold">{weekend.name}</h2>
      </div>
      <div className="flex overflow-x-auto border-b border-white/10">
        {games.map((game, index) => (
          <Tab
            key={game.id}
            isActive={index === activeIndex}
            onClick={() => setActiveIndex(index)}
            title={
              <span className="block">
                <span className="block font-medium text-inherit">
                  {game.mission?.name || `Гра ${index + 1}`}
                </span>
                {game.date && (
                  <span className="mt-0.5 block text-xs capitalize text-zinc-500">
                    {formatGameDate(game.date)}
                  </span>
                )}
              </span>
            }
          />
        ))}
      </div>
      <div className="relative min-h-0 flex-1 overflow-hidden bg-black/50">
        {image ? (
          <img
            src={image}
            alt={active.mission?.name ?? ""}
            className="absolute inset-0 h-full w-full object-cover"
          />
        ) : (
          <div className="flex h-full items-center justify-center text-zinc-500">Немає зображення</div>
        )}
        <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black via-black/80 to-transparent p-3">
          {active.mission?.island?.name && (
            <p className="mb-2 inline-flex items-center gap-1.5 text-sm text-zinc-200">
              <MapIcon className="size-4 text-text-primary" />
              {active.mission.island.name}
            </p>
          )}
          <div className="grid gap-2 sm:grid-cols-2">
            <SideBlock
              label="Атака"
              name={active.missionVersion?.attackSideName}
              sideType={active.missionVersion?.attackSideType}
              slots={attackSlots}
            />
            <SideBlock
              label="Оборона"
              name={active.missionVersion?.defenseSideName}
              sideType={active.missionVersion?.defenseSideType}
              slots={defenseSlots}
            />
          </div>
        </div>
      </div>
    </Card>
  );
};

const SideBlock = ({
  label,
  name,
  sideType,
  slots,
}: {
  label: string;
  name?: string | null;
  sideType?: string | null;
  slots: number;
}) => (
  <div className="rounded-md border border-white/10 bg-black/55 px-3 py-2 backdrop-blur-sm">
    <p className="text-[11px] uppercase tracking-wide text-zinc-500">{label}</p>
    <p className={`mt-0.5 text-sm font-medium ${sideColor(sideType)}`}>{name || "—"}</p>
    <p className="mt-1 inline-flex items-center gap-1.5 text-xs text-zinc-400">
      <UsersIcon className="size-4" />
      {slots} слотів
    </p>
  </div>
);
