/**
 * A keycap, so a shortcut reads as something to press rather than something to know.
 *
 * Sized as a box rather than as text, so a cap holding one icon comes out square
 * next to a cap holding a word. An icon is aria-hidden by definition, so a cap
 * with no text in it takes a `label` to say which key it is.
 */
export function Key({ label, children }: { label?: string; children: React.ReactNode }) {
  return (
    <kbd
      aria-label={label}
      className="inline-flex h-5 min-w-5 items-center justify-center rounded border border-line bg-panel px-1.5 align-middle font-mono text-meta leading-none text-body"
    >
      {children}
    </kbd>
  );
}
