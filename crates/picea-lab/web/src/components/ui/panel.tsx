import * as React from "react";
import { cn } from "../../lib/utils";

export function PanelHeader({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "flex h-9 shrink-0 items-center justify-between border-b border-lab-line bg-lab-panel px-3",
        className,
      )}
      {...props}
    />
  );
}

export function PanelTitle({ className, ...props }: React.HTMLAttributes<HTMLHeadingElement>) {
  return <h2 className={cn("text-xs font-semibold uppercase tracking-normal text-lab-muted", className)} {...props} />;
}
