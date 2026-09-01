import { useReducedMotion, type Transition, type Variants } from "motion/react";

/**
 * One motion vocabulary for the page, wrapping `motion/react`.
 *
 * Everything here is short, small and in one direction: content arrives from
 * slightly below, once. Scattered effects read as a template with the animation
 * knob turned up; a single orchestrated arrival reads as intent.
 */
export const EASE: Transition["ease"] = [0.16, 1, 0.3, 1];

/** Parent of a staggered group. The children carry the actual movement. */
export const group = (stagger = 0.06, delay = 0): Variants => ({
  hidden: {},
  shown: { transition: { staggerChildren: stagger, delayChildren: delay } },
});

export const rise: Variants = {
  hidden: { opacity: 0, y: 8 },
  shown: { opacity: 1, y: 0, transition: { duration: 0.45, ease: EASE } },
};

/**
 * The same variants with the movement removed.
 *
 * Returning empty objects rather than skipping the props keeps every component
 * written one way, and means a reduced-motion visitor still sees the content in
 * its final state rather than stuck at `hidden`.
 */
export function useMotionSafe() {
  const reduced = useReducedMotion();
  return {
    reduced: Boolean(reduced),
    rise: reduced ? ({ hidden: {}, shown: {} } as Variants) : rise,
    group: (stagger?: number, delay?: number) =>
      reduced ? ({ hidden: {}, shown: {} } as Variants) : group(stagger, delay),
  };
}
