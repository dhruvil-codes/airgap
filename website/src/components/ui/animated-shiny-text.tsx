import { type ComponentPropsWithoutRef, type CSSProperties, type FC } from "react";
import { cx } from "@/lib";

export interface AnimatedShinyTextProps extends ComponentPropsWithoutRef<"span"> {
  shimmerWidth?: number;
}

// Magic UI AnimatedShinyText, restyled with plain CSS (`.shiny` in site.css) instead of Tailwind utilities.
export const AnimatedShinyText: FC<AnimatedShinyTextProps> = ({ children, className, shimmerWidth = 100, style, ...props }) => {
  return (
    <span style={{ "--shiny-width": `${shimmerWidth}px`, ...style } as CSSProperties} className={cx("shiny", className)} {...props}>
      {children}
    </span>
  );
};
