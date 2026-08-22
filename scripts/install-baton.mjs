#!/usr/bin/env node
/**
 * Installs the `/baton` closing command and seeds the wiki.
 *
 * The command has to sit at user level, not in this repo, because you run it in
 * whatever repo the session was about. That is why this is an install step and
 * not a checked-in path.
 *
 * The repo copy under skills/ is canonical. This overwrites the tool copies
 * every run, so editing one of them is always the wrong move: it will be
 * silently replaced the next time anyone runs this.
 */
import { existsSync, mkdirSync, copyFileSync, readdirSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repo = dirname(dirname(fileURLToPath(import.meta.url)));
const home = homedir();

/** Every agent tool that reads user-level skills from a home directory. */
const TOOLS = [
  { name: "Claude Code", dir: ".claude" },
  { name: "Codex", dir: ".codex" },
  { name: "Cursor", dir: ".cursor" },
];

const done = [];
const skipped = [];

for (const tool of TOOLS) {
  const base = join(home, tool.dir);
  if (!existsSync(base)) {
    skipped.push(`${tool.name}: ~/${tool.dir} does not exist, tool not installed`);
    continue;
  }
  const target = join(base, "skills", "baton");
  mkdirSync(target, { recursive: true });
  copyFileSync(join(repo, "skills", "baton", "SKILL.md"), join(target, "SKILL.md"));
  done.push(`${tool.name}: ~/${tool.dir}/skills/baton/SKILL.md`);
}

// The wiki itself. One central folder covering every project, by decision, so
// it lives in the home directory rather than inside any repo.
const wiki = join(home, "Baton");
const schema = join(wiki, "AGENTS.md");

if (!existsSync(wiki)) {
  mkdirSync(join(wiki, "projects"), { recursive: true });
  mkdirSync(join(wiki, "concepts"), { recursive: true });
  done.push(`wiki: created ~/Baton`);
}

if (existsSync(schema)) {
  // Never clobber this. It is the contract every page was written against, and
  // it is meant to be edited as the schema is repaired.
  skipped.push("schema: ~/Baton/AGENTS.md already exists, left alone");
} else {
  copyFileSync(join(repo, "skills", "wiki", "AGENTS.md"), schema);
  done.push("schema: ~/Baton/AGENTS.md");
}

console.log("\nBaton install\n");
for (const line of done) console.log(`  installed  ${line}`);
for (const line of skipped) console.log(`  skipped    ${line}`);

if (done.length === 0) {
  console.log("\nNothing to install. No supported agent tool found in your home directory.");
  process.exit(1);
}

const pages = existsSync(wiki)
  ? readdirSync(wiki, { recursive: true }).filter((f) => String(f).endsWith(".md")).length
  : 0;

console.log(`\n  ~/Baton holds ${pages} markdown file(s).`);
console.log("\nRun /baton at the end of a session to file what it learned.");
console.log("Consider `git init ~/Baton` for history. It is local only, no remote needed.\n");
