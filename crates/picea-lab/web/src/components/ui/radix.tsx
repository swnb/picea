import * as CheckboxPrimitive from "@radix-ui/react-checkbox";
import * as SelectPrimitive from "@radix-ui/react-select";
import * as SliderPrimitive from "@radix-ui/react-slider";
import * as TooltipPrimitive from "@radix-ui/react-tooltip";
import type { ReactNode } from "react";
import { Check, ChevronDown } from "lucide-react";
import { cn } from "../../lib/utils";

export function Tooltip({ label, children }: { label: string; children: ReactNode }) {
  return (
    <TooltipPrimitive.Provider delayDuration={250}>
      <TooltipPrimitive.Root>
        <TooltipPrimitive.Trigger asChild>{children}</TooltipPrimitive.Trigger>
        <TooltipPrimitive.Portal>
          <TooltipPrimitive.Content
            sideOffset={6}
            className="z-50 rounded-md border border-lab-line bg-lab-panel2 px-2 py-1 text-xs text-lab-text shadow-xl"
          >
            {label}
            <TooltipPrimitive.Arrow className="fill-lab-panel2" />
          </TooltipPrimitive.Content>
        </TooltipPrimitive.Portal>
      </TooltipPrimitive.Root>
    </TooltipPrimitive.Provider>
  );
}

export function Checkbox({
  checked,
  onCheckedChange,
  label,
}: {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  label: string;
}) {
  return (
    <label className="flex h-7 cursor-pointer items-center gap-2 text-sm text-lab-text">
      <CheckboxPrimitive.Root
        checked={checked}
        onCheckedChange={(value) => onCheckedChange(value === true)}
        className="grid h-4 w-4 place-items-center rounded border border-lab-line bg-lab-canvas data-[state=checked]:border-lab-accent data-[state=checked]:bg-lab-accent/20"
      >
        <CheckboxPrimitive.Indicator>
          <Check className="h-3 w-3 text-lab-accent" />
        </CheckboxPrimitive.Indicator>
      </CheckboxPrimitive.Root>
      <span>{label}</span>
    </label>
  );
}

export function Slider({
  value,
  min,
  max,
  step,
  onValueChange,
  className,
}: {
  value: number;
  min: number;
  max: number;
  step?: number;
  onValueChange: (value: number) => void;
  className?: string;
}) {
  return (
    <SliderPrimitive.Root
      value={[value]}
      min={min}
      max={max}
      step={step}
      onValueChange={([next]) => onValueChange(next)}
      className={cn("relative flex h-6 touch-none select-none items-center", className)}
    >
      <SliderPrimitive.Track className="relative h-1 grow rounded-full bg-lab-line">
        <SliderPrimitive.Range className="absolute h-full rounded-full bg-lab-accent" />
      </SliderPrimitive.Track>
      <SliderPrimitive.Thumb className="block h-4 w-4 rounded-full border border-lab-accent bg-lab-panel2 shadow focus-visible:outline-none focus-visible:shadow-focus" />
    </SliderPrimitive.Root>
  );
}

export function Select({
  value,
  onValueChange,
  items,
  className,
  ariaLabel,
}: {
  value: string;
  onValueChange: (value: string) => void;
  items: Array<{ value: string; label: string }>;
  className?: string;
  ariaLabel?: string;
}) {
  return (
    <SelectPrimitive.Root value={value} onValueChange={onValueChange}>
      <SelectPrimitive.Trigger
        aria-label={ariaLabel}
        className={cn(
          "inline-flex h-8 min-w-40 items-center justify-between gap-2 rounded-md border border-lab-line bg-lab-panel2 px-2 text-sm text-lab-text",
          "focus-visible:outline-none focus-visible:shadow-focus",
          className,
        )}
      >
        <SelectPrimitive.Value />
        <SelectPrimitive.Icon>
          <ChevronDown className="h-4 w-4 text-lab-muted" />
        </SelectPrimitive.Icon>
      </SelectPrimitive.Trigger>
      <SelectPrimitive.Portal>
        <SelectPrimitive.Content className="z-50 overflow-hidden rounded-md border border-lab-line bg-lab-panel2 text-lab-text shadow-xl">
          <SelectPrimitive.Viewport className="p-1">
            {items.map((item) => (
              <SelectPrimitive.Item
                key={item.value}
                value={item.value}
                className="relative flex h-7 cursor-default select-none items-center rounded px-7 text-sm outline-none data-[highlighted]:bg-lab-accent/[0.18]"
              >
                <SelectPrimitive.ItemIndicator className="absolute left-2 inline-flex items-center">
                  <Check className="h-3.5 w-3.5 text-lab-accent" />
                </SelectPrimitive.ItemIndicator>
                <SelectPrimitive.ItemText>{item.label}</SelectPrimitive.ItemText>
              </SelectPrimitive.Item>
            ))}
          </SelectPrimitive.Viewport>
        </SelectPrimitive.Content>
      </SelectPrimitive.Portal>
    </SelectPrimitive.Root>
  );
}
