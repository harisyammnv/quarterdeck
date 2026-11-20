import type { ButtonHTMLAttributes } from "react";
import type { Icon } from "@phosphor-icons/react";
import { Keycap } from "./Keycap";
import { Tooltip } from "./Tooltip";

type Variant = "primary" | "secondary" | "ghost";

interface ButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
  /** Solid black means an action, and only an action: `primary` is the one main action of a card, panel or field. */
  variant?: Variant;
  size?: "md" | "sm";
  icon?: Icon;
  /** A shortcut drawn as a keycap inside the button, such as "⌘↵". */
  shortcut?: string;
  children?: React.ReactNode;
}

/** Button (design system): `qd-btn`, with `--primary`, `--ghost` and `--sm`. */
export function Button({ variant = "secondary", size = "md", icon: I, shortcut, children, className, type = "button", ...rest }: ButtonProps) {
  const cls = ["qd-btn", variant === "secondary" ? "" : `qd-btn--${variant}`, size === "sm" ? "qd-btn--sm" : "", className ?? ""].filter(Boolean).join(" ");
  return (
    <button type={type} className={cls} {...rest}>
      {I && <I size={14} weight={variant === "primary" ? "bold" : "regular"} />}
      {children}
      {shortcut && <Keycap>{shortcut}</Keycap>}
    </button>
  );
}

/**
 * IconButton (design system): a 28px round icon-only button, `qd-ibtn`, with its tooltip.
 * `pressed` makes it a toggle (`aria-pressed`), drawn as the solid `toggle-on` disc.
 */
export function IconButton({
  icon: I,
  label,
  shortcut,
  pressed,
  onClick,
  disabled,
}: {
  icon: Icon;
  label: string;
  shortcut?: string;
  pressed?: boolean;
  onClick?: () => void;
  disabled?: boolean;
}) {
  return (
    <Tooltip label={label} shortcut={shortcut}>
      <button type="button" className="qd-ibtn" aria-label={shortcut ? `${label}, ${shortcut}` : label} aria-pressed={pressed} onClick={onClick} disabled={disabled}>
        <I size={16} />
      </button>
    </Tooltip>
  );
}
