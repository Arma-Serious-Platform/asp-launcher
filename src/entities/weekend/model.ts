export type WeekendGame = {
  id: string;
  date: string;
  position: number;
  mission?: {
    name: string;
    image?: { url: string } | null;
    island?: { name: string } | null;
  } | null;
  missionVersion?: {
    attackSideName?: string | null;
    defenseSideName?: string | null;
    attackSideSlots?: number | null;
    defenseSideSlots?: number | null;
    attackSideType?: string | null;
    defenseSideType?: string | null;
  } | null;
};

export type Weekend = {
  id: string;
  name: string;
  description?: string | null;
  games?: WeekendGame[] | null;
};

export type WeekendsResponse = {
  data?: Weekend[];
  total?: number;
};

export function parseWeekendsPayload(payload: unknown): Weekend | null {
  if (!payload || typeof payload !== "object") {
    return null;
  }
  const data = (payload as WeekendsResponse).data;
  return data?.[0] ?? null;
}

export function sortedGames(weekend: Weekend | null): WeekendGame[] {
  if (!weekend?.games) {
    return [];
  }
  return [...weekend.games].sort((a, b) => a.position - b.position);
}

export function formatGameDate(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return iso;
  }
  const weekday = new Intl.DateTimeFormat("uk-UA", { weekday: "long" }).format(date);
  const day = new Intl.DateTimeFormat("uk-UA", {
    day: "2-digit",
    month: "2-digit",
  }).format(date);
  return `${weekday} ${day}`;
}
