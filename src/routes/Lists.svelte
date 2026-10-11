<script lang="ts">
  // Vocabulary editor: places, people, shelves, utensils, temae (aliases).
  // Rows can be renamed, archived, given a parent (places/shelves), flagged as teacher
  // (people), and moved up/down. Temae tab adds aliases of pack temae with modifiers.
  import { api } from "$lib/api";
  import { t, term, axisLabel } from "$lib/i18n.svelte";
  import { loadVocab, vocab, active, packName } from "$lib/vocab.svelte";
  import { NONE, UNKNOWN } from "$lib/types";
  import type { Vocab, AxisValues } from "$lib/types";
  import TemaeListEditor from "./TemaeListEditor.svelte";

  const kinds = ["place", "person", "tana", "tool", "event-kind", "temae"];
  let kind = $state("place");
  let showArchived = $state(false);
  let editing = $state<string | null>(null);
  let draft = $state<{ ja: string; romaji: string; en: string; parent_id: string | null; teacher: boolean; note: string; paired_with: string | null; form: string }>({ ja: "", romaji: "", en: "", parent_id: null, teacher: false, note: "", paired_with: null, form: "chaji" });
  function formOf(v: Vocab): string { return (v.meta as { form?: string } | null)?.form ?? "chaji"; }

  // new plain row
  let newJa = $state("");
  let newRomaji = $state("");
  let newTeacher = $state(false);
  let newParent = $state<string | null>(null);
  let newNote = $state("");
  let newForm = $state<"chaji" | "chakai">("chaji");

  // new alias (temae tab)
  let aliasJa = $state("");
  let aliasRomaji = $state("");
  let aliasOf = $state<string>("");
  let aliasMods = $state<AxisValues>({});
  let aliasTsukimono = $state("");

  const rows = $derived((vocab.byKind[kind] ?? []).filter((v) => showArchived || !v.archived));
  const parentsFor = $derived(
    kind === "place" || kind === "tana" ? active(kind).filter((v) => !v.parent_id)
    : kind === "tool" ? active("tool-cat")
    : []);
  const toolsForPair = $derived(kind === "tool" ? active("tool") : []);
  function catName(v: Vocab): string {
    return v.parent_id ? term(vocab.byId[v.parent_id]) : "";
  }
  function noteOf(v: Vocab): string { return (v.meta as { note?: string } | null)?.note ?? ""; }
  function pairOf(v: Vocab): string | null { return (v.meta as { paired_with?: string } | null)?.paired_with ?? null; }
  const mainTemae = $derived(active("temae").filter((v) => !(v.meta as { alias_of?: string } | null)?.alias_of));
  const tanas = $derived(active("tana"));

  function slug(s: string) {
    const base = s.normalize("NFKD").replace(/[^\w]+/g, "-").replace(/^-|-$/g, "").toLowerCase();
    return base || Math.random().toString(36).slice(2, 8);
  }
  function isSelf(v: Vocab) { return !!(v.meta as { is_self?: boolean } | null)?.is_self; }
  function isTeacher(v: Vocab) { return !!(v.meta as { teacher?: boolean } | null)?.teacher; }
  function aliasTarget(v: Vocab) { return (v.meta as { alias_of?: string } | null)?.alias_of ?? null; }
  function aliasModsOf(v: Vocab): AxisValues { return (v.meta as { modifiers?: AxisValues } | null)?.modifiers ?? {}; }

  async function add() {
    // Japanese is optional for names written only in the alphabet: the display name
    // (`ja`) then falls back to the romaji/alphabet field.
    const ja = newJa.trim() || newRomaji.trim();
    if (!ja) return;
    const v: Vocab = {
      id: `${kind === "event-kind" ? "event" : kind}.${slug(newRomaji || newJa)}-${Date.now().toString(36)}`,
      kind, ja, romaji: newRomaji.trim() || null, en: null,
      parent_id: newParent, sort: (vocab.byKind[kind]?.length ?? 0) + 1, origin: "user", archived: false,
      meta: kind === "person" && newTeacher ? { teacher: true }
          : kind === "tool" && newNote.trim() ? { note: newNote.trim() }
          : kind === "event-kind" ? { form: newForm } : null,
    };
    await api.vocabUpsert(v);
    newJa = ""; newRomaji = ""; newTeacher = false; newNote = "";
    if (kind !== "tool") newParent = null; // keep the category for the next tool
    await loadVocab();
  }

  async function addAlias() {
    const ja = aliasJa.trim() || aliasRomaji.trim();
    if (!ja || !aliasOf) return;
    const mods: AxisValues = {};
    for (const k of ["tea", "tana", "hearth", "katte", "room", "kinin"] as const) {
      const val = aliasMods[k];
      if (val) mods[k] = val;
    }
    const ts = aliasTsukimono.split(/[,\s、]+/).filter(Boolean);
    if (ts.length) mods.tsukimono = ts;
    const main = vocab.byId[aliasOf];
    const v: Vocab = {
      id: `temae.${slug(aliasRomaji || aliasJa)}-${Date.now().toString(36)}`,
      kind: "temae", ja, romaji: aliasRomaji.trim() || null, en: null,
      parent_id: main?.parent_id ?? null, sort: 999, origin: "user", archived: false,
      meta: { alias_of: aliasOf, modifiers: mods },
    };
    await api.vocabUpsert(v);
    aliasJa = ""; aliasRomaji = ""; aliasOf = ""; aliasMods = {}; aliasTsukimono = "";
    await loadVocab();
  }

  function startEdit(v: Vocab) {
    editing = v.id;
    draft = { ja: v.ja, romaji: v.romaji ?? "", en: v.en ?? "", parent_id: v.parent_id ?? null, teacher: isTeacher(v), note: noteOf(v), paired_with: pairOf(v), form: formOf(v) };
  }
  async function commitEdit(v: Vocab) {
    const meta = { ...((v.meta as Record<string, unknown> | null) ?? {}) };
    if (kind === "person") { if (draft.teacher) meta.teacher = true; else delete meta.teacher; }
    if (kind === "event-kind") meta.form = draft.form;
    if (kind === "tool") {
      if (draft.note.trim()) meta.note = draft.note.trim(); else delete meta.note;
      if (draft.paired_with && draft.paired_with !== v.id) meta.paired_with = draft.paired_with; else delete meta.paired_with;
    }
    await api.vocabUpsert({
      ...v, ja: draft.ja.trim() || draft.romaji.trim() || v.ja, romaji: draft.romaji.trim() || null, en: draft.en.trim() || null,
      parent_id: kind === "place" || kind === "tana" || kind === "tool" ? draft.parent_id : v.parent_id,
      meta: Object.keys(meta).length ? meta : null,
    });
    // pairing is symmetric: make the partner point back
    if (kind === "tool" && draft.paired_with && draft.paired_with !== v.id) {
      const partner = vocab.byId[draft.paired_with];
      if (partner && pairOf(partner) !== v.id) {
        await api.vocabUpsert({ ...partner, meta: { ...((partner.meta as Record<string, unknown> | null) ?? {}), paired_with: v.id } });
      }
    }
    editing = null;
    await loadVocab();
  }
  let delMsg = $state<string | null>(null);
  async function remove(v: Vocab) {
    delMsg = null;
    try {
      await api.vocabDelete(v.id);
      await loadVocab();
    } catch (e) {
      const m = String(e);
      const n = /used by (\d+)/.exec(m)?.[1];
      delMsg = n ? t("lists.inUse", { name: term(v), n }) : m;
    }
  }
  async function toggleArchive(v: Vocab) {
    await api.vocabArchive(v.id, !v.archived);
    await loadVocab();
  }
  async function move(v: Vocab, d: number) {
    const list = rows;
    const i = list.findIndex((x) => x.id === v.id);
    const j = i + d;
    if (j < 0 || j >= list.length) return;
    const a = list[i], b = list[j];
    const sa = a.sort ?? i, sb = b.sort ?? j;
    await api.vocabUpsert({ ...a, sort: sb === sa ? sb + d : sb });
    await api.vocabUpsert({ ...b, sort: sa });
    await loadVocab();
  }

  function modsSummary(m: AxisValues): string {
    const parts: string[] = [];
    if (m.tea) parts.push(axisLabel("tea", m.tea));
    if (m.tana) parts.push(m.tana === NONE ? axisLabel("tana", NONE) : term(vocab.byId[m.tana]) || m.tana);
    if (m.hearth) parts.push(term(vocab.byId["hearth." + m.hearth]) || m.hearth);
    if (m.katte) parts.push(axisLabel("katte", m.katte));
    if (m.room) parts.push(axisLabel("room", m.room));
    if (m.kinin) parts.push(axisLabel("kinin", m.kinin));
    if (m.tsukimono?.length) parts.push(m.tsukimono.map((k) => t("tsukimono." + k)).join("・"));
    return parts.join(" / ");
  }
