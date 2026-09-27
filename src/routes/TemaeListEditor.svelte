<script lang="ts">
  // Temae tab of the list editor: add a procedure or a familiar name (alias), edit
  // names / category / defaults, hide, delete user rows, copy a row as YAML for the pack.
  import { api } from "$lib/api";
  import { t, term, axisLabel } from "$lib/i18n.svelte";
  import { active, categoryTree, loadVocab, packName, uncategorisedTemae, vocab, type CatNode } from "$lib/vocab.svelte";
  import { NONE, TSUKIMONO } from "$lib/types";
  import type { Vocab, AxisValues } from "$lib/types";

  let { showArchived }: { showArchived: boolean } = $props();

  type Draft = {
    mode: "main" | "alias";
    ja: string; romaji: string; en: string;
    category: string | null;       // temae-cat id
    aliasOf: string;               // temae id (alias mode)
    axes: AxisValues;              // defaults (main) or modifiers (alias)
  };
  const blank = (): Draft => ({ mode: "main", ja: "", romaji: "", en: "", category: null, aliasOf: "", axes: {} });
  let add = $state<Draft>(blank());
  let editing = $state<string | null>(null);
  let edit = $state<Draft>(blank());
  let msg = $state<string | null>(null);

  const tree = $derived(categoryTree());
  const mainTemae = $derived(active("temae").filter((v) => !aliasTarget(v)));
  const tanas = $derived(active("tana"));

  function aliasTarget(v: Vocab) { return (v.meta as { alias_of?: string } | null)?.alias_of ?? null; }
  function metaOf(v: Vocab) { return (v.meta as Record<string, unknown> | null) ?? {}; }
  function axesOf(v: Vocab): AxisValues {
    const m = metaOf(v);
    return ((aliasTarget(v) ? m.modifiers : m.defaults) as AxisValues | undefined) ?? {};
  }
  function isPinned(v: Vocab) { return metaOf(v).pinned === true; }
  function isPack(v: Vocab) { return v.origin.startsWith("pack:"); }
  function catLabel(id: string | null): string {
    if (!id) return "";
    const c = vocab.byId[id];
    if (!c) return "";
    const parent = c.parent_id ? vocab.byId[c.parent_id] : null;
    return parent ? `${term(parent)} › ${term(c)}` : term(c);
  }
  function slug(s: string) {
    const base = s.normalize("NFKD").replace(/[^\w]+/g, "-").replace(/^-|-$/g, "").toLowerCase();
    return base || Math.random().toString(36).slice(2, 8);
  }
  function cleanAxes(a: AxisValues): AxisValues {
    const out: AxisValues = {};
    for (const k of ["tea", "tana", "hearth", "katte", "room", "kinin"] as const) if (a[k]) out[k] = a[k];
    if (a.tsukimono?.length) out.tsukimono = [...a.tsukimono];
    return out;
  }
  function axesSummary(a: AxisValues): string {
    const parts: string[] = [];
    if (a.tea) parts.push(axisLabel("tea", a.tea));
    if (a.tana) parts.push(a.tana === NONE ? axisLabel("tana", NONE) : term(vocab.byId[a.tana]) || a.tana);
    if (a.hearth) parts.push(term(vocab.byId["hearth." + a.hearth]) || a.hearth);
    if (a.katte) parts.push(axisLabel("katte", a.katte));
    if (a.room) parts.push(axisLabel("room", a.room));
    if (a.kinin) parts.push(axisLabel("kinin", a.kinin));
    if (a.tsukimono?.length) parts.push(a.tsukimono.map((k) => t("tsukimono." + k)).join("・"));
    return parts.join(" / ");
  }

  // category options, flattened with indent
  type CatOpt = { id: string; label: string };
  function flatCats(nodes: CatNode[], depth: number, out: CatOpt[]) {
    for (const n of nodes) { out.push({ id: n.cat.id, label: "　".repeat(depth) + term(n.cat) }); flatCats(n.children, depth + 1, out); }
  }
  const catOpts = $derived.by(() => { const o: CatOpt[] = []; flatCats(tree, 0, o); return o; });

  function categorySlug(catId: string | null): string | undefined {
    if (!catId) return undefined;
    // temae-cat.urasenke.konarai.mae8 → konarai.mae8 (pack category key kept in meta.category)
    const parts = catId.split(".");
    return parts.length >= 3 ? parts.slice(2).join(".") : parts.slice(1).join(".");
  }

  function toVocab(d: Draft, existing?: Vocab): Vocab {
    const ja = d.ja.trim() || d.romaji.trim();
    const meta: Record<string, unknown> = { ...(existing ? metaOf(existing) : {}) };
    delete meta.alias_of; delete meta.modifiers; delete meta.defaults; delete meta.category;
    if (d.mode === "alias") {
      meta.alias_of = d.aliasOf;
      meta.modifiers = cleanAxes(d.axes);
    } else {
      const ax = cleanAxes(d.axes);
      if (Object.keys(ax).length) meta.defaults = ax;
      const cs = categorySlug(d.category);
      if (cs) meta.category = cs;
    }
    if (existing && isPack(existing)) meta.pinned = true;
    return {
      id: existing?.id ?? `temae.user.${slug(d.romaji || d.ja)}-${Date.now().toString(36)}`,
      kind: "temae", ja, romaji: d.romaji.trim() || null, en: d.en.trim() || null,
      parent_id: d.mode === "alias" ? (vocab.byId[d.aliasOf]?.parent_id ?? d.category) : d.category,
      sort: existing?.sort ?? 999, origin: existing?.origin ?? "user", archived: existing?.archived ?? false,
      meta: Object.keys(meta).length ? meta : null,
    };
  }
  async function submitAdd() {
    if (!(add.ja.trim() || add.romaji.trim())) return;
    if (add.mode === "alias" && !add.aliasOf) return;
    await api.vocabUpsert(toVocab(add));
    add = { ...blank(), mode: add.mode, category: add.category };
    await loadVocab();
  }
  function startEdit(v: Vocab) {
    editing = v.id;
    const a = aliasTarget(v);
    edit = {
      mode: a ? "alias" : "main", ja: v.ja, romaji: v.romaji ?? "", en: v.en ?? "",
      category: v.parent_id ?? null, aliasOf: a ?? "", axes: JSON.parse(JSON.stringify(axesOf(v))),
    };
  }
  async function commitEdit(v: Vocab) {
    await api.vocabUpsert(toVocab(edit, v));
    editing = null;
    await loadVocab();
  }
  async function resetToPack(v: Vocab) {
    await api.packResetRow(v.id);
    editing = null;
    await loadVocab();
  }
  async function toggleArchive(v: Vocab) { await api.vocabArchive(v.id, !v.archived); await loadVocab(); }
  async function remove(v: Vocab) {
    msg = null;
    try { await api.vocabDelete(v.id); await loadVocab(); }
    catch (e) { const m = String(e); const n = /used by (\d+)/.exec(m)?.[1]; msg = n ? t("lists.inUse", { name: term(v), n }) : m; }
  }
  function toggleTsukimono(d: Draft, k: string) {
    const list = d.axes.tsukimono ?? [];
    const i = list.indexOf(k);
    if (i >= 0) list.splice(i, 1); else list.push(k);
    d.axes.tsukimono = list;
  }

  /** Pack-file snippet for a row (ids unscoped, tana without pack prefix). */
  function toYaml(v: Vocab): string {
    const strip = (id: string) => id.replace(/^temae\.[^.]+\./, "").replace(/^temae\./, "");
    const stripTana = (id: string) => (id === "-" || id === "?" ? `"${id}"` : id.replace(/^tana\.[^.]+\./, "").replace(/^tana\./, ""));
    const a = axesOf(v);
    const ax: string[] = [];
    if (a.tea) ax.push(`tea: ${a.tea === "-" ? '"-"' : a.tea}`);
    if (a.tana) ax.push(`tana: ${stripTana(a.tana)}`);
    if (a.hearth) ax.push(`hearth: ${a.hearth}`);
    if (a.katte) ax.push(`katte: ${a.katte}`);
    if (a.room) ax.push(`room: ${a.room}`);
    if (a.kinin) ax.push(`kinin: ${a.kinin === "-" ? '"-"' : a.kinin}`);
    if (a.tsukimono?.length) ax.push(`tsukimono: [${a.tsukimono.join(", ")}]`);
    const lines = [`  - id: ${strip(v.id)}`, `    ja: ${v.ja}`];
    if (v.romaji) lines.push(`    romaji: ${v.romaji}`);
    if (v.en) lines.push(`    en: ${v.en}`);
    const cat = categorySlug(v.parent_id ?? null);
    if (cat) lines.push(`    category: ${cat}`);
    const alias = aliasTarget(v);
    if (alias) { lines.push(`    alias_of: ${strip(alias)}`); lines.push(`    modifiers: { ${ax.join(", ")} }`); }
    else if (ax.length) lines.push(`    defaults: { ${ax.join(", ")} }`);
    return lines.join("\n") + "\n";
  }
  async function copyYaml(v: Vocab) {
    try { await navigator.clipboard.writeText(toYaml(v)); msg = t("lists.yamlCopied"); }
    catch { msg = toYaml(v); }
  }

  const tsukimonoText = (d: Draft) => (d.axes.tsukimono ?? []);
