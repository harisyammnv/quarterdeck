import type { ButtonHTMLAttributes } from "react";
import type { Icon } from "@phosphor-icons/react";
import { Kbd } from "./Kbd";

type Variant = "primary" | "secondary" | "ghost";

interface ButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
  /** Solid black means an action, and only an action: use `primary` for the one main action. */
  variant?: Variant;
  size?: "md" | "sm";
  icon?: Icon;
  /** A shortcut drawn as a keycap inside the button, such as "⌘↵". */
  shortcut?: string;
  children?: React.ReactNode;
}

export function Button({ variant = "secondary", size = "md", icon: I, shortcut, children, className, type = "button", ...rest }: ButtonProps) {
  const cls = ["btn", variant === "secondary" ? "" : variant, size === "sm" ? "sm" : "", className ?? ""].filter(Boolean).join(" ");
  return (
    <button type={type} className={cls} {...rest}>
      {I && <I size={14} weight={variant === "primary" ? "bold" : "regular"} />}
      {children}
      {shortcut && <Kbd>{shortcut}</Kbd>}
    </button>
  );
}

/** A round icon-only button, as in the title bar. `label` is required for screen readers. */
export function IconButton({ icon: I, label, pressed, onClick, disabled }: { icon: Icon; label: string; pressed?: boolean; onClick?: () => void; disabled?: boolean }) {
  return (
    <button type="button" className={`ibtn${pressed ? " on" : ""}`} aria-label={label} title={label} aria-pressed={pressed} onClick={onClick} disabled={disabled}>
      <I size={16} />
    </button>
  );
}
