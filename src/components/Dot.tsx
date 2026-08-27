/**
 * The separator between items in a metadata row. A rendered dot rather than a
 * middot character, which drifts off the baseline and carries too much weight.
 * Belongs inside a `flex items-center gap-1.5` parent.
 */
export function Dot() {
  return (
    <span
      aria-hidden="true"
      className="inline-block size-1 shrink-0 rounded-full bg-stone-300 dark:bg-stone-600"
    />
  );
}
