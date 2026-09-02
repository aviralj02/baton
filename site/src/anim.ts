import { useReducedMotion, type Transition } from "motion/react";

/* ─────────────────────────────────────────────────────────
 * ANIMATION STORYBOARD
 *
 * One element at a time, 70ms apart, in the order each one
 * reaches the viewport. Nothing on this page ever animates
 * alongside anything else.
 *
 * Each arrival is the same three-part move: it fades up 8px
 * and resolves out of a 6px blur, so a line reads as coming
 * into focus rather than switching on.
 *
 *   on load    mark and repo link
 *      +70ms   headline
 *     +140ms   lede
 *     +210ms   download buttons
 *
 *  on scroll   section heading
 *      +70ms   the hairline under it
 *     +140ms   body copy
 *     +210ms   each further row, one at a time
 *
 * The order is not written per element. Every element takes
 * the next free slot from one page-wide queue when it becomes
 * visible, which is what makes the sequence hold: two sections
 * crossing the threshold in the same frame still arrive one
 * after the other, and a section reached on its own starts
 * immediately instead of waiting out a clock that already ran.
 * ───────────────────────────────────────────────────────── */

export const TIMING = {
  /** ms between one element arriving and the next. */
  step: 70,
};

/**
 * The one movement on the page.
 *
 * A short rise and a fade, on a spring rather than a duration, so a fast
 * scroller catching an element mid-flight sees it settle rather than snap.
 */
export const RISE = {
  offsetY: 8, // px each element rises from
  blur: 6, // px of blur each element resolves out of
  spring: { type: "spring", stiffness: 350, damping: 30 } as Transition,
  /* Blur gets a duration, not the spring. A spring overshoots, and blur
     clamps at zero, so the overshoot lands as a visible stall at the end. */
  focus: { duration: 0.45, ease: [0.16, 1, 0.3, 1] } as Transition,
};

/** Elements claim their slot this far before the viewport edge. */
export const VIEW_MARGIN = "-80px";

export const EASE: Transition["ease"] = [0.16, 1, 0.3, 1];

/**
 * The page-wide queue.
 *
 * `freeAt` is the first moment nothing is arriving. An element claiming a slot
 * takes that moment or now, whichever is later, and pushes the marker one step
 * out. Module state rather than context because there is one page and one
 * sequence, and a claim has to be readable from an effect without a re-render.
 */
let freeAt = 0;

export function claimSlot(): number {
  const now = performance.now();
  const start = Math.max(now, freeAt);
  freeAt = start + TIMING.step;
  return start - now;
}

/**
 * Reduced motion keeps the order and drops the movement.
 *
 * Returning a zeroed offset and an instant transition rather than skipping the
 * animation entirely means every component stays written one way, and a visitor
 * who asked for less motion still lands on finished content.
 */
export function useMotionSafe() {
  const reduced = Boolean(useReducedMotion());
  return {
    reduced,
    offsetY: reduced ? 0 : RISE.offsetY,
    blurred: reduced ? "none" : `blur(${RISE.blur}px)`,
    transition: (delayMs: number): Transition => {
      if (reduced) return { duration: 0 };
      const delay = delayMs / 1000;
      return { ...RISE.spring, delay, filter: { ...RISE.focus, delay } };
    },
  };
}
