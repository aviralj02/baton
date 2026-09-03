import { useEffect, useState } from "react";
import * as api from "../lib/api";
import { Logo } from "./Logo";
import { Button } from "./Button";
import { CheckIcon, CircleIcon } from "./Icon";
import type { WikiStatus } from "../types";

/**
 * First-run setup.
 *
 * Baton indexes a wiki it does not write. Without the `/baton` skill installed
 * in an agent tool, a new user reaches an empty launcher telling them to run a
 * command that does not exist, so the one step the app cannot do for itself is
 * the one thing this screen exists to do.
 *
 * The wiki folder is already created by the time this renders (it is Baton's
 * own data). Installing the skill writes into another tool's config, so it
 * stays an explicit action.
 */
export function Setup({ status, onDone }: { status: WikiStatus; onDone: () => void }) {
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
    <div className="h-full overflow-y-auto px-8 py-10">
      <div className="mx-auto max-w-lg">
        <div className="mb-3 flex items-center gap-2.5">
          <Logo size={20} className="text-brand" />
          <h1 className="font-serif text-display tracking-tight text-ink">
            Set up Baton
          </h1>
        </div>

        <p className="mb-8 text-read text-body">
          Baton reads a wiki that your coding agent writes. It never calls a model itself,
          so the agent needs a <Code>/baton</Code> command to file what it learned.
        </p>

        <Step n={1} done title="Wiki folder created">
          <button
            onClick={() => void api.revealWiki()}
            className="cursor-pointer font-mono text-ui text-muted underline-offset-2 transition-all duration-150 hover:text-ink hover:underline active:scale-[0.98]"
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
            <p className="text-ui leading-relaxed text-body">
              No agent tools found. Install Claude Code, Codex, or Cursor, then reopen
              this screen.
            </p>
          ) : (
            <>
              <ul className="mb-3">
                {detected.map((s) => {
                  const ready = s.installed && !s.outdated;
                  return (
                    <li
                      key={s.name}
                      className="flex items-center gap-2 border-b border-line-soft py-1.5 text-ui text-body last:border-0"
                    >
                      {ready ? (
                        <CheckIcon size={13} className="shrink-0 text-brand" />
                      ) : (
                        <CircleIcon size={13} className="shrink-0 text-faint" />
                      )}
                      {s.name}
                      {s.outdated && (
                        <span className="ml-auto text-meta text-muted">
                          needs updating
                        </span>
                      )}
                    </li>
                  );
                })}
              </ul>
              {missing.length > 0 && (
                <Button variant="primary" disabled={busy} onClick={() => void install()}>
                  {busy
                    ? "Installing…"
                    : `Install for ${missing.length} tool${missing.length > 1 ? "s" : ""}`}
                </Button>
              )}
              {installed && (
                <p className="mt-2 text-ui text-brand">
                  Installed for {installed.join(", ")}.
                </p>
              )}
              {error && <p className="mt-2 text-ui text-danger">{error}</p>}
            </>
          )}
        </Step>

        <Step n={3} done={status.pageCount > 0} title="File your first session">
          <p className="text-ui leading-relaxed text-body">
            Finish a piece of work with your agent, then run <Code>/baton</Code>. It files
            what the session learned without asking. After that, press the hotkey anywhere
            to copy a project's context into a fresh session.
          </p>
        </Step>

        <div className="mt-8 border-t border-line pt-5">
          <Button onClick={onDone}>
            {status.pageCount > 0 ? "Done" : "Skip for now"}
          </Button>
        </div>
      </div>
    </div>
  );
}

function Code({ children }: { children: React.ReactNode }) {
  return (
    <code className="rounded bg-panel px-1 py-0.5 font-mono text-ui text-ink">
      {children}
    </code>
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
    <section className="mb-7 flex gap-3.5">
      <span
        className={`mt-0.5 flex size-5 shrink-0 items-center justify-center rounded-full text-meta font-medium ${
          done ? "bg-brand text-on-brand" : "bg-panel text-muted"
        }`}
      >
        {done ? <CheckIcon size={12} /> : n}
      </span>
      <div className="min-w-0 flex-1">
        <h2 className="mb-1.5 text-ui font-medium text-ink">{title}</h2>
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
    api
      .wikiStatus()
      .then(setStatus)
      .catch(() => setStatus(null));
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
