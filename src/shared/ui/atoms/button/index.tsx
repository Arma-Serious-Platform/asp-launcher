import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/shared/lib/cn";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md border text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4 shrink-0 [&_svg]:shrink-0 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-lime-500/60 focus-visible:ring-offset-2 focus-visible:ring-offset-black cursor-pointer",
  {
    variants: {
      variant: {
        default:
          "bg-lime-700 border-lime-600 text-white hover:bg-lime-600 hover:border-lime-500 active:bg-lime-700/90",
        destructive: "bg-destructive text-white border-red-700 hover:bg-destructive/90",
        outline:
          "bg-transparent border-lime-700 text-lime-300 hover:bg-lime-700/10 hover:text-lime-200",
        secondary:
          "bg-neutral-800 border-neutral-600 text-zinc-100 hover:bg-neutral-700 hover:border-neutral-500",
        ghost: "border-transparent bg-transparent text-zinc-200 hover:bg-white/5 hover:border-white/10",
      },
      size: {
        default: "h-9 px-4 py-2 has-[>svg]:px-3",
        sm: "h-8 gap-1.5 px-3 has-[>svg]:px-2.5 text-sm",
        lg: "h-12 px-8 has-[>svg]:px-5 text-lg",
        icon: "size-9",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  },
);

function Button({
  className,
  variant,
  size,
  asChild = false,
  ...props
}: React.ComponentProps<"button"> &
  VariantProps<typeof buttonVariants> & {
    asChild?: boolean;
  }) {
  const Comp = asChild ? Slot : "button";
  return <Comp className={cn(buttonVariants({ variant, size }), className)} {...props} />;
}

export { Button, buttonVariants };
