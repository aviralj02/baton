import { useEffect, useRef, useState } from "react";
import { motion, useInView } from "motion/react";
import { claimSlot, useMotionSafe, VIEW_MARGIN } from "./anim";

/**
 * One element arriving.
 *
 * Every animated thing on the page is one of these. It carries no timing of its
 * own: when it becomes visible it takes the next free slot from the page queue
 * in `anim.ts`, so the sequence follows the order things are actually reached.
 *
 * It drops the filter once it has settled. A element left holding `blur(0px)`
 * keeps a compositing layer and goes on rasterising its text through a filter
 * pass, which costs a little of the crispness the blur was there to hand back.
 *
 * It deliberately does not use variant inheritance. A parent's `staggerChildren`
 * only reaches children that do not declare their own `initial`, so the moment a
 * section needed its own scroll trigger it silently dropped out of the parent's
 * sequence. Per-element delays fixed that but introduced the opposite fault:
 * every element ran its own clock, so two sections entering together played two
 * cascades at once. One queue has neither problem.
 */
export function Reveal({
  onView = false,
  className,
  children,
}: {
  /** Wait for the scroll into view rather than claiming a slot on load. */
  onView?: boolean;
  className?: string;
  children: React.ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const inView = useInView(ref, { once: true, margin: VIEW_MARGIN });
  const m = useMotionSafe();
  const ready = onView ? inView : true;

  // Null until this element has a slot. The ref guards against a second claim,
  // which is what StrictMode's remount would otherwise take in development.
  const [delay, setDelay] = useState<number | null>(null);
  const [settled, setSettled] = useState(false);
  const claimed = useRef(false);

  useEffect(() => {
    if (!ready || claimed.current) return;
    claimed.current = true;
    setDelay(claimSlot());
  }, [ready]);

  const shown = delay !== null;

  return (
    <motion.div
      ref={ref}
      initial={{ opacity: 0, y: m.offsetY, filter: m.blurred }}
      animate={
        shown
          ? { opacity: 1, y: 0, filter: settled ? "none" : "blur(0px)" }
          : { opacity: 0, y: m.offsetY, filter: m.blurred }
      }
      transition={m.transition(delay ?? 0)}
      // Guarded on `shown`: the element settles at the end of its reveal, not at
      // the end of the no-op animation it runs while it is still waiting.
      onAnimationComplete={() => shown && setSettled(true)}
      className={className}
    >
      {children}
    </motion.div>
  );
}
