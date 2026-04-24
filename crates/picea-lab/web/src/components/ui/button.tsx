import * as React from "react";
import { cn } from "../../lib/utils";

type ButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "default" | "ghost" | "outline" | "danger";
  size?: "sm" | "md" | "icon";
};

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { className, variant = "default", size = "md", type = "button", ...props },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      className={cn(
        "inline-flex shrink-0 items-center justify-center gap-2 rounded-md border text-sm font-medium transition-colors",
        "focus-visible:outline-none focus-visible:shadow-focus disabled:pointer-events-none disabled:opacity-45",
        variant === "default" &&
          "border-lab-accent/70 bg-lab-accent/15 text-lab-text hover:bg-lab-accent/25",
        variant === "ghost" && "border-transparent bg-transparent text-lab-muted hover:bg-white/[0.07] hover:text-lab-text",
        variant === "outline" && "border-lab-line bg-lab-panel2 text-lab-text hover:bg-white/[0.07]",
        variant === "danger" && "border-lab-danger/60 bg-lab-danger/[0.12] text-lab-text hover:bg-lab-danger/20",
        size === "sm" && "h-7 px-2.5",
        size === "md" && "h-8 px-3",
        size === "icon" && "h-8 w-8 p-0",
        className,
      )}
      {...props}
    />
  );
});
