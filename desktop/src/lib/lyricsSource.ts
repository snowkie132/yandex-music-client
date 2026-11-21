import { api } from "@/api/client";
import type { Lyrics, Track } from "@/api/types";
import { artistNames } from "@/lib/format";
import type { LyricsSource } from "@/stores/ui/defaults";
import { useUiStore } from "@/stores/ui/index";

export type LyricsOrigin = "betterlyrics" | "lrclib" | "yandex" | "musixmatch";

export interface LyricsResult {
  lyrics: Lyrics;
  origin: LyricsOrigin;
}

export const ORIGIN_LABEL: Record<LyricsOrigin, string> = {
  betterlyrics: "better lyrics",
  lrclib: "lrclib",
  yandex: "яндекс",
  musixmatch: "musixmatch",
};

export function lyricsMode(): LyricsSource {
  return useUiStore().settings.lyricsSource;
}

export function sourceOrder(mode: LyricsSource): LyricsOrigin[] {
  switch (mode) {
    case "betterlyrics":
      return ["betterlyrics"];
    case "lrclib":
      return ["lrclib"];
    case "yandex":
      return ["yandex"];
    case "musixmatch":
      return ["musixmatch"];
    default:
      return ["betterlyrics", "lrclib", "yandex", "musixmatch"];
  }
}

const PAYWALL_MARK = "Подключите подписку";

export function isPaywallLyrics(value: Lyrics | null | undefined): boolean {
  return Boolean(
    value?.lines?.some((line) => (line.text || "").includes(PAYWALL_MARK)),
  );
}

export function hasText(value: Lyrics | null | undefined): value is Lyrics {
  return Boolean(value?.lines?.some((line) => (line.text || "").trim()));
}

function mainArtist(track: Track): string {
  return track.artists?.[0]?.name || artistNames(track.artists || []);
}

async function fromLrclib(
  track: Track,
  force: boolean,
): Promise<Lyrics | null> {
  const found = await api.lrclibLookup({
    title: track.title,
    artist: mainArtist(track),
    album: track.album_title,
    duration: track.duration_ms ? Math.round(track.duration_ms / 1000) : null,
    force,
  });
  if (!found) return null;

  const lyrics: Lyrics = {
    track_id: track.id,
    synced: found.synced,
    lines: found.lines,
    writers: [],
  };
  return hasText(lyrics) ? lyrics : null;
}

function durationSeconds(track: Track): number | null {
  const ms = track.duration_ms ?? 0;
  return ms > 0 ? Math.round(ms / 1000) : null;
}

async function fromBetterLyrics(track: Track): Promise<Lyrics | null> {
  const found = await api
    .betterLyrics(
      track.id.split(":")[0],
      track.title,
      mainArtist(track),
      durationSeconds(track),
    )
    .catch(() => null);
  if (!found || !hasText(found)) return null;
  return { ...found, track_id: track.id };
}

async function fromMusixmatch(track: Track): Promise<Lyrics | null> {
  const found = await api
    .musixmatch(track.id.split(":")[0], track.title, mainArtist(track))
    .catch(() => null);
  if (!found || !hasText(found)) return null;
  return { ...found, track_id: track.id };
}

async function fromYandex(track: Track): Promise<Lyrics | null> {
  const found = await api.lyrics(track.id);
  if (isPaywallLyrics(found)) return null;
  return hasText(found) ? found : null;
}

export async function loadTrackLyrics(
  track: Track,
  mode: LyricsSource = lyricsMode(),
  force = false,
): Promise<LyricsResult | null> {
  const primary = sourceOrder(mode);
  const order =
    mode === "auto"
      ? primary
      : [
          ...primary,
          ...sourceOrder("auto").filter((origin) => !primary.includes(origin)),
        ];
  let lastError: unknown = null;

  for (const origin of order) {
    try {
      const lyrics =
        origin === "betterlyrics"
          ? await fromBetterLyrics(track)
          : origin === "lrclib"
            ? await fromLrclib(track, force)
            : origin === "musixmatch"
              ? await fromMusixmatch(track)
              : await fromYandex(track);
      if (lyrics) return { lyrics, origin };
    } catch (error) {
      lastError = error;
    }
  }

  if (lastError) throw lastError;
  return null;
}
