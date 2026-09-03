import type { PageType } from "../types";

/** One 16px grid, one stroke weight, currentColor throughout. */
function Svg({
  size = 16,
  className,
  children,
}: {
  size?: number;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.4}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

type IconProps = { size?: number; className?: string };

export function SearchIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <circle cx="7" cy="7" r="4.25" />
      <path d="m10.25 10.25 3.25 3.25" />
    </Svg>
  );
}

export function RefreshIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M13.2 7A5.2 5.2 0 1 0 12 11.3" />
      <path d="M13.5 3.2V7h-3.7" />
    </Svg>
  );
}

export function CopyIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <rect x="5.5" y="5.5" width="8" height="8" rx="1.6" />
      <path d="M10.5 3.2A1.7 1.7 0 0 0 9 2.5H4.2a1.7 1.7 0 0 0-1.7 1.7V9c0 .66.37 1.2.9 1.45" />
    </Svg>
  );
}

export function TrashIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M2.8 4.3h10.4" />
      <path d="M6.4 4.3V3.1a.9.9 0 0 1 .9-.9h1.4a.9.9 0 0 1 .9.9v1.2" />
      <path d="M4.2 4.3 4.8 13a.9.9 0 0 0 .9.8h4.6a.9.9 0 0 0 .9-.8l.6-8.7" />
    </Svg>
  );
}

export function FileIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M12.6 8.4v4.1a1.3 1.3 0 0 1-1.3 1.3H3.5a1.3 1.3 0 0 1-1.3-1.3V4.7a1.3 1.3 0 0 1 1.3-1.3h4.1" />
      <path d="M10.2 2.4h3.4v3.4" />
      <path d="m13.6 2.4-5.4 5.4" />
    </Svg>
  );
}

export function ChevronIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="m6 3.5 4.5 4.5L6 12.5" />
    </Svg>
  );
}

export function InstallIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M8 2.4v7.2" />
      <path d="M5.2 6.8 8 9.6l2.8-2.8" />
      <path d="M2.8 11.4v1.2a1 1 0 0 0 1 1h8.4a1 1 0 0 0 1-1v-1.2" />
    </Svg>
  );
}

export function SettingsIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <circle cx="8" cy="8" r="2.1" />
      <path d="M12.9 9.9a1.1 1.1 0 0 0 .22 1.21l.04.04a1.33 1.33 0 1 1-1.88 1.88l-.04-.04a1.1 1.1 0 0 0-1.21-.22 1.1 1.1 0 0 0-.67 1v.11a1.33 1.33 0 0 1-2.66 0v-.06a1.1 1.1 0 0 0-.72-1 1.1 1.1 0 0 0-1.21.22l-.04.04A1.33 1.33 0 1 1 2.85 11.2l.04-.04a1.1 1.1 0 0 0 .22-1.21 1.1 1.1 0 0 0-1-.67h-.11a1.33 1.33 0 1 1 0-2.66h.06a1.1 1.1 0 0 0 1-.72 1.1 1.1 0 0 0-.22-1.21l-.04-.04A1.33 1.33 0 1 1 4.68 2.77l.04.04a1.1 1.1 0 0 0 1.21.22h.05a1.1 1.1 0 0 0 .67-1v-.11a1.33 1.33 0 0 1 2.66 0v.06a1.1 1.1 0 0 0 .67 1 1.1 1.1 0 0 0 1.21-.22l.04-.04a1.33 1.33 0 1 1 1.88 1.88l-.04.04a1.1 1.1 0 0 0-.22 1.21v.05a1.1 1.1 0 0 0 1 .67h.11a1.33 1.33 0 0 1 0 2.66h-.06a1.1 1.1 0 0 0-1 .67Z" />
    </Svg>
  );
}

export function ReturnIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M13.5 3.5V7a2 2 0 0 1-2 2H3.2" />
      <path d="M6 6.2 3.2 9 6 11.8" />
    </Svg>
  );
}

