/**
 * The two headings this app has.
 *
 * `Label` is the small caps line over a group of rows: a category, not a
 * title, so it stays quiet and never competes with the thing it labels. `CAPS`
 * is the same recipe as a class string, for the one place that has to hand its
 * heading to a library rather than render it.
 *
 * `SectionHeading` is a heading over content someone actually reads. Serif with
 * a hairline under it, so the eye finds the top of a section without the
 * heading having to shout in weight or size.
 */
export const CAPS = "font-mono text-micro uppercase tracking-[0.14em] text-muted";

export function Label({ children }: { children: React.ReactNode }) {
  return <h2 className={CAPS}>{children}</h2>;
}

export function SectionHeading({ children }: { children: React.ReactNode }) {
  return (
    <div className="mb-3">
      <h2 className="font-serif text-title tracking-tight text-ink">{children}</h2>
      <div className="mt-2 h-px bg-line" />
    </div>
  );
}
