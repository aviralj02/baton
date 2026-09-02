/**
 * A label for an icon-only control.
 *
 * Not the native `title` attribute: that waits about a second, cannot be styled,
 * and on an icon row it is the difference between a discoverable control and a
 * guess. Say what the control does, not what it is called.
 *
 * Only safe in a container that does not scroll. An `overflow-y-auto` parent
 * clips both axes and would cut this off.
 */
export function Tooltip({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <span className="group/tip relative inline-flex">
      {children}
      <span
        role="tooltip"
        className="pointer-events-none absolute right-0 top-full z-20 mt-1.5 whitespace-nowrap rounded-md bg-ink px-2 py-1 text-meta font-medium text-surface opacity-0 shadow-md transition-opacity duration-150 group-focus-within/tip:opacity-100 group-hover/tip:opacity-100 group-hover/tip:delay-300"
      >
        {label}
      </span>
    </span>
  );
}
