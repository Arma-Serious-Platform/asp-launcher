import type { ReactNode } from "react";
import { cn } from "@/shared/lib/cn";

export const Card = ({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) => {
  return <div className={cn("paper bg-card p-4", className)}>{children}</div>;
};
