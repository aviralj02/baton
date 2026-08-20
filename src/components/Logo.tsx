export function Logo({ size = 18, className }: { size?: number; className?: string }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      className={className}
      aria-hidden="true"
    >
      <path
        d="M9.75 14.25 18 6"
        stroke="currentColor"
        strokeWidth={3.5}
        strokeLinecap="round"
      />
      <path
        d="M4.5 15.75 7.75 12.5"
        stroke="currentColor"
        strokeWidth={2.25}
        strokeLinecap="round"
        opacity={0.4}
      />
    </svg>
  );
}
