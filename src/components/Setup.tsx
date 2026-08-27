import { useEffect, useState } from "react";
import * as api from "../lib/api";
import { Logo } from "./Logo";
import type { WikiStatus } from "../types";

/**
 * First-run setup.
 *
 * Baton indexes a wiki it does not write. Without the `/baton` skill installed
 * in an agent tool, a new user reaches an empty launcher telling them to run a
 * command that does not exist — so the one step the app cannot do for itself is
 * the one thing this screen exists to do.
 *
 * The wiki folder is already created by the time this renders (it is Baton's
 * own data). Installing the skill writes into another tool's config, so it
 * stays an explicit action.
 */
export function Setup({
  status,
  onDone,
}: {
  status: WikiStatus;
  onDone: () => void;
}) {
  const [busy, setBusy] = useState(false);
  const [installed, setInstalled] = useState<string[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const detected = status.skills.filter((s) => s.detected);
  const missing = detected.filter((s) => !s.installed || s.outdated);

  const install = async () => {
    setBusy(true);
    setError(null);
    try {
      setInstalled(await api.installSkills());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex h-full flex-col overflow-y-auto px-6 py-5">
      <div className="mb-4 flex items-center gap-2.5">
        <Logo size={20} />
        <h1 className="text-[15px] font-medium dark:text-stone-100">
          Set up Baton
        </h1>
      </div>

      <p className="mb-5 text-sm leading-relaxed text-stone-600 dark:text-stone-300">
        Baton reads a wiki that your coding agent writes. It never calls a model
        itself — so the agent needs a <code className="rounded bg-black/5 px-1 py-0.5 text-[12px] dark:bg-white/10">/baton</code>{" "}
        command to file what it learned.
      </p>

      <Step n={1} done title="Wiki folder created">
        <button
          onClick={() => void api.revealWiki()}
          className="font-mono text-[12px] text-stone-500 underline-offset-2 hover:underline dark:text-stone-400"
        >
          {status.root}
        </button>
      </Step>

      <Step
        n={2}
        done={detected.length > 0 && missing.length === 0}
        title="Install the /baton command"
      >
        {detected.length === 0 ? (
          <p className="text-[13px] text-stone-500 dark:text-stone-400">
            No agent tools found. Install Claude Code, Codex, or Cursor, then
            reopen this screen.
          </p>
        ) : (
          <>
            <ul className="mb-2 space-y-1">
              {detected.map((s) => (
                <li
                  key={s.name}
                  className="flex items-center gap-2 text-[13px] text-stone-600 dark:text-stone-300"
                >
                  <span
                    className={
                      s.installed && !s.outdated
                        ? "text-brand"
                        : "text-stone-300 dark:text-stone-600"
                    }
                  >
                    ●
                  </span>
                  {s.name}
                  {s.outdated && (
                    <span className="text-[11px] text-stone-400 dark:text-stone-500">needs updating</span>
                  )}
                </li>
              ))}
            </ul>
            {missing.length > 0 && (
              <button
                onClick={() => void install()}
                disabled={busy}
                className="cursor-pointer rounded-md bg-brand px-2.5 py-1 text-xs font-medium text-white transition-all duration-150 hover:bg-brand-strong active:scale-[0.98] disabled:opacity-50 dark:text-stone-900"
              >
                {busy ? "Installing…" : `Install for ${missing.length} tool${missing.length > 1 ? "s" : ""}`}
              </button>
            )}
            {installed && (
              <p className="mt-2 text-[12px] text-brand">
                Installed for {installed.join(", ")}.
              </p>
            )}
            {error && (
              <p className="mt-2 text-[12px] text-red-600 dark:text-red-400">{error}</p>
            )}
          </>
        )}
      </Step>

      <Step n={3} done={status.pageCount > 0} title="File your first session">
        <p className="text-[13px] leading-relaxed text-stone-500 dark:text-stone-400">
          Finish a piece of work with your agent, then run{" "}
          <code className="rounded bg-black/5 px-1 py-0.5 text-[12px] dark:bg-white/10">
            /baton
          </code>
          . It files what the session learned without asking. After that,
          press the hotkey anywhere to copy a project's context into a fresh
          session.
        </p>
      </Step>

      <button
        onClick={onDone}
        className="mt-5 self-start text-[13px] text-stone-500 underline-offset-2 hover:underline dark:text-stone-400"
      >
        {status.pageCount > 0 ? "Done" : "Skip for now"}
      </button>
    </div>
  );
}

function Step({
  n,
  done,
  title,
  children,
}: {
  n: number;
  done?: boolean;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="mb-5 flex gap-3">
      <span
        className={`mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-[11px] font-medium ${
          done
            ? "bg-brand text-white dark:text-stone-900"
            : "bg-black/10 text-stone-500 dark:bg-white/15 dark:text-stone-300"
        }`}
      >
        {done ? "✓" : n}
      </span>
      <div className="min-w-0 flex-1">
        <h2 className="mb-1 text-[13px] font-medium dark:text-stone-100">{title}</h2>
        {children}
      </div>
    </section>
  );
}

/** Shows setup until the wiki has pages, unless the user dismissed it. */
export function useSetupGate() {
  const [status, setStatus] = useState<WikiStatus | null>(null);
  const [dismissed, setDismissed] = useState(false);

  const refresh = () => {
    api.wikiStatus().then(setStatus).catch(() => setStatus(null));
  };

  useEffect(refresh, []);

  // Setup is needed while there is nothing to read and no way to write.
  //
  // The no-tools-detected case counts: a fresh machine with no agent installed
  // is exactly the user who needs to be told what `/baton` is. An earlier
  // version required a detected tool, which meant the people furthest from a
  // working setup were the only ones who never saw the screen.
  const detected = status?.skills.filter((s) => s.detected) ?? [];
  const needed =
    !dismissed &&
    status !== null &&
    status.pageCount === 0 &&
    (detected.length === 0 || detected.some((s) => !s.installed || s.outdated));

  return { status, needed, dismiss: () => setDismissed(true), refresh };
}
