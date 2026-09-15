import { cn } from "@/shared/lib/cn";

export const Progress = ({
  value,
  className,
  indeterminate,
}: {
  value: number;
  className?: string;
  indeterminate?: boolean;
}) => {
  const pct = Math.max(0, Math.min(100, Number.isFinite(value) ? value : 0));
  return (
    <div className={cn("h-2 overflow-hidden rounded bg-black/50", className)}>
      <div
        className={cn(
          "h-full bg-lime-700",
          indeterminate && "w-1/3 animate-pulse",
        )}
        style={indeterminate ? undefined : { width: `${pct}%` }}
      />
    </div>
  );
};
