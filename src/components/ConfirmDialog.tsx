import { useEffect, useRef } from "react";
import { Button } from "./Button";

/**
 * The confirm step for anything consequential.
 *
 * One dialog, rendered once at the page level and pointed at whatever is being
 * confirmed. The sidebar used to swap its own footer for a confirmation, which
 * put a destructive choice in a 256px column next to the control that opened it.
 *
 * It acts, then closes. `pending` disables both buttons and blocks every route
 * out, and the caller closes it only once the work succeeded, so a dialog that
 * has visually gone away is never still running. On failure it stays open.
 */
export function ConfirmDialog({
  open,
  title,
  body,
  confirmLabel,
  variant = "danger",
  pending,
  onConfirm,
  onCancel,
}: {
  open: boolean;
  title: string;
  body: string;
  confirmLabel: string;
  variant?: "primary" | "danger";
  pending: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  const panel = useRef<HTMLDivElement>(null);

  // Escape and Tab, while the dialog owns the keyboard. Tab cycles inside the
  // panel: focus that walks out onto the page behind a scrim is focus lost.
  useEffect(() => {
    if (!open) return;

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !pending) {
        e.preventDefault();
        onCancel();
        return;
      }
      if (e.key !== "Tab" || !panel.current) return;

      const stops = panel.current.querySelectorAll<HTMLElement>("button:not([disabled])");
      if (stops.length === 0) return;
      const first = stops[0];
      const last = stops[stops.length - 1];
      const active = document.activeElement;

      if (e.shiftKey && (active === first || !panel.current.contains(active))) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && active === last) {
        e.preventDefault();
        first.focus();
      }
    };

    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [open, pending, onCancel]);

  // Cancel takes focus, not the destructive button: Return should not delete.
  useEffect(() => {
    if (open) panel.current?.querySelector("button")?.focus();
  }, [open]);

  if (!open) return null;

  return (
    <div
      onMouseDown={(e) => {
        if (e.target === e.currentTarget && !pending) onCancel();
      }}
      className="animate-scrim fixed inset-0 z-50 flex items-center justify-center bg-scrim p-8"
    >
      <div
        ref={panel}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="confirm-title"
        aria-describedby="confirm-body"
        className="animate-panel max-h-[calc(100svh-4rem)] w-full max-w-sm overflow-y-auto rounded-xl border border-line bg-surface shadow-2xl"
      >
        <div className="px-5 pb-5 pt-5">
          <h2
            id="confirm-title"
            className="font-serif text-title tracking-tight text-ink"
          >
            {title}
          </h2>
          <p id="confirm-body" className="mt-2 text-ui leading-relaxed text-body">
            {body}
          </p>
        </div>

        <div className="flex items-center justify-end gap-2 border-t border-line px-5 py-3">
          <Button disabled={pending} onClick={onCancel}>
            Cancel
          </Button>
          <Button variant={variant} disabled={pending} onClick={onConfirm}>
            {pending ? "Working…" : confirmLabel}
          </Button>
        </div>
      </div>
    </div>
  );
}
