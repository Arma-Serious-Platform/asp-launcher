export type GameServerInfo = {
  name?: string | null;
  game?: string | null;
  map?: string | null;
  maxPlayers?: number | null;
  players?: number | null;
  ping?: number | null;
};

export type GameServer = {
  id: string;
  name: string;
  status: string;
  ip: string;
  port: number;
  info?: GameServerInfo | null;
};

export function parseServersPayload(payload: unknown): GameServer[] {
  if (!Array.isArray(payload)) {
    return [];
  }
  return payload.filter((item): item is GameServer => {
    if (!item || typeof item !== "object") {
      return false;
    }
    const server = item as Partial<GameServer>;
    return typeof server.id === "string" && typeof server.ip === "string";
  });
}

export function firstActiveServer(servers: GameServer[]): GameServer | null {
  return servers.find((server) => server.status === "ACTIVE") ?? servers[0] ?? null;
}
