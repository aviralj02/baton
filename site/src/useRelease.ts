import { useEffect, useState } from "react";

const REPO = "aviralj02/baton";
export const RELEASES_URL = `https://github.com/${REPO}/releases`;

export type Platform = "mac" | "windows" | "other";

export type Download = {
  url: string;
  size: string;
};

export type Release = {
  version: string;
  mac?: Download;
  windows?: Download;
};

/**
 * The latest release's assets, looked up at page load.
 *
 * Tauri names assets with the version in them, so a static
 * `releases/latest/download/Baton_0.1.0_universal.dmg` link dies the moment
 * 0.2.0 ships. Matching on extension instead survives every version bump with
 * no change here and none to the release workflow.
 *
 * `null` means the lookup has not finished or did not work, and every button
 * falls back to the releases page. Unauthenticated GitHub allows 60 requests an
 * hour per address, so a shared network can exhaust it, and the page must stay
 * useful when it does.
 */
export function useRelease(): { status: Status; release: Release | null } {
  const [release, setRelease] = useState<Release | null>(null);
  const [status, setStatus] = useState<Status>("loading");

  useEffect(() => {
    let cancelled = false;

    fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: { Accept: "application/vnd.github+json" },
    })
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(String(r.status)))))
      .then((data: { tag_name: string; assets: GitHubAsset[] }) => {
        if (cancelled) return;
        setStatus("ready");
        setRelease({
          version: data.tag_name?.replace(/^v/, "") ?? "",
          mac: pick(data.assets, (name) => name.endsWith(".dmg")),
          // The NSIS installer, not the .msi: it is the one that upgrades in
          // place, which is what the updater hands Windows.
          windows: pick(data.assets, (name) => name.endsWith("-setup.exe")),
        });
      })
      .catch(() => {
        // Rate limited, offline, or nothing released yet. Saying which would be
        // a guess, so the copy covers all three: use the releases page.
        if (!cancelled) setStatus("unavailable");
      });

    return () => {
      cancelled = true;
    };
  }, []);

  return { status, release };
}

/** `unavailable` is not an error to apologise for; it is a link to somewhere useful. */
export type Status = "loading" | "ready" | "unavailable";

type GitHubAsset = { name: string; browser_download_url: string; size: number };

function pick(
  assets: GitHubAsset[],
  matches: (name: string) => boolean,
): Download | undefined {
  const asset = assets?.find((a) => matches(a.name));
  if (!asset) return undefined;
  return {
    url: asset.browser_download_url,
    size: `${Math.round(asset.size / 1_000_000)} MB`,
  };
}

/**
 * Which download to offer first.
 *
 * Nobody should have to know what a .dmg is, so the platform they are on gets
 * the primary button and the other becomes a quiet secondary link. Guessing
 * wrong costs one extra click, which is why this never hides the other option.
 */
export function detectPlatform(): Platform {
  if (typeof navigator === "undefined") return "other";
  const hint = (navigator as NavigatorUAData).userAgentData?.platform;
  const source = `${hint ?? ""} ${navigator.userAgent}`.toLowerCase();
  if (source.includes("mac")) return "mac";
  if (source.includes("win")) return "windows";
  return "other";
}

type NavigatorUAData = Navigator & {
  userAgentData?: { platform?: string };
};
