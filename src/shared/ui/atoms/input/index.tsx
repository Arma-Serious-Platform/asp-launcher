import * as React from "react";
import { cn } from "@/shared/lib/cn";

export type InputProps = React.InputHTMLAttributes<HTMLInputElement> & {
  label?: string;
  error?: string;
};

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, label, error, id, ...props }, ref) => {
    const inputId = id ?? label?.toLowerCase().replace(/\s+/g, "-");
    return (
      <label className="flex w-full flex-col gap-1 text-sm text-zinc-200">
        {label && <span className="text-zinc-400">{label}</span>}
        <input
          id={inputId}
          ref={ref}
          className={cn(
            "flex h-9 w-full rounded-md border border-neutral-700 bg-black/70 px-2 py-1 text-sm text-zinc-100 shadow-sm transition-colors placeholder:text-zinc-500 focus-visible:border-lime-500 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lime-500/40 disabled:cursor-not-allowed disabled:opacity-45",
            error && "border-destructive",
            className,
          )}
          {...props}
        />
        {error && <span className="text-xs text-destructive">{error}</span>}
      </label>
    );
  },
);

Input.displayName = "Input";
