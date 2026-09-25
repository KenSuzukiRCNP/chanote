<script lang="ts">
  // Photos attached to a saved session. Thumbnails come as data URLs from the backend;
  // clicking opens the original in the system viewer.
  import { onMount, onDestroy } from "svelte";
  import { api, isTauri } from "$lib/api";
  import { pickOpenPaths, openWithSystem } from "$lib/dialogs";
  import { t } from "$lib/i18n.svelte";
  import type { Attachment } from "$lib/types";

  let { sessionId, editable = true }: { sessionId: number; editable?: boolean } = $props();

  let items = $state<Attachment[]>([]);
  let thumbs = $state<Record<number, string>>({});
  let busy = $state(false);
  let dragOver = $state(false);
  let unlisten: (() => void) | null = null;
  const IMG = /\.(jpe?g|png|heic|heif|webp|gif|tiff?|bmp)$/i;

  async function addPaths(paths: string[]) {
    const imgs = paths.filter((p) => IMG.test(p));
    if (!imgs.length) return;
    busy = true;
    try { await api.attachmentAdd(sessionId, imgs); await load(); } finally { busy = false; }
  }

  async function load() {
    items = await api.attachmentList(sessionId);
    for (const a of items) {
      if (!thumbs[a.id]) thumbs[a.id] = (await api.attachmentThumb(a.id)) ?? "";
    }
  }
  onMount(async () => {
    await load();
    if (isTauri) {
      // Files dropped anywhere on the window arrive here with their absolute paths.
      const { getCurrentWebview } = await import("@tauri-apps/api/webview");
      unlisten = await getCurrentWebview().onDragDropEvent((ev) => {
        const p = ev.payload;
        if (p.type === "enter" || p.type === "over") dragOver = true;
        else if (p.type === "leave") dragOver = false;
        else if (p.type === "drop") { dragOver = false; void addPaths(p.paths); }
      });
    }
  });
  onDestroy(() => { unlisten?.(); });

  async function add() {
    const paths = await pickOpenPaths({ multiple: true, ext: ["jpg", "jpeg", "png", "heic", "heif", "webp", "gif", "tiff", "bmp"], title: t("photo.add") });
    await addPaths(paths);
  }
  async function remove(a: Attachment) {
    await api.attachmentDelete(a.id);
    await load();
  }
  async function open(a: Attachment) {
    const p = await api.attachmentPath(a.id);
    if (p) await openWithSystem(p);
  }
  async function caption(a: Attachment, v: string) {
    a.caption = v || null;
    await api.attachmentCaption(a.id, a.caption);
  }
</script>

<div class="rounded-md transition {dragOver ? 'ring-2 ring-matcha bg-matcha/5' : ''}">
<div class="flex items-center gap-3 mb-2">
  <span class="label mb-0">{t("photo.title")}</span>
  <span class="text-[11px] text-stone-400">{t("photo.dropHint")}</span>
  <span class="grow"></span>
  {#if editable}<button class="btn py-0.5 text-xs" onclick={add} disabled={busy}>＋ {t("photo.add")}</button>{/if}
</div>
{#if items.length === 0}
  <p class="text-xs text-stone-400">{t("photo.none")}</p>
{:else}
  <div class="flex flex-wrap gap-3">
    {#each items as a (a.id)}
      <figure class="w-40 flex flex-col gap-1">
        <button class="block w-40 h-28 rounded-md overflow-hidden border border-line bg-stone-100" onclick={() => open(a)} title={a.path}>
          {#if thumbs[a.id]}
            <img src={thumbs[a.id]} alt={a.caption ?? ""} class="w-full h-full object-cover" />
          {:else}
            <span class="text-xs text-stone-400">{t("photo.noThumb")}</span>
          {/if}
        </button>
        {#if editable}
          <span class="flex gap-1">
            <input class="field py-0.5 text-xs" placeholder={t("photo.caption")} value={a.caption ?? ""} onchange={(e) => caption(a, e.currentTarget.value)} />
            <button class="px-1 text-stone-400 hover:text-red-600" onclick={() => remove(a)}>×</button>
          </span>
        {:else if a.caption}
          <figcaption class="text-xs text-stone-500">{a.caption}</figcaption>
        {/if}
      </figure>
    {/each}
  </div>
{/if}
</div>
