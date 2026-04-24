import * as React from "react";
import { cn } from "../../lib/utils";

type BadgeProps = React.HTMLAttributes<HTMLSpanElement> & {
  tone?: "neutral" | "accent" | "warn" | "danger" | "green";
};

export function Badge({ className, tone = "neutral", ...props }: BadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex h-5 items-center rounded border px-1.5 text-[11px] font-medium uppercase tracking-normal",
        tone === "neutral" && "border-lab-line bg-white/5 text-lab-muted",
        tone === "accent" && "border-lab-accent/50 bg-lab-accent/[0.12] text-lab-accent",
        tone === "warn" && "border-lab-warn/50 bg-lab-warn/[0.12] text-lab-warn",
        tone === "danger" && "border-lab-danger/50 bg-lab-danger/[0.12] text-lab-danger",
        tone === "green" && "border-lab-green/50 bg-lab-green/[0.12] text-lab-green",
        className,
      )}
      {...props}
    />
  );
}
