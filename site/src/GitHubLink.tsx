import { motion } from "motion/react";
import { EASE } from "./anim";

export const REPO_URL = "https://github.com/aviralj02/baton";

/**
 * The repo link.
 *
 * The word slides out of the icon on hover rather than sitting beside it: at
 * rest this is one quiet glyph, and the label appears only for someone who has
 * already decided to look at it.
 */
export function GitHubLink() {
  return (
    <motion.a
      href={REPO_URL}
      aria-label="Baton on GitHub"
      initial="rest"
      whileHover="hover"
      whileFocus="hover"
      whileTap={{ scale: 0.96 }}
      className="group inline-flex items-center rounded-full border border-line p-2 text-muted transition-colors duration-200 hover:border-brand/40 hover:text-brand"
    >
      <motion.span
        variants={{ rest: { rotate: 0 }, hover: { rotate: -12 } }}
        transition={{ duration: 0.4, ease: EASE }}
        className="grid place-items-center"
      >
        <GitHubMark />
      </motion.span>

      <motion.span
        // The gap animates with the label, or the pill sits lopsided at rest.
        variants={{
          rest: { width: 0, opacity: 0, marginLeft: 0 },
          hover: { width: "auto", opacity: 1, marginLeft: 6 },
        }}
        transition={{ duration: 0.32, ease: EASE }}
        className="overflow-hidden whitespace-nowrap text-meta font-medium"
      >
        GitHub
      </motion.span>
    </motion.a>
  );
}

function GitHubMark() {
  return (
    <svg
      width="16"
      height="16"
      viewBox="0 0 16 16"
      fill="currentColor"
      aria-hidden="true"
    >
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82a7.4 7.4 0 0 1 2-.27c.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z" />
    </svg>
  );
}
