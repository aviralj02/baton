import { Tooltip } from "./Tooltip";

/**
 * Every text button in the app.
 *
 * Two heights and three intents, chosen with props rather than a class string
 * per call site: the moment a height is spelled out inline, the next button
 * gets a different one and the row stops lining up.
 */
type Variant = "primary" | "danger" | "quiet" | "armed";
type Size = "md" | "sm";

const VARIANT: Record<Variant, string> = {
  primary: "bg-brand text-on-brand shadow-sm hover:bg-brand-strong",
  danger: "bg-danger text-on-danger shadow-sm hover:bg-danger-strong",
  quiet: "text-muted hover:bg-hover hover:text-ink",
  /* A control that is live and listening, rather than one waiting to be pressed. */
  armed: "bg-brand-soft text-brand",
};

/** `md` everywhere; `sm` only for the inline confirmations inside a list. */
const SIZE: Record<Size, string> = {
  md: "h-8 gap-1.5 px-3.5 text-ui",
  sm: "h-7 gap-1 px-3 text-meta",
};

const BASE =
  "inline-flex shrink-0 cursor-pointer items-center justify-center rounded-full font-medium transition-all duration-150 active:scale-[0.98] disabled:cursor-not-allowed disabled:opacity-50";

export function Button({
  variant = "quiet",
  size = "md",
  disabled,
  /** Set on a toggle, so the control announces its state rather than just its label. */
  pressed,
  onClick,
  children,
}: {
  variant?: Variant;
  size?: Size;
  disabled?: boolean;
  pressed?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      aria-pressed={pressed}
      className={`${BASE} ${SIZE[size]} ${VARIANT[variant]}`}
    >
      {children}
    </button>
  );
}

/**
 * An icon-only control. The label is the tooltip and the accessible name at
 * once, so a control cannot ship with one and not the other.
 */
export function IconButton({
  label,
  danger,
  /** The control opens a confirm dialog rather than acting on the click. */
  haspopup,
  onClick,
  children,
}: {
  label: string;
  danger?: boolean;
  haspopup?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <Tooltip label={label}>
      <button
        onClick={onClick}
        aria-label={label}
        aria-haspopup={haspopup ? "dialog" : undefined}
        className={`cursor-pointer rounded-md p-1.5 text-muted transition-all duration-150 active:scale-95 ${
          danger
            ? "hover:bg-danger-soft hover:text-danger"
            : "hover:bg-hover hover:text-ink"
        }`}
      >
        {children}
      </button>
    </Tooltip>
  );
}
