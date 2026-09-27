<script lang="ts">
  // Import wizard: choose whose diary this is (self swap), review every record with
  // duplicate warnings, tick what to import.
  import { api } from "$lib/api";
  import { t } from "$lib/i18n.svelte";
  import { loadVocab } from "$lib/vocab.svelte";
  import type { ImportPreview, ImportReport } from "$lib/types";

  let { path, onclose }: { path: string; onclose: (report: ImportReport | null) => void } = $props();

  let preview = $state<ImportPreview | null>(null);
  let selfAs = $state<string>("");          // "" = file's own self
  let authorName = $state("");
  let checked = $state<Record<number, boolean>>({});
  let busy = $state(false);
  let err = $state<string | null>(null);

  const fileSelf = $derived(preview?.persons.find((p) => p.is_self) ?? null);
  const swapped = $derived(!!selfAs && selfAs !== fileSelf?.id);
  const nDup = $derived(preview?.rows.filter((r) => r.duplicate_of != null).length ?? 0);
  const nChecked = $derived(Object.values(checked).filter(Boolean).length);

  async function load() {
    busy = true; err = null;
    try {
      preview = await api.importPreview(path, swapped ? { self_as: selfAs, original_self_name: authorName || null } : {});
      checked = Object.fromEntries(preview.rows.map((r) => [r.index, r.duplicate_of == null]));
    } catch (e) { err = String(e); } finally { busy = false; }
  }
  $effect(() => { void selfAs; void load(); });

  async function apply() {
    busy = true; err = null;
    try {
      const include = Object.entries(checked).filter(([, v]) => v).map(([k]) => Number(k));
      const rep = await api.importApply(path, { self_as: swapped ? selfAs : null, original_self_name: swapped ? authorName || null : null, include });
      await loadVocab();
      onclose(rep);
    } catch (e) { err = String(e); busy = false; }
  }
  const setAll = (v: boolean) => { for (const k of Object.keys(checked)) checked[Number(k)] = v; };
</script>

<div class="fixed inset-0 bg-black/30 flex items-center justify-center z-50" role="dialog" aria-modal="true">
  <div class="card w-[44rem] max-h-[85vh] flex flex-col gap-3">
    <div class="font-semibold">{t("import.title")}</div>
    <div class="text-xs text-stone-500 break-all">{path}</div>

    {#if preview}
      <div class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-2 items-center text-sm">
        <span class="label mb-0">{t("import.whoAmI")}</span>
        <select class="field" bind:value={selfAs}>
          <option value="">{t("import.fileSelf", { name: fileSelf?.name ?? "—" })}</option>
          {#each preview.persons.filter((p) => !p.is_self) as p}
            <option value={p.id}>{p.name}（{p.n}）</option>
          {/each}
        </select>
        {#if swapped}
          <span class="label mb-0">{t("import.authorName")}</span>
          <input class="field" bind:value={authorName} placeholder={t("import.authorPlaceholder")} onchange={load} />
          <span></span>
          <span class="text-xs text-stone-500">{t("import.swapHelp")}</span>
        {/if}
      </div>

      <div class="flex items-center gap-3 text-sm whitespace-nowrap">
        <span>{t("import.records", { n: preview.rows.length })}</span>
        {#if nDup}<span class="text-clay">{t("import.duplicates", { n: nDup })}</span>{/if}
        <span class="text-stone-400">{t("import.vocabNew", { n: preview.vocab_new })}</span>
        <span class="grow"></span>
        <button class="btn py-0.5 text-xs" onclick={() => setAll(true)}>{t("import.selectAll")}</button>
        <button class="btn py-0.5 text-xs" onclick={() => setAll(false)}>{t("import.selectNone")}</button>
      </div>

      <div class="overflow-auto border border-line rounded-md grow min-h-40">
        <table class="text-sm w-full">
          <tbody>
            {#each preview.rows as r (r.index)}
              <tr class="border-b border-line/60 {r.duplicate_of != null ? 'bg-clay/5' : ''}">
                <td class="px-2 py-1 w-6"><input type="checkbox" bind:checked={checked[r.index]} /></td>
                <td class="px-2 py-1 font-mono text-xs whitespace-nowrap">{r.date ?? "—"}</td>
                <td class="px-2 py-1 whitespace-nowrap">{r.place}</td>
                <td class="px-2 py-1 text-stone-600">{r.summary}</td>
                <td class="px-2 py-1 text-xs whitespace-nowrap">
                  {#if r.duplicate_of != null}<span class="text-clay" title={t("import.dupHelp")}>⚠ {t("import.dup")}</span>{/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {:else if busy}
      <p class="text-stone-500">{t("common.loading")}</p>
    {/if}
    {#if err}<p class="text-sm text-red-700">{err}</p>{/if}

    <div class="flex gap-2 justify-end">
      <button class="btn" onclick={() => onclose(null)} disabled={busy}>{t("common.cancel")}</button>
      <button class="btn btn-primary" onclick={apply} disabled={busy || nChecked === 0}>{t("import.go", { n: nChecked })}</button>
    </div>
  </div>
</div>
