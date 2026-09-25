<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { t, term } from "$lib/i18n.svelte";
  import { active, get } from "$lib/vocab.svelte";
  import { buildCoverage, buildStale, daysSince, seasonOf, type CategoryBlock, type HearthGroup } from "$lib/stats";
  import type { ByRow, CoverageRow, CrossCell, MonthlyRow, StatsTotals } from "$lib/types";
  import Timeline from "./Timeline.svelte";
  import CrossTable from "./CrossTable.svelte";

  const tabs = ["coverage", "interval", "timeline", "cross", "people"] as const;
  let tab = $state<(typeof tabs)[number]>("coverage");

  // filters
  let period = $state<"all" | "1y" | "3y">("all");
  let placeId = $state<string | null>(null);
  let split = $state(false);          // split by ro / furo
  let seasonOnly = $state(true);      // interval tab
  let showUnseen = $state(true);      // coverage tab

  let rows = $state<CoverageRow[]>([]);
  let totals = $state<StatsTotals | null>(null);
  let monthly = $state<MonthlyRow[]>([]);
  let sessionDates = $state<string[]>([]);
  let loading = $state(true);

  // timeline
  let includeOther = $state(false);
  // cross table
  let rowAxis = $state("temae");
  let colAxis = $state("hearth_group");
  let crossHostOnly = $state(true);
  let cross = $state<CrossCell[]>([]);
  // people / places tab
  const dims = ["teacher", "place", "host", "with"] as const;
  let dim = $state<(typeof dims)[number]>("teacher");
  let by = $state<ByRow[]>([]);
  async function loadBy() {
    by = await api.statsBy(dim, { since, place_id: placeId });
  }
  $effect(() => { void since; void placeId; void dim; void loadBy(); });
  function keyLabel(k: string): string {
    if (k.startsWith("free:")) return k.slice(5);
    return term(get(k)) || k;
  }

  const axes = ["temae", "alias", "hearth_group", "hearth", "tana", "tea", "katte", "room", "kinin", "host", "teacher", "place", "my_role", "year", "session_kind"];

  const since = $derived.by(() => {
    if (period === "all") return null;
    const d = new Date();
    d.setFullYear(d.getFullYear() - (period === "1y" ? 1 : 3));
    return d.toISOString().slice(0, 10);
  });

  async function load() {
    loading = true;
    const filter = { since, place_id: placeId };
    const [r, tt, m, list] = await Promise.all([api.statsCoverage(filter), api.statsTotals(filter), api.statsMonthly(filter), api.sessionList(100000, 0)]);
    rows = r; totals = tt; monthly = m;
    sessionDates = list
      .filter((x) => x.date && (!since || x.date >= since) && (!placeId || x.place_id === placeId))
      .map((x) => x.date as string);
    loading = false;
  }
  async function loadCross() {
    cross = await api.statsCross(rowAxis, colAxis, crossHostOnly, { since, place_id: placeId });
  }
  onMount(load);
  $effect(() => { void since; void placeId; void load(); });
  $effect(() => { void since; void placeId; void rowAxis; void colAxis; void crossHostOnly; void loadCross(); });

  const blocks: CategoryBlock[] = $derived(buildCoverage(rows, active("temae"), active("temae-cat")));
  const stale = $derived(buildStale(blocks, seasonOnly));
  // category summary: per category, how many of its temae I have done as host, total counts, last
  const categorySummary = $derived(blocks.map((b) => {
    const done = b.rows.filter((r) => r.host.n > 0).length;
    const nHost = b.rows.reduce((a, r) => a + r.host.n, 0);
    const nOther = b.rows.reduce((a, r) => a + r.other.n, 0);
    const nRecent = b.rows.reduce((a, r) => a + r.host.nRecent, 0);
    const last = b.rows.map((r) => r.host.last).filter(Boolean).sort().at(-1) ?? null;
    return { b, done, total: b.rows.length, nHost, nOther, nRecent, last };
  }));
  let showCategories = $state(true);

  const season: HearthGroup = seasonOf();
  const groups: HearthGroup[] = ["ro", "furo", "other"];

  function fmtDays(d: number | null) {
    if (d == null) return "";
    if (d < 31) return t("stats.days", { n: d });
    if (d < 365) return t("stats.months", { n: Math.floor(d / 30.4) });
    return t("stats.years", { n: (d / 365.25).toFixed(1) });
  }
  function aliasNote(a: Record<string, number>) {
    return Object.entries(a).map(([id, n]) => `${term(get(id))} ×${n}`).join("、");
  }
