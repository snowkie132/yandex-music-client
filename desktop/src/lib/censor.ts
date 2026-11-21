import { api } from "@/api/client";
import type { Track } from "@/api/types";
import { artistNames } from "@/lib/format";

const LIST_URL =
  "https://raw.githubusercontent.com/Hazzz895/FckCensorData/refs/heads/main/list.json";
const CACHE_KEY = "mashiro.censor.map";
const TTL = 1000 * 60 * 60 * 24;

let map: Record<string, string> = {};
let loaded = false;
let loading: Promise<void> | null = null;

function key(id: string): string {
  return id.split(":")[0];
}

function loadCache(): Record<string, string> | null {
  try {
    const raw = localStorage.getItem(CACHE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as {
      at: number;
      map: Record<string, string>;
    };
    if (!parsed || typeof parsed.map !== "object" || !parsed.map) return null;
    if (Date.now() - (parsed.at || 0) > TTL) return null;
    return parsed.map;
  } catch {
    return null;
  }
}

function persist(): void {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify({ at: Date.now(), map }));
  } catch {}
}

export async function ensureCensorList(): Promise<void> {
  if (loaded) return;
  const cached = loadCache();
  if (cached) {
    map = cached;
    loaded = true;
    return;
  }
  if (loading) return loading;
  loading = fetch(LIST_URL)
    .then((r) =>
      r.ok ? r.json() : Promise.reject(new Error(String(r.status))),
    )
    .then((data: unknown) => {
      const tracks =
        data && typeof data === "object"
          ? ((data as { tracks?: Record<string, unknown> }).tracks ?? {})
          : {};
      const next: Record<string, string> = {};
      for (const [id, url] of Object.entries(tracks)) {
        if (typeof url === "string" && url) next[key(String(id))] = url;
      }
      map = next;
      loaded = true;
      altCache.clear();
      persist();
    })
    .catch(() => {
      loaded = true;
    })
    .finally(() => {
      loading = null;
    });
  return loading;
}

export function isCensored(id: string | null | undefined): boolean {
  if (!id) return false;
  return Boolean(map[key(String(id))]);
}

export function censorUrl(id: string | null | undefined): string | null {
  if (!id) return null;
  return map[key(String(id))] ?? null;
}

const altCache = new Map<string, string | null>();

function norm(value: string): string {
  return value
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, " ")
    .trim();
}

// Если чистого варианта этого трека в списке нет, ищем тот же трек в других
// изданиях (сингл, делюкс, ремастер) — там он может быть под другим id.
export async function censorUrlFor(track: Track): Promise<string | null> {
  const direct = censorUrl(track.id);
  if (direct) return direct;
  const id = key(track.id);
  if (altCache.has(id)) return altCache.get(id) ?? null;
  let found: string | null = null;
  try {
    const query = `${track.title} ${artistNames(track.artists)}`.trim();
    const result = await api.search(query);
    const wanted = norm(track.title);
    for (const item of result.tracks || []) {
      if (key(item.id) === id) continue;
      if (norm(item.title) !== wanted) continue;
      const url = censorUrl(item.id);
      if (url) {
        found = url;
        break;
      }
    }
  } catch {}
  altCache.set(id, found);
  return found;
}

const PREFER_KEY = "mashiro.censor.prefer";

type Preference = "clean" | "original";

function readPrefer(): Record<string, Preference> {
  try {
    const raw = localStorage.getItem(PREFER_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, Preference>;
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

let prefer: Record<string, Preference> = readPrefer();

function persistPrefer(): boolean {
  try {
    localStorage.setItem(PREFER_KEY, JSON.stringify(prefer));
    return true;
  } catch {
    return false;
  }
}

export function prefersOriginal(id: string | null | undefined): boolean {
  if (!id) return false;
  return prefer[key(String(id))] === "original";
}

export function setPrefersOriginal(
  id: string | null | undefined,
  value: boolean,
): boolean {
  if (!id) return false;
  const k = key(String(id));
  if (value) prefer[k] = "original";
  else delete prefer[k];
  return persistPrefer();
}
