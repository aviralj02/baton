/** A keycap, so a shortcut reads as something to press rather than something to know. */
export function Key({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="rounded border border-black/10 bg-black/4 px-1.5 py-0.5 font-mono text-[11px] leading-4 dark:border-white/10 dark:bg-white/5">
      {children}
    </kbd>
  );
}
