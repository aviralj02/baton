import { useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { Mark } from "./Mark";
import { GitHubLink, REPO_URL } from "./GitHubLink";
import { Reveal } from "./Reveal";
import { EASE, useMotionSafe } from "./anim";
import {
  detectPlatform,
  RELEASES_URL,
  useRelease,
  type Download,
  type Release,
  type Status,
} from "./useRelease";

/**
 * The download page.
 *
 * Every entrance is a `Reveal`. They carry no timing: each takes the next slot
 * from one page-wide queue as it is reached, so nothing ever arrives alongside
 * anything else. The storyboard is at the top of `anim.ts`.
 */
export function App() {
  const { status, release } = useRelease();
  const platform = useState(detectPlatform)[0];

  return (
    <div className="min-h-screen bg-surface text-ink antialiased">
      <main className="mx-auto max-w-2xl px-6 py-16 sm:py-24">
        <Reveal className="flex items-center justify-between">
          <Mark className="text-brand" size={36} />
          <GitHubLink />
        </Reveal>

        <Reveal>
          <h1 className="mt-12 font-serif text-hero tracking-tight sm:text-hero-lg">
            Your context, independent of the AI you&rsquo;re using.
          </h1>
        </Reveal>

        <Reveal>
          <p className="mt-5 text-lede text-body">
            You spend an hour with an AI getting somewhere real. Then the chat ends, and
            the next one starts from nothing. Baton keeps what the session learned, and
            hands it to whatever you open next.
          </p>
        </Reveal>

        <Reveal>
          <Downloads platform={platform} status={status} release={release} />
        </Reveal>

        <Loop />
        <Automatic />
        <Privacy />

        <Footer version={release?.version} />
      </main>
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
      <div className="flex flex-wrap items-center gap-5">
        {platform === "windows" ? [windows, mac] : [mac, windows]}
      </div>

      {status === "unavailable" && (
        <p className="mt-4 text-ui text-muted">
          Couldn&rsquo;t reach GitHub for the latest build, so both buttons go to the{" "}
          <a className="text-brand hover:underline" href={RELEASES_URL}>
            releases page
          </a>
          .
        </p>
      )}
    </section>
  );
}

/**
 * How far a button gives under a press. The same figure the app uses for
 * `active:scale-[0.98]`, so a control feels identical in both places.
 *
 * Press is the only movement a button makes. Hover is a colour change: lifting
 * a button towards the cursor animates the thing the cursor is already over,
 * which reads as the page flinching rather than as the button responding.
 */
const PRESS = 0.98;

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
        whileTap={reduced ? undefined : { scale: PRESS }}
        className="text-read text-muted underline-offset-4 transition-colors duration-200 hover:text-brand hover:underline"
      >
        {label}
      </motion.a>
    );
  }

  return (
    <motion.a
      href={href}
      whileTap={reduced ? undefined : { scale: PRESS }}
      transition={{ duration: 0.2, ease: EASE }}
      className="inline-flex h-11 items-center gap-2 rounded-full bg-brand px-6 text-read font-medium text-on-brand shadow-sm transition-colors duration-200 hover:bg-brand-strong"
    >
      {label}
      <AnimatePresence>
        {note && (
          <motion.span
            initial={{ opacity: 0, width: 0 }}
            animate={{ opacity: 0.7, width: "auto" }}
            transition={{ duration: 0.3, ease: EASE }}
            className="tnum overflow-hidden whitespace-nowrap font-mono text-meta"
          >
            {note}
          </motion.span>
        )}
      </AnimatePresence>
    </motion.a>
  );
}

/** The loop, as three steps rather than a diagram nobody reads. */
const STEPS = [
  {
    title: "Your agent writes it",
    body: (
      <>
        Finish a session, run <Code>/baton</Code>. The agent that did the work files what
        it learned: decisions with the alternatives that lost, approaches that failed,
        constraints that bit you.
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
        Press the hotkey anywhere, pick a project, hit enter. Its whole context is on your
        clipboard. Paste into Claude, ChatGPT, Cursor, Codex, anything.
      </>
    ),
  },
];

function Loop() {
  return (
    <Section heading="How it works">
      <ol className="flex flex-col gap-6">
        {/* The Reveal sits inside the li: an ol may only contain list items. */}
        {STEPS.map((step, i) => (
          <li key={step.title}>
            <Reveal onView className="flex gap-4">
              <span className="tnum mt-1 font-mono text-meta text-brand">
                {String(i + 1).padStart(2, "0")}
              </span>
              <div className="min-w-0">
                <h3 className="text-read font-medium text-ink">{step.title}</h3>
                <p className="mt-1 text-read text-body">{step.body}</p>
              </div>
            </Reveal>
          </li>
        ))}
      </ol>
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
  return (
    <Section heading="Make it automatic">
      <Reveal onView>
        <p className="text-read text-body">
          The one habit Baton depends on is running <Code>/baton</Code>. Put a line in the
          instructions file your agent already reads, whether that is{" "}
          <Code>CLAUDE.md</Code>, <Code>AGENTS.md</Code> or <Code>.cursor/rules</Code>,
          and it happens without you remembering.
        </p>
      </Reveal>

      <Reveal onView>
        <pre className="mt-5 overflow-x-auto rounded-xl border border-line bg-panel p-4 font-mono text-ui leading-relaxed text-body">
          <code>
            Run /baton every two or three exchanges, and again{"\n"}
            after finishing anything worth remembering.
          </code>
        </pre>
      </Reveal>

      <Reveal onView>
        <p className="mt-4 text-ui text-muted">
          Every session then leaves the next one better informed, whichever tool you open
          it in.
        </p>
      </Reveal>
    </Section>
  );
}

function Privacy() {
  return (
    <Section heading="What it doesn't do">
      <Reveal onView>
        <p className="text-read text-body">
          No account. No cloud. No telemetry. Baton makes no model calls and has no API
          key of its own, because your agent does the writing. Nothing leaves your
          machine, and everything it keeps is a folder of markdown you can read, edit and
          delete.
        </p>
      </Reveal>
    </Section>
  );
}

function Footer({ version }: { version?: string }) {
  return (
    <Reveal onView>
      <footer className="mt-24 flex flex-wrap items-center gap-x-5 gap-y-2 border-t border-line pt-6 text-ui text-muted">
        <a
          className="transition-colors duration-200 hover:text-brand"
          href={RELEASES_URL}
        >
          Releases
        </a>
        <a
          className="transition-colors duration-200 hover:text-brand"
          href={`${REPO_URL}/blob/main/LICENSE`}
        >
          MIT
        </a>
        {version && <span className="tnum ml-auto font-mono text-meta">v{version}</span>}
      </footer>
    </Reveal>
  );
}

/**
 * A section heading, in the app's own vocabulary: serif over a hairline.
 *
 * The heading and its rule are part of the sequence rather than standing there
 * finished while the body fades in beneath them. That was the single biggest
 * reason the old page did not read as sequential.
 */
function Section({ heading, children }: { heading: string; children: React.ReactNode }) {
  return (
    <section className="mt-16">
      <Reveal onView>
        <h2 className="font-serif text-title tracking-tight text-ink">{heading}</h2>
      </Reveal>
      <Reveal onView>
        <div className="mt-2 mb-6 h-px bg-line" />
      </Reveal>
      {children}
    </section>
  );
}

function Code({ children }: { children: React.ReactNode }) {
  return (
    <code className="rounded bg-panel px-1.5 py-0.5 font-mono text-ui text-brand">
      {children}
    </code>
  );
}
