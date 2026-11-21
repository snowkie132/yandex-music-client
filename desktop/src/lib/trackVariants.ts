export type TrackVariant = "normal" | "speedup" | "slowed";

const STORAGE_KEY = "mashiro.trackVariants";

export const VARIANT_LABEL: Record<TrackVariant, string> = {
  normal: "Обычная",
  speedup: "Sped up",
  slowed: "Slowed",
};

const cache = new Map<string, TrackVariant>();
let loaded = false;

function key(id: string | null | undefined): string {
  return String(id ?? "").split(":")[0];
}

function load(): void {
  if (loaded) return;
  loaded = true;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    for (const [id, value] of Object.entries(parsed)) {
      if (value === "speedup" || value === "slowed") cache.set(id, value);
    }
  } catch {}
}

function persist(): void {
  try {
    const payload: Record<string, TrackVariant> = {};
    for (const [id, value] of cache) payload[id] = value;
    localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
  } catch {}
}

export function variantFor(id: string | null | undefined): TrackVariant {
  load();
  return cache.get(key(id)) ?? "normal";
}

export function setTrackVariant(
  id: string | null | undefined,
  variant: TrackVariant,
): void {
  if (!id) return;
  load();
  const trackId = key(id);
  if (variant === "normal") cache.delete(trackId);
  else cache.set(trackId, variant);
  persist();
}

export function variantRate(
  variant: TrackVariant,
  speedup: number,
  slowed: number,
): number {
  if (variant === "speedup") return speedup;
  if (variant === "slowed") return slowed;
  return 1;
}