/**
 * The four modifier keys.
 *
 * Drawn rather than typed. `⌘ ⇧ ⌥ ⌃` are characters, so they arrive at whatever
 * weight the mono face happens to have, sit off the cap's optical centre, and
 * ignore the icon size scale. As paths they match every other mark in this file.
 */
export function CommandIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M10 4v8a2 2 0 1 0 2-2H4a2 2 0 1 0 2 2V4a2 2 0 1 0-2 2h8a2 2 0 1 0-2-2" />
    </Svg>
  );
}

export function ShiftIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M8 2.4 2.9 8h2.9v5.2h4.4V8h2.9z" />
    </Svg>
  );
}

export function ControlIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="m3.4 10.2 4.6-4.4 4.6 4.4" />
    </Svg>
  );
}

export function OptionIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M2.6 4.4h3.6l4.6 7.2h2.6" />
      <path d="M9.6 4.4h3.8" />
    </Svg>
  );
}

export function ArrowUpIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M8 13V4" />
      <path d="m4.2 7.8 3.8-3.8 3.8 3.8" />
    </Svg>
  );
}

export function ArrowDownIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M8 3v9" />
      <path d="m11.8 8.2-3.8 3.8-3.8-3.8" />
    </Svg>
  );
}

export function CheckIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="m3.4 8.4 3 3 6.2-6.8" />
    </Svg>
  );
}

/** A step that has not happened yet: the unfilled counterpart to the check. */
export function CircleIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <circle cx="8" cy="8" r="4.6" />
    </Svg>
  );
}

export function LinkIcon(p: IconProps) {
  return (
    <Svg {...p}>
      <path d="M6.6 9.4a2.6 2.6 0 0 0 3.8 0l2-2a2.7 2.7 0 0 0-3.8-3.8l-.9.9" />
      <path d="M9.4 6.6a2.6 2.6 0 0 0-3.8 0l-2 2a2.7 2.7 0 0 0 3.8 3.8l.9-.9" />
    </Svg>
  );
}

/**
 * A mark per page type, so a sidebar row fits on one line.
 *
 * Geometry rather than pictures: an overview is a target, a decision forks, an
 * open question is an unclosed ring, an attempt is an arrow that came back, a
 * component stacks, a constraint is bracketed on both sides.
 */
export function TypeIcon({ type, ...p }: IconProps & { type: PageType }) {
  switch (type) {
    case "project":
      return (
        <Svg {...p}>
          <circle cx="8" cy="8" r="5" />
          <circle cx="8" cy="8" r="1.6" fill="currentColor" stroke="none" />
        </Svg>
      );
    case "decision":
      return (
        <Svg {...p}>
          <path d="M8 2.6v4.2" />
          <path d="M8 6.8 4.4 10.4" />
          <path d="M8 6.8l3.6 3.6" />
          <circle cx="4.4" cy="12.1" r="1.3" />
          <circle cx="11.6" cy="12.1" r="1.3" />
        </Svg>
      );
    case "open":
      return (
        <Svg {...p}>
          <path d="M8 3a5 5 0 1 1-4.6 3" />
          <circle cx="8" cy="8" r="1.3" fill="currentColor" stroke="none" />
        </Svg>
      );
    case "attempt":
      return (
        <Svg {...p}>
          <path d="M12.6 9A4.8 4.8 0 1 0 8 12.8" />
          <path d="M9.6 2.6 12.8 5l-3.2 2.4" />
        </Svg>
      );
    case "component":
      return (
        <Svg {...p}>
          <rect x="3" y="2.8" width="10" height="4.2" rx="1.2" />
          <rect x="3" y="9" width="10" height="4.2" rx="1.2" />
        </Svg>
      );
    case "gotcha":
      return (
        <Svg {...p}>
          <path d="M6 2.8H3.6v10.4H6" />
          <path d="M10 2.8h2.4v10.4H10" />
        </Svg>
      );
  }
}

/** What the sidebar shows on hover, and what the detail header spells out. */
export const TYPE_LABEL: Record<PageType, string> = {
  project: "Overview",
  decision: "Decision",
  open: "Open question",
  attempt: "Attempt",
  component: "Component",
  gotcha: "Constraint",
};
