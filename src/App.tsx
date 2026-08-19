import { useEffect, useState } from "react";
import { Command } from "cmdk";
import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { MOD_LABEL, ENTER_LABEL, hasMod } from "./lib/platform";
import type { Context } from "./types";

// Milestone 1 runs on fake data. Milestone 2 replaces this with
// invoke<Context[]>("list_contexts") / invoke("search_contexts", { query }).
const FAKE_CONTEXTS: Pick<Context, "id" | "name" | "updatedAt">[] = [
  { id: "1", name: "Auth migration", updatedAt: "3h ago" },
  { id: "2", name: "Stripe integration", updatedAt: "yesterday" },
  { id: "3", name: "Dashboard redesign", updatedAt: "Aug 10" },
];

export default function App() {
  const [query, setQuery] = useState("");

  const dismiss = () => {
    setQuery("");
    invoke("hide_launcher");
  };

  // Escape dismisses. The window also hides on blur (Milestone 1 task).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        dismiss();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const copyContext = async (ctx: { name: string }) => {
    // Milestone 2: invoke("copy_context", { id }) renders markdown in Rust.
    await writeText(`# ${ctx.name}\n\n(placeholder)`);
    dismiss();
  };

  return (
    <div className="h-screen w-screen">
      <Command
        className="flex h-full flex-col overflow-hidden rounded-xl border border-black/10 bg-white/60 dark:border-white/10 dark:bg-neutral-900/50"
        onKeyDown={(e) => {
          if (hasMod(e) && e.key === "Enter") {
            e.preventDefault();
          }
        }}
      >
        <div data-tauri-drag-region className="border-b border-black/5 dark:border-white/5">
          <Command.Input
            autoFocus
            value={query}
            onValueChange={setQuery}
            placeholder="Search or create context..."
            className="w-full bg-transparent px-4 py-3.5 text-[15px] outline-none placeholder:text-neutral-400 dark:text-neutral-100"
          />
        </div>

        <Command.List className="flex-1 overflow-y-auto p-2">
          <Command.Empty className="px-3 py-6 text-center text-sm text-neutral-400">
            No contexts found.
          </Command.Empty>

          <Command.Group
            heading="Actions"
            className="[&_[cmdk-group-heading]]:px-3 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-[11px] [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:uppercase [&_[cmdk-group-heading]]:tracking-wide [&_[cmdk-group-heading]]:text-neutral-400"
          >
            <Item onSelect={() => {}}>Create context</Item>
            <Item onSelect={() => {}}>Create handoff from conversation</Item>
          </Command.Group>

          <Command.Group
            heading="Recent"
            className="[&_[cmdk-group-heading]]:px-3 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-[11px] [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:uppercase [&_[cmdk-group-heading]]:tracking-wide [&_[cmdk-group-heading]]:text-neutral-400"
          >
            {FAKE_CONTEXTS.map((ctx) => (
              <Item key={ctx.id} onSelect={() => copyContext(ctx)}>
                <span>{ctx.name}</span>
                <span className="ml-auto text-xs text-neutral-400">{ctx.updatedAt}</span>
              </Item>
            ))}
          </Command.Group>
        </Command.List>

        <div className="flex items-center gap-4 border-t border-black/5 px-4 py-2 text-[11px] text-neutral-400 dark:border-white/5">
          <span>↑↓ Navigate</span>
          <span>{ENTER_LABEL} Select</span>
          <span className="ml-auto">
            {MOD_LABEL}
            {ENTER_LABEL} Copy context
          </span>
        </div>
      </Command>
    </div>
  );
}

function Item({
  children,
  onSelect,
}: {
  children: React.ReactNode;
  onSelect: () => void;
}) {
  return (
    <Command.Item
      onSelect={onSelect}
      className="flex cursor-default items-center gap-2 rounded-md px-3 py-2 text-sm text-neutral-700 data-[selected=true]:bg-neutral-100 dark:text-neutral-200 dark:data-[selected=true]:bg-white/10"
    >
      {children}
    </Command.Item>
  );
}
