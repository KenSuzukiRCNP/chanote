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

/** Read a picked file's bytes (mobile: the picker may return a content:// URI that only
 *  the fs plugin can open; desktop paths work too). */
export async function readPickedBytes(path: string): Promise<Uint8Array> {
  if (!isTauri) return new Uint8Array();
  const { readFile } = await import("@tauri-apps/plugin-fs");
  return await readFile(path);
}

export async function readPickedText(path: string): Promise<string> {
  if (!isTauri) return "{}";
  const { readTextFile } = await import("@tauri-apps/plugin-fs");
  return await readTextFile(path);
}

/** Android: save dialog returns a content:// URI; write through the fs plugin. */
export async function writePickedBytes(path: string, bytes: Uint8Array) {
  if (!isTauri) return;
  const { writeFile } = await import("@tauri-apps/plugin-fs");
  await writeFile(path, bytes);
}

export async function writePickedText(path: string, text: string) {
  if (!isTauri) return;
  const { writeTextFile } = await import("@tauri-apps/plugin-fs");
  await writeTextFile(path, text);
}

/** File name from a path or content URI. */
export function baseName(path: string): string {
  const q = path.split(/[?#]/)[0];
  const last = decodeURIComponent(q.split(/[\\/]/).pop() ?? "file");
  return last.split(":").pop() || "file";
}

/** Desktop folder picker (iOS uses the native picker inside the sync_choose_folder command). */
export async function pickFolder(title: string): Promise<string | null> {
  if (!isTauri) return "/mock/iCloud Drive";
  const { open } = await import("@tauri-apps/plugin-dialog");
  const res = await open({ directory: true, title });
  return (Array.isArray(res) ? res[0] : res) ?? null;
}

/** iOS: HEIC photos become JPEG before they reach the app (no HEIC decoder in Rust; the
 *  WebView can decode them). Orientation is applied by createImageBitmap. Falls back to the
 *  original bytes if decoding fails. */
export async function heicToJpeg(name: string, bytes: Uint8Array): Promise<{ name: string; bytes: Uint8Array }> {
  if (!/\.(heic|heif)$/i.test(name)) return { name, bytes };
  try {
    const blob = new Blob([bytes as BlobPart], { type: "image/heic" });
    const bmp = await createImageBitmap(blob, { imageOrientation: "from-image" });
    const canvas = document.createElement("canvas");
    canvas.width = bmp.width; canvas.height = bmp.height;
    canvas.getContext("2d")!.drawImage(bmp, 0, 0);
    const jpg: Blob | null = await new Promise((res) => canvas.toBlob(res, "image/jpeg", 0.9));
    if (!jpg) return { name, bytes };
    return { name: name.replace(/\.(heic|heif)$/i, ".jpg"), bytes: new Uint8Array(await jpg.arrayBuffer()) };
  } catch {
    return { name, bytes };
  }
}
