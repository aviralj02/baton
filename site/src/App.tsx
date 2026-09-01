import { useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { Mark } from "./Mark";
import { GitHubLink, REPO_URL } from "./GitHubLink";
import { EASE, useMotionSafe } from "./anim";
import {
  detectPlatform,
  RELEASES_URL,
  useRelease,
  type Download,
  type Release,
  type Status,
} from "./useRelease";

export function App() {
  const { status, release } = useRelease();
  const platform = useState(detectPlatform)[0];
  const m = useMotionSafe();

  return (
    <div className="min-h-screen bg-stone-50 text-stone-900 antialiased dark:bg-stone-950 dark:text-stone-100">
      <motion.main
        variants={m.group(0.07)}
        initial="hidden"
        animate="shown"
        className="mx-auto max-w-2xl px-6 py-16 sm:py-24"
      >
        <motion.div variants={m.rise} className="flex items-center justify-between">
          <Mark className="text-brand" size={36} />
          <GitHubLink />
        </motion.div>

        <motion.h1
          variants={m.rise}
          className="mt-10 text-3xl font-medium tracking-tight sm:text-4xl"
        >
          Your context, independent of the AI you&rsquo;re using.
        </motion.h1>

        <motion.p
          variants={m.rise}
          className="mt-4 text-lg leading-relaxed text-stone-600 dark:text-stone-300"
        >
          You spend an hour with an AI getting somewhere real. Then the chat ends, and
          the next one starts from nothing. Baton keeps what the session learned, and
          hands it to whatever you open next.
        </motion.p>

        <motion.div variants={m.rise}>
          <Downloads platform={platform} status={status} release={release} />
        </motion.div>

        <Loop />
        <Automatic />
        <Privacy />

        <Footer version={release?.version} />
      </motion.main>
    </div>
  );
}

function Downloads({
  platform,
  status,
  release,
}: {
  platform: ReturnType<typeof detectPlatform>;
  status: Status;
  release: Release | null;
}) {
  const mac = (
    <Button
      key="mac"
      primary={platform !== "windows"}
      href={release?.mac?.url ?? RELEASES_URL}
      label="Download for macOS"
      note={release?.mac?.size}
    />
  );
  const windows = (
    <Button
      key="windows"
      primary={platform === "windows"}
      href={release?.windows?.url ?? RELEASES_URL}
      label="Download for Windows"
      note={release?.windows?.size}
    />
  );

  return (
    <section className="mt-10">
      <div className="flex flex-wrap items-center gap-4">
        {platform === "windows" ? [windows, mac] : [mac, windows]}
      </div>

      {status === "unavailable" && (
        <p className="mt-3 text-sm text-stone-500 dark:text-stone-400">
          Couldn&rsquo;t reach GitHub for the latest build, so both buttons go to the{" "}
          <a className="text-brand hover:underline" href={RELEASES_URL}>
            releases page
          </a>
          .
        </p>
      )}

      <FirstLaunch />
    </section>
  );
}

/**
 * The unsigned warning.
 *
 * Without this the macOS download is a dead end: Gatekeeper refuses an unsigned
 * app outright, and since Sequoia the old Control-click bypass no longer works.
 * Sending someone to a refusal with no instructions is worse than not shipping.
 * Delete this whole component the day notarisation lands.
 */
function FirstLaunch() {
  const [open, setOpen] = useState(false);
  const { reduced } = useMotionSafe();

  return (
    <div className="mt-6 overflow-hidden rounded-xl border border-black/10 bg-white/60 text-sm dark:border-white/10 dark:bg-white/5">
      <button
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        className="flex w-full cursor-pointer items-center gap-2.5 p-4 text-left font-medium"
      >
        <motion.span
          animate={{ rotate: open ? 90 : 0 }}
          transition={reduced ? { duration: 0 } : { duration: 0.3, ease: EASE }}
          className="text-brand"
          aria-hidden="true"
        >
          ›
        </motion.span>
        These builds aren&rsquo;t code-signed yet. Opening one takes an extra step.
      </button>

      <AnimatePresence initial={false}>
        {open && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={reduced ? { duration: 0 } : { duration: 0.32, ease: EASE }}
          >
            <div className="space-y-3 px-4 pb-4 leading-relaxed text-stone-600 dark:text-stone-300">
              <p>
                <b>macOS.</b> Open Baton once and macOS will refuse it. Then go to{" "}
                <b>System Settings &rarr; Privacy &amp; Security</b>, scroll to the
                bottom, and click <b>Open Anyway</b>. That button only appears for about
                an hour after the blocked launch, so do it straight away.
              </p>
              <p>
                <b>Windows.</b> SmartScreen shows a warning. Click <b>More info</b>, then{" "}
                <b>Run anyway</b>.
              </p>
              <p className="text-stone-500 dark:text-stone-400">
                Both go away once the certificates are in place. Until then you can also{" "}
                <a
                  className="text-brand hover:underline"
                  href={`${REPO_URL}/blob/main/docs/DEVELOPMENT.md`}
                >
                  build it from source
                </a>
                .
              </p>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

function Button({
  primary,
  href,
  label,
  note,
}: {
  primary: boolean;
  href: string;
  label: string;
  note?: Download["size"];
}) {
  const { reduced } = useMotionSafe();

  if (!primary) {
    return (
      <motion.a
        href={href}
        whileHover={reduced ? undefined : { y: -1 }}
        className="text-sm text-stone-500 underline-offset-4 transition-colors duration-200 hover:text-brand hover:underline dark:text-stone-400"
      >
        {label}
      </motion.a>
    );
  }

  return (
    <motion.a
      href={href}
      whileHover={reduced ? undefined : { y: -2 }}
      whileTap={reduced ? undefined : { scale: 0.985 }}
      transition={{ duration: 0.2, ease: EASE }}
      className="inline-flex items-center gap-2 rounded-xl bg-brand px-5 py-3 font-medium text-white shadow-sm transition-colors duration-200 hover:bg-brand-strong dark:text-stone-900"
    >
      {label}
      <AnimatePresence>
        {note && (
          <motion.span
            initial={{ opacity: 0, width: 0 }}
            animate={{ opacity: 0.7, width: "auto" }}
            transition={{ duration: 0.3, ease: EASE }}
            className="tnum overflow-hidden whitespace-nowrap font-mono text-xs"
          >
            {note}
          </motion.span>
        )}
      </AnimatePresence>
    </motion.a>
  );
}

/** The loop, as three steps rather than a diagram nobody reads. */
function Loop() {
  const m = useMotionSafe();
  const steps = [
    {
      title: "Your agent writes it",
      body: (
        <>
          Finish a session, run <Code>/baton</Code>. The agent that did the work files
          what it learned: decisions with the alternatives that lost, approaches that
          failed, constraints that bit you.
        </>
      ),
    },
    {
      title: "Baton keeps it",
      body: (
        <>
          As plain markdown you own, on your machine. It works with git, and it works
          offline.
        </>
      ),
    },
    {
      title: "One key gets it back",
      body: (
        <>
          Press the hotkey anywhere, pick a project, hit enter. Its whole context is on
          your clipboard. Paste into Claude, ChatGPT, Cursor, Codex, anything.
        </>
      ),
    },
  ];

  return (
    <Section heading="How it works">
      <motion.ol
        variants={m.group(0.08)}
        initial="hidden"
        whileInView="shown"
        viewport={{ once: true, margin: "-80px" }}
        className="mt-4 space-y-5"
      >
        {steps.map((step, i) => (
          <motion.li key={step.title} variants={m.rise} className="flex gap-4">
            <span className="tnum mt-0.5 font-mono text-xs text-brand">
              {String(i + 1).padStart(2, "0")}
            </span>
            <div className="min-w-0">
              <h3 className="font-medium">{step.title}</h3>
              <p className="mt-1 leading-relaxed text-stone-600 dark:text-stone-300">
                {step.body}
              </p>
            </div>
          </motion.li>
        ))}
      </motion.ol>
    </Section>
  );
}

/**
 * The habit that makes the whole thing work.
 *
 * Running /baton by hand is a tax on finishing, and a tax on finishing gets
 * skipped. Putting the instruction in the file the agent already reads means the
 * user never has to remember, which is the difference between a wiki that fills
 * up and one with three pages in it.
 */
function Automatic() {
  const m = useMotionSafe();

  return (
    <Section heading="Make it automatic">
      <motion.div
        variants={m.group(0.07)}
        initial="hidden"
        whileInView="shown"
        viewport={{ once: true, margin: "-80px" }}
      >
        <motion.p
          variants={m.rise}
          className="mt-3 leading-relaxed text-stone-600 dark:text-stone-300"
        >
          The one habit Baton depends on is running <Code>/baton</Code>. Put a line in
          the instructions file your agent already reads &mdash;{" "}
          <Code>CLAUDE.md</Code>, <Code>AGENTS.md</Code> or{" "}
          <Code>.cursor/rules</Code> &mdash; and it happens without you remembering.
        </motion.p>

        <motion.pre
          variants={m.rise}
          className="mt-4 overflow-x-auto rounded-xl border border-black/10 bg-white/60 p-4 font-mono text-[13px] leading-relaxed text-stone-700 dark:border-white/10 dark:bg-white/5 dark:text-stone-300"
        >
          <code>
            Run /baton every two or three exchanges, and again{"\n"}
            after finishing anything worth remembering.
          </code>
        </motion.pre>

        <motion.p
          variants={m.rise}
          className="mt-3 text-sm leading-relaxed text-stone-500 dark:text-stone-400"
        >
          Every session then leaves the next one better informed, whichever tool you
          open it in.
        </motion.p>
      </motion.div>
    </Section>
  );
}

function Privacy() {
  const m = useMotionSafe();

  return (
    <Section heading="What it doesn't do">
      <motion.p
        variants={m.rise}
        initial="hidden"
        whileInView="shown"
        viewport={{ once: true, margin: "-80px" }}
        className="mt-3 leading-relaxed text-stone-600 dark:text-stone-300"
      >
        No account. No cloud. No telemetry. Baton makes no model calls and has no API
        key of its own &mdash; your agent does the writing. Nothing leaves your machine,
        and everything it keeps is a folder of markdown you can read, edit and delete.
      </motion.p>
    </Section>
  );
}

function Footer({ version }: { version?: string }) {
  const m = useMotionSafe();

  return (
    <motion.footer
      variants={m.rise}
      initial="hidden"
      whileInView="shown"
      viewport={{ once: true }}
      className="mt-20 flex flex-wrap items-center gap-x-5 gap-y-2 border-t border-black/10 pt-6 text-sm text-stone-500 dark:border-white/10 dark:text-stone-400"
    >
      <a className="transition-colors hover:text-brand" href={RELEASES_URL}>
        Releases
      </a>
      <a className="transition-colors hover:text-brand" href={`${REPO_URL}/blob/main/LICENSE`}>
        MIT
      </a>
      {version && <span className="tnum ml-auto font-mono text-xs">v{version}</span>}
    </motion.footer>
  );
}

function Section({ heading, children }: { heading: string; children: React.ReactNode }) {
  return (
    <section className="mt-16">
      <h2 className="font-mono text-[11px] uppercase tracking-[0.14em] text-stone-400 dark:text-stone-500">
        {heading}
      </h2>
      {children}
    </section>
  );
}

function Code({ children }: { children: React.ReactNode }) {
  return (
    <code className="rounded bg-black/5 px-1.5 py-0.5 font-mono text-[13px] text-brand dark:bg-white/10">
      {children}
    </code>
  );
}
