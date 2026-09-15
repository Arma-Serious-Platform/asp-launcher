import { useServers } from "@/entities/server/use-servers";
import { Card } from "@/shared/ui/atoms/card";

export const ServerStatus = () => {
  const { server, loading, error } = useServers();
  const info = server?.info;
  const online = server?.status === "ACTIVE";

  return (
    <Card className="shrink-0">
      <p className="text-[11px] uppercase tracking-wide text-text-primary">Сервер</p>
      {loading ? (
        <p className="mt-2 text-sm text-zinc-500">Перевірка сервера...</p>
      ) : error || !server ? (
        <p className="mt-2 text-sm text-zinc-500">{error || "Немає даних про сервер"}</p>
      ) : (
        <div className="mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-sm">
          <span className="inline-flex items-center gap-2 font-medium">
            <span className={`size-2.5 rounded-full ${online ? "bg-lime-500" : "bg-zinc-500"}`} />
            {info?.players ?? 0}/{info?.maxPlayers ?? 0}
          </span>
          {info?.map && <span className="text-zinc-300">{info.map}</span>}
          {info?.game && <span className="truncate text-zinc-400">{info.game}</span>}
          <span className="text-zinc-500">
            {server.ip}:{server.port}
          </span>
          {typeof info?.ping === "number" && <span className="text-zinc-400">{info.ping} ms</span>}
        </div>
      )}
    </Card>
  );
};
