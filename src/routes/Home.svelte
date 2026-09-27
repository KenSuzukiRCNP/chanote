<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { t, term } from "$lib/i18n.svelte";
  import { get, ranked, teachers, temaeUnderCategory, toolGroups, vocab } from "$lib/vocab.svelte";
  import { homeFilter, homeFilterActive, resetHomeFilter, toSessionFilter } from "$lib/homeFilter.svelte";
  import { fmtDate } from "$lib/format";
  import type { SessionSummary, Session } from "$lib/types";
  import TemaeLine from "./TemaeLine.svelte";
  import TemaePicker from "./TemaePicker.svelte";

  let rows = $state<SessionSummary[]>([]);
  let detail = $state<Record<number, Session>>({});
  let photos = $state<Record<number, number>>({});
  let thumbs = $state<Record<number, string>>({});
  let showFilters = $state(homeFilterActive());
  let loading = $state(true);
  let personPick = $state("");
  let toolPick = $state("");

  const persons = $derived(ranked("person").filter((p) => p.id !== vocab.selfId));
  const places = $derived(ranked("place"));

  async function load() {
    loading = true;
    const f = toSessionFilter(temaeUnderCategory);
    rows = homeFilterActive() ? await api.sessionSearch(f, 500, 0) : await api.sessionList(500, 0);
    loading = false;
    for (const r of rows) {
      if (!detail[r.id]) {
        const s = await api.sessionGet(r.id);
        if (s) detail[r.id] = s;
      }
    }
  }
  onMount(async () => {
    const show = await api.settingGet("home_show_notes");
    if (show != null) homeFilter.showNotes = show === "1";
    for (const [sid, n] of await api.attachmentCounts()) photos[sid] = n;
    for (const sid of Object.keys(photos).map(Number)) {
      const [first] = await api.attachmentList(sid);
      if (first) thumbs[sid] = (await api.attachmentThumb(first.id)) ?? "";
    }
  });
  // reload whenever the filter changes
  $effect(() => {
    void homeFilter.temaeSel; void homeFilter.myRoleOnly; void homeFilter.place_id; void homeFilter.teacher_id;
    void homeFilter.since; void homeFilter.until; void homeFilter.persons.length; void homeFilter.tools.length; void homeFilter.text;
    void load();
  });
  // filtering by temae is usually done to re-read the notes: switch them on
  $effect(() => {
    if (homeFilter.temaeSel) homeFilter.showNotes = true;
  });
  async function toggleNotes(v: boolean) {
    homeFilter.showNotes = v;
    await api.settingSet("home_show_notes", v ? "1" : "0");
  }
  function addPerson() {
    if (personPick && !homeFilter.persons.includes(personPick)) homeFilter.persons.push(personPick);
    personPick = "";
  }
  function addTool() {
    if (toolPick && !homeFilter.tools.includes(toolPick)) homeFilter.tools.push(toolPick);
    toolPick = "";
  }
  /** temae rows shown for a session: all, or only those matching the temae filter */
  function visibleTemae(s: Session) {
    const f = toSessionFilter(temaeUnderCategory);
    if (!f.temae?.length) return s.temae;
    const set = new Set(f.temae);
    const hit = s.temae.filter((tm) => (tm.temae_id && set.has(tm.temae_id)) || (tm.temae_alias_id && set.has(tm.temae_alias_id)));
    return hit.length ? hit : s.temae;
  }
</script>