</script>

{#snippet axesForm(d: Draft)}
  <div class="grid grid-cols-4 gap-2">
    <label><div class="label">{t("temae.tea")}</div>
      <select class="field" bind:value={d.axes.tea}><option value={undefined}>—</option>{#each ["koicha", "usucha", "tsuzuki", NONE] as x}<option value={x}>{axisLabel("tea", x)}</option>{/each}</select></label>
    <label><div class="label">{t("temae.tana")}</div>
      <select class="field" bind:value={d.axes.tana}><option value={undefined}>—</option><option value={NONE}>{axisLabel("tana", NONE)}</option>{#each tanas as x}<option value={x.id}>{x.parent_id ? "　" : ""}{term(x)}</option>{/each}</select></label>
    <label><div class="label">{t("temae.hearth")}</div>
      <select class="field" bind:value={d.axes.hearth}><option value={undefined}>—</option>{#each active("hearth") as h}<option value={h.id.replace("hearth.", "")}>{term(h)}</option>{/each}</select></label>
    <label><div class="label">{t("temae.kinin")}</div>
      <select class="field" bind:value={d.axes.kinin}><option value={undefined}>—</option>{#each [NONE, "kinindate", "kiyotsugu"] as x}<option value={x}>{axisLabel("kinin", x)}</option>{/each}</select></label>
    <label><div class="label">{t("temae.katte")}</div>
      <select class="field" bind:value={d.axes.katte}><option value={undefined}>—</option>{#each ["jun", "gyaku"] as x}<option value={x}>{axisLabel("katte", x)}</option>{/each}</select></label>
    <label><div class="label">{t("temae.room")}</div>
      <select class="field" bind:value={d.axes.room}><option value={undefined}>—</option>{#each ["koma", "yojohan", "hiroma"] as x}<option value={x}>{axisLabel("room", x)}</option>{/each}</select></label>
    <div class="col-span-2"><div class="label">{t("temae.tsukimono")}</div>
      <div class="flex flex-wrap gap-x-3 gap-y-1 text-xs pt-1">
        {#each TSUKIMONO as k}<label class="flex items-center gap-1"><input type="checkbox" checked={tsukimonoText(d).includes(k)} onchange={() => toggleTsukimono(d, k)} />{t("tsukimono." + k)}</label>{/each}
      </div></div>
  </div>
{/snippet}

{#snippet headForm(d: Draft)}
  <div class="grid grid-cols-3 gap-2">
    <label><div class="label">{t("lists.nameJa")}</div><input class="field" bind:value={d.ja} /></label>
    <label><div class="label">{t("lists.nameLatin")}</div><input class="field" bind:value={d.romaji} /></label>
    <label><div class="label">{t("lists.gloss")}</div><input class="field" bind:value={d.en} /></label>
    {#if d.mode === "alias"}
      <label class="col-span-3"><div class="label">{t("lists.aliasOf")}</div>
        <select class="field" bind:value={d.aliasOf}><option value="">—</option>{#each mainTemae as v}<option value={v.id}>{catLabel(v.parent_id ?? null)} › {term(v)}</option>{/each}</select></label>
    {:else}
      <label class="col-span-3"><div class="label">{t("lists.category")}</div>
        <select class="field" bind:value={d.category}><option value={null}>—</option>{#each catOpts as c}<option value={c.id}>{c.label}</option>{/each}</select></label>
    {/if}
  </div>
{/snippet}

<div class="card max-w-3xl mb-4 flex flex-col gap-2">
  <div class="flex gap-1 items-center">
    <button class="btn py-0.5" class:bg-stone-200={add.mode === "main"} onclick={() => (add.mode = "main")}>{t("lists.newTemae")}</button>
    <button class="btn py-0.5" class:bg-stone-200={add.mode === "alias"} onclick={() => (add.mode = "alias")}>{t("lists.newAlias")}</button>
    <span class="text-xs text-stone-500 ml-2">{add.mode === "alias" ? t("lists.aliasHelp") : t("lists.temaeHelp")}</span>
  </div>
  {@render headForm(add)}
  <div class="label mb-0">{add.mode === "alias" ? t("lists.modifiers") : t("lists.defaults")}</div>
  {@render axesForm(add)}
  <div class="flex justify-end"><button class="btn btn-primary" onclick={submitAdd} disabled={!(add.ja.trim() || add.romaji.trim()) || (add.mode === "alias" && !add.aliasOf)}>＋ {t("common.save")}</button></div>
</div>

{#if msg}<p class="text-sm text-clay mb-2 whitespace-pre-wrap font-mono text-xs">{msg}</p>{/if}

{#snippet rowsOf(items: Vocab[])}
  {#each items.filter((v) => showArchived || !v.archived) as v (v.id)}
    <tr class="border-b border-line/60" class:opacity-40={v.archived}>
      {#if editing === v.id}
        <td colspan="3" class="py-2">
          <div class="flex flex-col gap-2">
            {@render headForm(edit)}
            <div class="label mb-0">{edit.mode === "alias" ? t("lists.modifiers") : t("lists.defaults")}</div>
            {@render axesForm(edit)}
            <div class="flex gap-2 items-center">
              {#if isPack(v)}<span class="text-xs text-stone-400">{t("lists.pinHint")}</span>{/if}
              <span class="grow"></span>
              {#if isPack(v) && isPinned(v)}<button class="btn py-0.5" onclick={() => resetToPack(v)}>{t("lists.resetPack")}</button>{/if}
              <button class="btn btn-primary py-0.5" onclick={() => commitEdit(v)}>{t("common.save")}</button>
              <button class="btn py-0.5" onclick={() => (editing = null)}>{t("common.cancel")}</button>
            </div>
          </div>
        </td>
      {:else}
        <td class="py-1 pr-3">
          <span class:pl-4={!!aliasTarget(v)}>{term(v)}</span>
          {#if aliasTarget(v)}<span class="text-[11px] text-stone-400"> = {term(vocab.byId[aliasTarget(v)!])}</span>{/if}
          {#if isPinned(v)}<span class="ml-1 text-[10px] rounded px-1 bg-clay/15 text-clay">{t("lists.edited")}</span>{/if}
          {#if axesSummary(axesOf(v))}<div class="text-[11px] text-stone-400">{axesSummary(axesOf(v))}</div>{/if}
        </td>
        <td class="py-1 pr-3 text-stone-400 text-xs whitespace-nowrap">{v.en ?? ""}</td>
        <td class="py-1 pr-3 text-stone-400 text-xs whitespace-nowrap">{packName(v) ?? t("lists.origin.user")}</td>
      {/if}
      <td class="py-1 text-right whitespace-nowrap text-xs text-stone-400">
        {#if editing !== v.id}
          <button class="px-1 hover:text-ink" onclick={() => startEdit(v)}>{t("common.edit")}</button>
          <button class="px-1 hover:text-ink" onclick={() => toggleArchive(v)}>{v.archived ? "↩" : t("lists.archive")}</button>
          {#if v.origin === "user"}
            <button class="px-1 hover:text-ink" onclick={() => copyYaml(v)} title={t("lists.yamlHelp")}>YAML</button>
            <button class="px-1 hover:text-red-600" onclick={() => remove(v)}>{t("common.delete")}</button>
          {/if}
        {/if}
      </td>
    </tr>
  {/each}
{/snippet}

{#snippet catBlock(nodes: CatNode[], depth: number)}
  {#each nodes as n (n.cat.id)}
    <tr class="bg-stone-100/70"><td colspan="4" class="py-1 px-1 text-xs font-medium text-stone-600">{"　".repeat(depth)}{term(n.cat)}</td></tr>
    {@render rowsOf(n.items)}
    {@render catBlock(n.children, depth + 1)}
  {/each}
{/snippet}

<table class="max-w-3xl w-full text-sm">
  <tbody>
    {@render catBlock(tree, 0)}
    {#if uncategorisedTemae().length}
      <tr class="bg-stone-100/70"><td colspan="4" class="py-1 px-1 text-xs font-medium text-stone-600">{t("stats.uncategorized")}</td></tr>
      {@render rowsOf(uncategorisedTemae())}
    {/if}
  </tbody>
</table>
