import { CheckIcon } from "lucide-react";
import { cn } from "@/shared/lib/cn";

export const Checkbox = ({
  checked,
  onClick,
  className,
  label,
}: {
  checked: boolean;
  onClick?: () => void;
  className?: string;
  label?: string;
}) => (
  <button
    type="button"
    onClick={onClick}
    className={cn("flex items-center gap-2 text-left text-sm text-zinc-100", className)}
  >
    <span
      className={cn(
        "inline-flex size-5 shrink-0 items-center justify-center rounded border",
        checked ? "border-lime-600 bg-lime-700" : "border-neutral-600 bg-black/70",
      )}
    >
      {checked && <CheckIcon className="size-3.5 text-white" />}
    </span>
    {label}
  </button>
);
