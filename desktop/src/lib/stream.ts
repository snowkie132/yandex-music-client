import { convertFileSrc, invoke } from "@tauri-apps/api/core";

const SCHEME = "ymstream";

let base = "";

// WebKitGTK (Linux) не может проигрывать медиа с кастомных схем, поэтому
// в Linux-сборке поток идёт через локальный http-сервер на 127.0.0.1.
export async function initStreamProxy(): Promise<void> {
  try {
    const url = await invoke<string>("stream_base");
    setStreamBase(url);
  } catch {}
}

export function setStreamBase(url: string): void {
  if (typeof url !== "string") return;
  base = url.replace(/\/+$/, "");
}

function encodeTarget(url: string): string {
  return encodeURIComponent(url).replace(
    /[!'()*]/g,
    (char) => `%${char.charCodeAt(0).toString(16).toUpperCase()}`,
  );
}

export function proxyStream(url: string): string {
  if (!url.startsWith("http://") && !url.startsWith("https://")) return url;
  if (base) return `${base}/${encodeTarget(url)}`;
  try {
    return convertFileSrc(url, SCHEME);
  } catch {
    return url;
  }
}