</script>

<h1 class="text-lg font-semibold mb-3">{t("stats.title")}</h1>

{#if totals}
  <div class="flex gap-6 text-sm mb-4 text-stone-600">
    <span><b class="text-ink text-lg">{totals.sessions}</b> {t("stats.sessions")}</span>
    <span><b class="text-ink text-lg">{totals.temae_as_host}</b> {t("stats.asHost")}</span>
    <span><b class="text-ink text-lg">{totals.temae_as_other}</b> {t("stats.asOther")}</span>
    {#if totals.first_date}<span class="text-stone-400">{totals.first_date} – {totals.last_date}</span>{/if}
  </div>
{/if}

<div class="flex gap-1 mb-3 items-center flex-wrap">
  {#each tabs as x}
    <button class="btn" class:bg-stone-200={tab === x} onclick={() => (tab = x)}>{t("stats." + x)}</button>
  {/each}
  <span class="grow"></span>
  <select class="field w-auto" bind:value={period}>
    <option value="all">{t("stats.period.all")}</option>
    <option value="1y">{t("stats.period.1y")}</option>
    <option value="3y">{t("stats.period.3y")}</option>
  </select>
  <select class="field w-auto" bind:value={placeId}>
    <option value={null}>{t("stats.allPlaces")}</option>
    {#each active("place") as p}<option value={p.id}>{term(p)}</option>{/each}
  </select>
</div>

{#if loading}
  <p class="text-stone-500">{t("common.loading")}</p>

{:else if tab === "coverage"}
  <div class="flex gap-4 mb-2 text-sm">
    <label class="flex items-center gap-1"><input type="checkbox" bind:checked={showCategories} />{t("stats.byCategory")}</label>
    <label class="flex items-center gap-1"><input type="checkbox" bind:checked={split} />{t("stats.splitHearth")}</label>
    <label class="flex items-center gap-1"><input type="checkbox" bind:checked={showUnseen} />{t("stats.showUnseen")}</label>
  </div>
  {#if showCategories}
    <table class="text-sm max-w-3xl w-full mb-6">
      <thead class="text-xs text-stone-500"><tr class="border-b border-line">
        <th class="text-left py-1 font-normal">{t("stats.category")}</th>
        <th class="text-right py-1 px-2 font-normal">{t("stats.covered")}</th>
        <th class="text-left py-1 px-2 font-normal w-40"></th>
        <th class="text-right py-1 px-2 font-normal">{t("stats.asHost")}</th>
        <th class="text-right py-1 px-2 font-normal">{t("stats.recent")}</th>
        <th class="text-right py-1 px-2 font-normal">{t("stats.asOther")}</th>
        <th class="text-right py-1 px-2 font-normal">{t("stats.last")}</th>
      </tr></thead>
      <tbody>
        {#each categorySummary as c (c.b.category?.id ?? "_")}
          <tr class="border-b border-line/60" class:text-stone-400={c.nHost === 0 && c.nOther === 0}>
            <td class="py-1">{c.b.path.map((x) => term(x)).join(" › ") || t("stats.uncategorized")}</td>
            <td class="text-right px-2 tabular-nums">{c.done} / {c.total}</td>
            <td class="px-2">
              <div class="h-2 rounded bg-stone-100 overflow-hidden"><div class="h-full bg-matcha" style="width:{c.total ? (100 * c.done) / c.total : 0}%"></div></div>
            </td>
            <td class="text-right px-2 tabular-nums font-medium">{c.nHost || ""}</td>
            <td class="text-right px-2 tabular-nums text-stone-500">{c.nRecent || ""}</td>
            <td class="text-right px-2 tabular-nums text-stone-400">{c.nOther || ""}</td>
            <td class="text-right px-2 tabular-nums text-stone-500">{c.last ?? ""}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
  <table class="text-sm w-full max-w-4xl">
    <thead class="text-xs text-stone-500">
      <tr class="border-b border-line">
        <th class="text-left py-1 font-normal">{t("temae.temae")}</th>
        {#if split}
          {#each groups as g}
            <th class="text-center py-1 font-normal px-2" colspan="2">{t("stats.hearth." + g)}</th>
          {/each}
        {:else}
          <th class="text-right py-1 font-normal px-2">{t("stats.asHost")}</th>
          <th class="text-right py-1 font-normal px-2">{t("stats.recent")}</th>
          <th class="text-right py-1 font-normal px-2">{t("stats.last")}</th>
          <th class="text-right py-1 font-normal px-2">{t("stats.ago")}</th>
        {/if}
        <th class="text-right py-1 font-normal px-2">{t("stats.asOther")}</th>
      </tr>
    </thead>
    <tbody>
      {#each blocks as b}
        {@const visible = b.rows.filter((r) => showUnseen || r.host.n > 0 || r.other.n > 0)}
        {#if visible.length}
          <tr class="bg-stone-100/70">
            <td colspan={split ? 8 : 6} class="py-1 px-1 text-xs font-medium text-stone-600">
              {b.path.map((c) => term(c)).join(" › ") || t("stats.uncategorized")}
            </td>
          </tr>
          {#each visible as r (r.temae.id)}
            {@const seen = r.host.n > 0 || r.other.n > 0}
            <tr class="border-b border-line/60" class:text-stone-400={!seen}>
              <td class="py-1 px-1">
                <span title={r.temae.en ?? undefined}>{term(r.temae)}</span>
                {#if Object.keys(r.aliases).length}
                  <div class="text-[11px] text-stone-400">{aliasNote(r.aliases)}</div>
                {/if}
              </td>
              {#if split}
                {#each groups as g}
                  {@const h = r.byHearth[g].host}
                  <td class="text-right px-2 tabular-nums" class:font-medium={h.n > 0}>{h.n || ""}</td>
                  <td class="text-right px-1 text-xs text-stone-400 tabular-nums">{h.last ?? ""}</td>
                {/each}
              {:else}
                <td class="text-right px-2 tabular-nums font-medium">{r.host.n || ""}</td>
                <td class="text-right px-2 tabular-nums text-stone-500">{r.host.nRecent || ""}</td>
                <td class="text-right px-2 tabular-nums text-stone-500">{r.host.last ?? ""}</td>
                <td class="text-right px-2 tabular-nums text-stone-500">{fmtDays(daysSince(r.host.last))}</td>
              {/if}
              <td class="text-right px-2 tabular-nums text-stone-400">{r.other.n || ""}</td>
            </tr>
          {/each}
        {/if}
      {/each}
    </tbody>
  </table>

{:else if tab === "interval"}
  <div class="flex gap-4 mb-2 text-sm items-center">
    <label class="flex items-center gap-1">
      <input type="checkbox" bind:checked={seasonOnly} />
      {t("stats.seasonOnly", { season: t("stats.hearth." + season) })}
    </label>
    <span class="text-xs text-stone-400">{t("stats.intervalHelp")}</span>
  </div>
  {#if stale.length === 0}
    <p class="text-stone-500">{t("stats.noData")}</p>
  {:else}
    <table class="text-sm max-w-3xl w-full">
      <thead class="text-xs text-stone-500">
        <tr class="border-b border-line">
          <th class="text-left py-1 font-normal">{t("temae.temae")}</th>
          <th class="text-right py-1 font-normal px-2">{t("stats.last")}</th>
          <th class="text-right py-1 font-normal px-2">{t("stats.ago")}</th>
          <th class="text-right py-1 font-normal px-2">{t("stats.asHost")}</th>
        </tr>
      </thead>
      <tbody>
        {#each stale as r (r.stat.temae.id)}
          <tr class="border-b border-line/60" class:text-clay={r.days != null && r.days > 365}>
            <td class="py-1 px-1">{term(r.stat.temae)}{#if r.n === 1}<span class="ml-1 text-[10px] rounded px-1 bg-stone-100 text-stone-500">{t("stats.once")}</span>{/if}</td>
            <td class="text-right px-2 tabular-nums">{r.last ?? ""}</td>
            <td class="text-right px-2 tabular-nums">{fmtDays(r.days)}</td>
            <td class="text-right px-2 tabular-nums text-stone-500">{r.n}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

{:else if tab === "timeline"}
  <label class="text-sm flex items-center gap-1 mb-2"><input type="checkbox" bind:checked={includeOther} />{t("stats.includeOther")}</label>
  <Timeline rows={monthly} {includeOther} {sessionDates} />

{:else if tab === "cross"}
  <div class="flex gap-3 items-center mb-3 text-sm flex-wrap">
    <label class="flex items-center gap-1">{t("stats.rows")}
      <select class="field w-auto" bind:value={rowAxis}>{#each axes as a}<option value={a}>{t("stats.axis." + a)}</option>{/each}</select></label>
    <label class="flex items-center gap-1">{t("stats.cols")}
      <select class="field w-auto" bind:value={colAxis}>{#each axes as a}<option value={a}>{t("stats.axis." + a)}</option>{/each}</select></label>
    <button class="btn py-0.5" onclick={() => { const r = rowAxis; rowAxis = colAxis; colAxis = r; }} title="swap">⇄</button>
    <label class="flex items-center gap-1"><input type="checkbox" bind:checked={crossHostOnly} />{t("stats.hostOnly")}</label>
  </div>
  <CrossTable cells={cross} {rowAxis} {colAxis} />

{:else if tab === "people"}
  <div class="flex gap-1 mb-3">
    {#each dims as d}
      <button class="btn py-0.5" class:bg-stone-200={dim === d} onclick={() => (dim = d)}>{t("stats.dim." + d)}</button>
    {/each}
  </div>
  <p class="text-xs text-stone-400 mb-2">{t("stats.dimHelp." + dim)}</p>
  {#if by.length === 0}
    <p class="text-stone-500">{t("stats.noData")}</p>
  {:else}
    <table class="text-sm max-w-3xl w-full">
      <thead class="text-xs text-stone-500"><tr class="border-b border-line">
        <th class="text-left py-1 font-normal">{t("stats.dim." + dim)}</th>
        <th class="text-right py-1 px-2 font-normal">{t("stats.sessionsLabel")}</th>
        {#if dim !== "host"}<th class="text-right py-1 px-2 font-normal">{t("stats.asHost")}</th>{/if}
        {#if dim !== "with"}<th class="text-right py-1 px-2 font-normal">{t("stats.asOther")}</th>{/if}
        <th class="text-right py-1 px-2 font-normal">{t("stats.first")}</th>
        <th class="text-right py-1 px-2 font-normal">{t("stats.last")}</th>
      </tr></thead>
      <tbody>
        {#each by as r (r.key)}
          <tr class="border-b border-line/60">
            <td class="py-1">{keyLabel(r.key)}</td>
            <td class="text-right px-2 tabular-nums">{r.n_sessions}</td>
            {#if dim !== "host"}<td class="text-right px-2 tabular-nums font-medium">{r.n_host || ""}</td>{/if}
            {#if dim !== "with"}<td class="text-right px-2 tabular-nums text-stone-400">{r.n_other || ""}</td>{/if}
            <td class="text-right px-2 tabular-nums text-stone-500">{r.first_date ?? ""}</td>
            <td class="text-right px-2 tabular-nums text-stone-500">{r.last_date ?? ""}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
{/if}
