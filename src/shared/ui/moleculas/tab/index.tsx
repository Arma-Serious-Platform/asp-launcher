import type { ReactNode } from "react";
import { cn } from "@/shared/lib/cn";

export const Tab = ({
  title,
  isActive,
  onClick,
  className,
}: {
  title: ReactNode;
  isActive: boolean;
  onClick: () => void;
  className?: string;
}) => (
  <button
    type="button"
    onClick={onClick}
    className={cn(
      "min-w-[8rem] flex-1 border-b-2 px-3 py-2 text-left transition-colors cursor-pointer",
      isActive
        ? "border-lime-600 bg-lime-700/10 text-lime-200"
        : "border-transparent text-zinc-400 hover:bg-white/5 hover:text-zinc-200",
      className,
    )}
  >
    {title}
  </button>
);
