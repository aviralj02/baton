# Releasing Baton

Two GitHub Actions workflows do the work. You never build a release by hand.

| Workflow | Fires when | Produces |
|---|---|---|
| `.github/workflows/ci.yml` | every push to `main`, every pull request | pass/fail only |
| `.github/workflows/release.yml` | you push a tag starting with `v` | a **draft** release with the installers attached |

CI is the safety net. Release is the button. Nothing is published to the world
until you click Publish yourself, so a bad tag costs you a deleted draft, not a
recall.

---

## One-time setup

Do this once, ever.

**1. Push the repo to GitHub.**

```bash
git remote -v          # confirm the remote is there
git push -u origin main
```

**2. Open the Actions tab.** You should see a run called **CI** start on its own.
Let it finish. If GitHub asks you to enable Actions for the repo first, say yes.

**3. Give Actions permission to create releases.**
`Settings → Actions → General → Workflow permissions` → select **Read and write
permissions** → Save. Without this the release job builds fine and then fails at
the last step, which is a confusing way to lose fifteen minutes.

That is the whole setup. There are no secrets to add until you get signing
certificates; see the last section.

---

## Cutting a release

### 1. Start from a green `main`

Open the Actions tab and confirm the latest **CI** run on `main` has a green
tick. If it is red, fix that first — the release workflow does not run the
tests, so a red `main` becomes a broken download.

### 2. Bump the version in three files

They must all match, and they must match the tag you are about to push. Tauri
names the artefacts from `tauri.conf.json`, so a mismatch here ships a file
called `Baton_0.1.0_universal.dmg` inside a release called `v0.2.0`.

| File | Line |
|---|---|
| `package.json` | `"version": "0.2.0"` |
| `src-tauri/tauri.conf.json` | `"version": "0.2.0"` |
| `src-tauri/Cargo.toml` | `version = "0.2.0"` |

Then refresh the lockfile, which records the version too:

```bash
cd src-tauri && cargo check && cd ..
```

### 3. Commit the bump

```bash
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "Release v0.2.0"
git push
```

### 4. Tag and push the tag

The tag is what triggers the build. Pushing the commit does not.

```bash
git tag v0.2.0
git push origin v0.2.0
```

### 5. Wait

Actions tab → the **Release** run. Two jobs, macOS and Windows, in parallel.
Expect **10–20 minutes** on a cold cache; later releases are faster. A red job
means no installer for that platform — read the failing step's log, fix, and see
*Re-running a release* below.

### 6. Publish the draft

`Releases` in the right-hand sidebar → your new draft. Attached you should find:

- `Baton_0.2.0_universal.dmg` — one download for both Intel and Apple silicon Macs
- `Baton_0.2.0_x64-setup.exe` and/or `Baton_0.2.0_x64_en-US.msi` — Windows

Download and open one before publishing. Then write what changed, keep
**pre-release** ticked while the builds are unsigned, and click **Publish
release**.

---

## Re-running a release

Tags are not special, they are just movable labels — but a tag that has already
built needs clearing on both sides or the run will not start again.

```bash
git tag -d v0.2.0                  # delete locally
git push origin :refs/tags/v0.2.0  # delete on GitHub
```

Then delete the draft release in the GitHub UI (deleting the tag does not delete
it), fix whatever was wrong, commit, and tag again.

To rehearse the whole thing without announcing anything, push `v0.0.1-test`,
watch it build, then delete the tag and the draft. The workflow is deliberately
tag-only — there is no "Run workflow" button — because running it on a branch
would cut a release named `main`.

---

## When something fails

**CI red on `windows-latest` but green on macOS.** Expected, eventually: the
non-macOS `#[cfg]` branches only compile on Windows and nobody has ever built
them. That is exactly what this job is for. Read the compiler error, fix, push.

**`pnpm install --frozen-lockfile` fails.** Someone changed `package.json`
without committing the updated `pnpm-lock.yaml`. Run `pnpm install` locally and
commit the lockfile.

**Release job fails at the last step with a permissions error.** Step 3 of
one-time setup was skipped.

**The build succeeded but there is no draft release.** Look at the tag you
pushed — it must begin with `v`. `0.2.0` does not trigger anything; `v0.2.0`
does.

---

## Later: signing

Right now the installers are unsigned. Windows users click through a SmartScreen
warning; macOS users have to approve the app under System Settings → Privacy &
Security, because since macOS Sequoia the Control-click → Open shortcut no longer
works. This is documented in the release notes the workflow writes.

When you have the certificates, add these as repository secrets under
`Settings → Secrets and variables → Actions`. No workflow edits are needed — the
names are already referenced in `release.yml`.

| Platform | Secrets |
|---|---|
| macOS | `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` |
| Windows | `WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PASSWORD` |

Then drop the pre-release tick, and update the Status section in `README.md`.
