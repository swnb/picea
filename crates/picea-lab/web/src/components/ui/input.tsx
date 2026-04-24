import * as React from "react";
import { cn } from "../../lib/utils";

export function Input({ className, ...props }: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      className={cn(
        "h-8 w-full rounded-md border border-lab-line bg-lab-canvas px-2 text-sm text-lab-text",
        "placeholder:text-lab-muted focus-visible:outline-none focus-visible:shadow-focus",
        className,
      )}
      {...props}
    />
  );
}