</script>

<h1 class="text-lg font-semibold mb-4">{t("lists.title")}</h1>
<div class="flex gap-1 mb-4 items-center">
  {#each kinds as k}
    <button class="btn" class:bg-stone-200={kind === k} onclick={() => { kind = k; editing = null; }}>{t("lists." + k)}</button>
  {/each}
  <span class="grow"></span>
  <label class="text-sm flex items-center gap-1"><input type="checkbox" bind:checked={showArchived} />{t("lists.showArchived")}</label>
</div>

{#if kind === "temae"}
  <TemaeListEditor {showArchived} />
{:else}
  <div class="card max-w-3xl flex gap-2 items-end mb-4">
    <label class="grow"><div class="label">{t("lists.nameJa")}</div><input class="field" bind:value={newJa} onkeydown={(e) => e.key === "Enter" && add()} /></label>
    <label class="grow"><div class="label">{t("lists.nameLatin")}</div><input class="field" bind:value={newRomaji} onkeydown={(e) => e.key === "Enter" && add()} /></label>
    {#if kind === "person"}
      <label class="text-sm flex items-center gap-1 pb-1.5"><input type="checkbox" bind:checked={newTeacher} />{t("session.teacher")}</label>
    {/if}
    {#if parentsFor.length}
      <label><div class="label">{kind === "tool" ? t("lists.category") : t("lists.parent")}</div>
        <select class="field w-auto" bind:value={newParent}>
          <option value={null}>—</option>
          {#each parentsFor as p}<option value={p.id}>{term(p)}</option>{/each}
        </select></label>
    {/if}
    {#if kind === "tool"}
      <label class="grow"><div class="label">{t("lists.note")}</div><input class="field" bind:value={newNote} placeholder={t("lists.notePlaceholder")} onkeydown={(e) => e.key === "Enter" && add()} /></label>
    {/if}
    {#if kind === "event-kind"}
      <label><div class="label">{t("lists.eventForm")}</div>
        <select class="field w-auto" bind:value={newForm}>
          <option value="chaji">{t("session.form.chaji")}</option>
          <option value="chakai">{t("session.form.chakai")}</option>
        </select></label>
    {/if}
    <button class="btn btn-primary" onclick={add}>＋</button>
  </div>
{/if}

{#if kind !== "temae"}
{#if delMsg}<p class="text-sm text-clay mb-2">{delMsg}</p>{/if}
<table class="max-w-3xl w-full text-sm">
  <tbody>
    {#each rows as v (v.id)}
      <tr class="border-b border-line" class:opacity-40={v.archived}>
        {#if editing === v.id}
          <td class="py-1 pr-2" colspan="3">
            <div class="flex gap-1 flex-wrap items-center">
              <input class="field w-40" bind:value={draft.ja} />
              <input class="field w-32" bind:value={draft.romaji} placeholder="romaji" />
              <input class="field w-40" bind:value={draft.en} placeholder="en" />
              {#if kind === "person" && !isSelf(v)}
                <label class="text-xs flex items-center gap-1"><input type="checkbox" bind:checked={draft.teacher} />{t("session.teacher")}</label>
              {/if}
              {#if parentsFor.length}
                <select class="field w-auto" bind:value={draft.parent_id}>
                  <option value={null}>—</option>
                  {#each parentsFor.filter((p) => p.id !== v.id) as p}<option value={p.id}>{term(p)}</option>{/each}
                </select>
              {/if}
              {#if kind === "event-kind"}
                <select class="field w-auto" bind:value={draft.form}>
                  <option value="chaji">{t("session.form.chaji")}</option>
                  <option value="chakai">{t("session.form.chakai")}</option>
                </select>
              {/if}
              {#if kind === "tool"}
                <input class="field w-72" bind:value={draft.note} placeholder={t("lists.notePlaceholder")} />
                <label class="text-xs flex items-center gap-1">{t("lists.pairedWith")}
                  <select class="field w-auto" bind:value={draft.paired_with}>
                    <option value={null}>—</option>
                    {#each toolsForPair.filter((p) => p.id !== v.id) as p}<option value={p.id}>{term(p)}</option>{/each}
                  </select></label>
              {/if}
              <button class="btn btn-primary py-0.5" onclick={() => commitEdit(v)}>{t("common.save")}</button>
              <button class="btn py-0.5" onclick={() => (editing = null)}>{t("common.cancel")}</button>
            </div>
          </td>
        {:else}
          <td class="py-1 pr-3">
            {v.parent_id && kind !== "temae" && kind !== "tool" ? "　" : ""}{term(v)}
            {#if kind === "tool"}
              {#if v.parent_id}<span class="ml-1 text-[10px] rounded px-1 bg-stone-100 text-stone-500">{catName(v)}</span>{/if}
              {#if pairOf(v)}<span class="ml-1 text-[10px] text-stone-400">⇄ {term(vocab.byId[pairOf(v)!])}</span>{/if}
              {#if noteOf(v)}<div class="text-[11px] text-stone-400 whitespace-pre-wrap">{noteOf(v)}</div>{/if}
            {/if}
            {#if kind === "person" && isTeacher(v)}<span class="ml-1 text-[10px] rounded px-1 bg-matcha/15 text-matcha-deep">{t("session.teacher")}</span>{/if}
            {#if kind === "event-kind"}<span class="ml-1 text-[10px] rounded px-1 bg-stone-100 text-stone-500">{t("session.form." + formOf(v))}</span>{/if}
            {#if kind === "temae" && aliasTarget(v)}
              <div class="text-[11px] text-stone-400">= {term(vocab.byId[aliasTarget(v)!])} / {modsSummary(aliasModsOf(v))}</div>
            {/if}
          </td>
          <td class="py-1 pr-3 text-stone-400 text-xs">{v.en ?? ""}</td>
          <td class="py-1 pr-3 text-stone-400 text-xs">{packName(v) ?? t("lists.origin.user")}</td>
        {/if}
        <td class="py-1 text-right whitespace-nowrap text-xs text-stone-400">
          {#if !isSelf(v) && editing !== v.id}
            {#if kind !== "temae" || v.origin === "user"}
              <button class="px-1 hover:text-ink" onclick={() => move(v, -1)}>↑</button>
              <button class="px-1 hover:text-ink" onclick={() => move(v, 1)}>↓</button>
            {/if}
            <button class="px-1 hover:text-ink" onclick={() => startEdit(v)}>{t("common.edit")}</button>
            <button class="px-1 hover:text-ink" onclick={() => toggleArchive(v)}>{v.archived ? "↩" : t("lists.archive")}</button>
            {#if v.origin === "user"}<button class="px-1 hover:text-red-600" onclick={() => remove(v)}>{t("common.delete")}</button>{/if}
          {/if}
        </td>
      </tr>
    {/each}
  </tbody>
</table>
{/if}