<div class="flex items-center gap-3 mb-3">
  <h1 class="text-lg font-semibold">{t("home.title")}</h1>
  <span class="text-sm text-stone-400">{rows.length}</span>
  <span class="grow"></span>
  <label class="text-sm flex items-center gap-1"><input type="checkbox" checked={homeFilter.showNotes} onchange={(e) => toggleNotes(e.currentTarget.checked)} />{t("home.showNotes")}</label>
  <button class="btn" class:bg-stone-200={showFilters} onclick={() => (showFilters = !showFilters)}>
    {t("home.filter")}{#if homeFilterActive()}<span class="ml-1 inline-block w-2 h-2 rounded-full bg-matcha"></span>{/if}
  </button>
</div>

{#if showFilters}
  <div class="card max-w-3xl mb-4 flex flex-col gap-2 text-sm">
    <div class="grid grid-cols-[auto_1fr_auto_auto] gap-2 items-center">
      <span class="label mb-0">{t("temae.temae")}</span>
      <TemaePicker bind:value={homeFilter.temaeSel} withCategories placeholder={t("home.anyTemae")} />
      <label class="flex items-center gap-1 whitespace-nowrap"><input type="checkbox" bind:checked={homeFilter.myRoleOnly} />{t("stats.hostOnly")}</label>
      <span></span>
    </div>
    <div class="grid grid-cols-[auto_1fr_auto_1fr] gap-2 items-center">
      <span class="label mb-0">{t("session.place")}</span>
      <select class="field" bind:value={homeFilter.place_id}>
        <option value={null}>—</option>
        {#each places as p}<option value={p.id}>{term(p)}</option>{/each}
      </select>
      <span class="label mb-0">{t("session.teacher")}</span>
      <select class="field" bind:value={homeFilter.teacher_id}>
        <option value={null}>—</option>
        {#each teachers() as p}<option value={p.id}>{term(p)}</option>{/each}
      </select>
      <span class="label mb-0">{t("home.period")}</span>
      <span class="flex gap-1 items-center">
        <input type="date" class="field" bind:value={homeFilter.since} /> –
        <input type="date" class="field" bind:value={homeFilter.until} />
      </span>
      <span class="label mb-0">{t("home.text")}</span>
      <input class="field" bind:value={homeFilter.text} placeholder={t("home.textPlaceholder")} />
    </div>
    <div class="grid grid-cols-[auto_1fr] gap-2 items-center">
      <span class="label mb-0">{t("home.persons")}</span>
      <span class="flex flex-wrap gap-1 items-center">
        {#each homeFilter.persons as pid (pid)}
          <span class="rounded-full bg-stone-100 px-2 py-0.5 text-xs flex items-center gap-1">{term(get(pid))}<button class="text-stone-400 hover:text-red-600" onclick={() => (homeFilter.persons = homeFilter.persons.filter((x) => x !== pid))}>×</button></span>
        {/each}
        <select class="field w-auto py-0.5" bind:value={personPick} onchange={addPerson}>
          <option value="">＋</option>
          {#each persons as p}<option value={p.id}>{term(p)}</option>{/each}
        </select>
      </span>
      <span class="label mb-0">{t("temae.tools")}</span>
      <span class="flex flex-wrap gap-1 items-center">
        {#each homeFilter.tools as tid (tid)}
          <span class="rounded-full bg-stone-100 px-2 py-0.5 text-xs flex items-center gap-1">{term(get(tid))}<button class="text-stone-400 hover:text-red-600" onclick={() => (homeFilter.tools = homeFilter.tools.filter((x) => x !== tid))}>×</button></span>
        {/each}
        <select class="field w-auto py-0.5" bind:value={toolPick} onchange={addTool}>
          <option value="">＋</option>
          {#each toolGroups() as g}
            <optgroup label={g.cat ? term(g.cat) : t("stats.uncategorized")}>
              {#each g.items as v}<option value={v.id}>{term(v)}</option>{/each}
            </optgroup>
          {/each}
        </select>
      </span>
    </div>
    <div class="flex justify-end">
      <button class="btn py-0.5 text-xs" onclick={resetHomeFilter} disabled={!homeFilterActive()}>{t("home.clearFilter")}</button>
    </div>
  </div>
{/if}

{#if !loading && rows.length === 0}
  <p class="text-stone-500">{homeFilterActive() ? t("home.noMatch") : t("home.empty")}</p>
{/if}

<div class="flex flex-col gap-3 max-w-3xl">
  {#each rows as r (r.id)}
    <a href={"#/edit/" + r.id} class="card block hover:border-matcha transition">
      <div class="flex items-baseline gap-3">
        <span class="font-mono text-sm">{fmtDate(r.date, r.date_prec)}</span>
        <span class="font-medium">{term(get(r.place_id)) || "—"}</span>
        {#if r.online}
          <span class="text-xs rounded px-1.5 py-0.5 bg-sky-100 text-sky-700">{t("session.online")}</span>
        {/if}
        {#if r.kind !== "keiko"}
          <span class="text-xs rounded px-1.5 py-0.5 bg-clay/15 text-clay">{t("session.kind." + r.kind)}</span>
        {/if}
        <span class="grow"></span>
        {#if photos[r.id]}<span class="text-xs text-stone-400" title={t("photo.title")}>▣ {photos[r.id]}</span>{/if}
        <span class="text-xs text-stone-400">{t("home.temaeCount", { n: r.temae_count })}</span>
      </div>
      <div class="flex gap-3">
        {#if detail[r.id]}
          {@const s = detail[r.id]}
          {@const shown = visibleTemae(s)}
          <ul class="mt-2 pl-4 text-sm flex flex-col gap-0.5 grow">
            {#each shown as tm}
              <li>
                <TemaeLine temae={tm} />
                {#if homeFilter.showNotes && tm.note}
                  <div class="text-xs text-stone-500 whitespace-pre-wrap pl-4 border-l-2 border-line ml-1 mt-0.5">{tm.note}</div>
                {/if}
              </li>
            {/each}
            {#if shown.length < s.temae.length}
              <li class="text-xs text-stone-400">{t("home.moreTemae", { n: s.temae.length - shown.length })}</li>
            {/if}
            {#if homeFilter.showNotes && s.note}
              <li class="text-xs text-stone-500 whitespace-pre-wrap mt-1">{s.note}</li>
            {/if}
          </ul>
        {/if}
        {#if thumbs[r.id]}
          <img src={thumbs[r.id]} alt="" class="mt-2 w-20 h-14 object-cover rounded border border-line shrink-0" />
        {/if}
      </div>
    </a>
  {/each}
</div>
