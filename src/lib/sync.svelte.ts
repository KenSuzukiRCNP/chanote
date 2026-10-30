// Background folder sync: runs at start, a few seconds after a save, every few minutes,
// and when the app comes back to the foreground. `sync.tick` increments whenever changes
// arrived, so lists can reload.
import { api } from "./api";
import { loadVocab } from "./vocab.svelte";
import type { SyncInfo, SyncReport } from "./types";

export const sync = $state({
  info: null as SyncInfo | null,
  running: false,
  last: null as SyncReport | null,
  error: null as string | null,
  tick: 0,
});

const INTERVAL_MS = 3 * 60 * 1000;
let timer: ReturnType<typeof setInterval> | null = null;
let soon: ReturnType<typeof setTimeout> | null = null;

export async function refreshSyncInfo() {
  try { sync.info = await api.syncStatus(); } catch { sync.info = null; }
}

export async function syncNow(): Promise<SyncReport | null> {
  if (sync.running || !sync.info?.enabled) return null;
  sync.running = true;
  try {
    const r = await api.syncNow();
    sync.last = r; sync.error = null;
    if (r.received > 0) { await loadVocab(); sync.tick++; }
    return r;
  } catch (e) {
    sync.error = String(e);
    return null;
  } finally {
    sync.running = false;
    await refreshSyncInfo();
  }
}

/** Call after a local change; batches several quick edits into one round. */
export function syncSoon(delayMs = 4000) {
  if (!sync.info?.enabled) return;
  if (soon) clearTimeout(soon);
  soon = setTimeout(() => { soon = null; void syncNow(); }, delayMs);
}

export async function startAutoSync() {
  await refreshSyncInfo();
  if (timer) clearInterval(timer);
  if (!sync.info?.enabled) return;
  void syncNow();
  timer = setInterval(() => void syncNow(), INTERVAL_MS);
}

if (typeof document !== "undefined") {
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "visible") void syncNow();
  });
}
