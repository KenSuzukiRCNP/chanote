// Native save dialog (Tauri) with a browser fallback that just returns a fake path.
import { isTauri } from "./api";

export async function pickSavePath(defaultName: string, ext: string): Promise<string | null> {
  if (!isTauri) {
    console.log(`[mock] save dialog → ${defaultName}`);
    return `/tmp/${defaultName}`;
  }
  const { save } = await import("@tauri-apps/plugin-dialog");
  const path = await save({
    defaultPath: defaultName,
    filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
  });
  return path ?? null;
}

/** Native open dialog. Returns absolute paths (possibly several). */
export async function pickOpenPaths(opts: { multiple?: boolean; ext?: string[]; title?: string } = {}): Promise<string[]> {
  if (!isTauri) {
    console.log("[mock] open dialog", opts);
    return opts.multiple ? ["/tmp/photo1.jpg", "/tmp/photo2.jpg"] : ["/tmp/chanote.json"];
  }
  const { open } = await import("@tauri-apps/plugin-dialog");
  const res = await open({
    multiple: opts.multiple ?? false,
    title: opts.title,
    filters: opts.ext ? [{ name: opts.ext.join("/"), extensions: opts.ext }] : undefined,
  });
  if (!res) return [];
  return Array.isArray(res) ? res : [res];
}

/** Open a file with the OS default app (no-op in the browser). */
export async function openWithSystem(path: string) {
  if (!isTauri) { console.log("[mock] open", path); return; }
  const { openPath } = await import("@tauri-apps/plugin-opener");
  await openPath(path);
}
