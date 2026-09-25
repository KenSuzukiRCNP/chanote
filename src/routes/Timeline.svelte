<script lang="ts">
  // Monthly activity as stacked bars (ro / furo / other), with hover tooltip,
  // legend, and a table view. Pure SVG, no library.
  import { t } from "$lib/i18n.svelte";
  import type { MonthlyRow } from "$lib/types";

  let { rows, includeOther, sessionDates }: { rows: MonthlyRow[]; includeOther: boolean; sessionDates: string[] } = $props();

  const SERIES = ["ro", "furo", "other"] as const;
  // validated categorical slots 1-3 (dataviz reference palette)
  const COLOR: Record<(typeof SERIES)[number], string> = { ro: "#2a78d6", furo: "#eb6834", other: "#1baf7a" };

  type Bucket = { key: string; ro: number; furo: number; other: number; sessions: number };
  let granularity = $state<"month" | "year">("month");
  let showTable = $state(false);
  let hover = $state<{ b: Bucket; x: number; y: number } | null>(null);

  const buckets: Bucket[] = $derived.by(() => {
    const m = new Map<string, Bucket>();
    for (const r of rows) {
      if (r.role === "absent") continue;
      if (r.role === "other" && !includeOther) continue;
      const key = granularity === "month" ? r.month : r.month.slice(0, 4);
      const b = m.get(key) ?? { key, ro: 0, furo: 0, other: 0, sessions: 0 };
      b[r.hearth] += r.n_temae;
      m.set(key, b);
    }
    // fill gaps between first and last month so the axis is continuous
    const keys = [...m.keys()].sort();
    if (keys.length && granularity === "month") {
      const [y0, m0] = keys[0].split("-").map(Number);
      const [y1, m1] = keys[keys.length - 1].split("-").map(Number);
      for (let y = y0, mo = m0; y < y1 || (y === y1 && mo <= m1); mo++) {
        if (mo > 12) { mo = 1; y++; }
        const k = `${y}-${String(mo).padStart(2, "0")}`;
        if (!m.has(k)) m.set(k, { key: k, ro: 0, furo: 0, other: 0, sessions: 0 });
      }
    }
    for (const d of sessionDates) {
      const key = granularity === "month" ? d.slice(0, 7) : d.slice(0, 4);
      const b = m.get(key);
      if (b) b.sessions++;
    }
    return [...m.values()].sort((a, b) => a.key.localeCompare(b.key));
  });

  const W = 860, H = 240, PAD = { l: 32, r: 12, t: 12, b: 28 };
  const innerW = W - PAD.l - PAD.r, innerH = H - PAD.t - PAD.b;
  const maxV = $derived(Math.max(1, ...buckets.map((b) => b.ro + b.furo + b.other)));
  const slot = $derived(innerW / Math.max(1, buckets.length));
  const barW = $derived(Math.max(3, Math.min(28, slot - 2)));
  const yScale = (v: number) => innerH * (v / maxV);
  const ticks = $derived.by(() => {
    const step = maxV <= 5 ? 1 : maxV <= 12 ? 2 : maxV <= 30 ? 5 : 10;
    const out: number[] = [];
    for (let v = 0; v <= maxV; v += step) out.push(v);
    return out;
  });
  function yearLabel(i: number): string | null {
    const k = buckets[i].key;
    if (granularity === "year") return k;
    if (!k.endsWith("-01") && i !== 0) return null;
    return k.slice(0, 4);
  }
  /** top-rounded rect path (4px radius on the data end) */
  function topRounded(x: number, y: number, w: number, h: number, r = 3): string {
    r = Math.min(r, h, w / 2);
    return `M${x},${y + h} V${y + r} Q${x},${y} ${x + r},${y} H${x + w - r} Q${x + w},${y} ${x + w},${y + r} V${y + h} Z`;
  }
</script>

<div class="flex items-center gap-4 mb-2 text-sm">
  <span class="flex gap-1">
    <button class="btn py-0.5" class:bg-stone-200={granularity === "month"} onclick={() => (granularity = "month")}>{t("stats.byMonth")}</button>
    <button class="btn py-0.5" class:bg-stone-200={granularity === "year"} onclick={() => (granularity = "year")}>{t("stats.byYear")}</button>
  </span>
  <span class="flex gap-3 text-xs text-stone-600">
    {#each SERIES as s}
      <span class="flex items-center gap-1"><span class="inline-block w-3 h-3 rounded-sm" style="background:{COLOR[s]}"></span>{t("stats.hearth." + s)}</span>
    {/each}
  </span>
  <span class="grow"></span>
  <label class="text-xs flex items-center gap-1"><input type="checkbox" bind:checked={showTable} />{t("stats.tableView")}</label>
</div>

{#if buckets.length === 0}
  <p class="text-stone-500">{t("stats.noData")}</p>
{:else if showTable}
  <table class="text-sm max-w-xl w-full">
    <thead class="text-xs text-stone-500"><tr class="border-b border-line">
      <th class="text-left py-1 font-normal">{granularity === "month" ? t("stats.month") : t("stats.year")}</th>
      {#each SERIES as s}<th class="text-right py-1 px-2 font-normal">{t("stats.hearth." + s)}</th>{/each}
      <th class="text-right py-1 px-2 font-normal">{t("stats.total")}</th>
      <th class="text-right py-1 px-2 font-normal">{t("stats.sessionsLabel")}</th>
    </tr></thead>
    <tbody>
      {#each buckets as b (b.key)}
        <tr class="border-b border-line/60">
          <td class="py-0.5 font-mono">{b.key}</td>
          {#each SERIES as s}<td class="text-right px-2 tabular-nums">{b[s] || ""}</td>{/each}
          <td class="text-right px-2 tabular-nums font-medium">{b.ro + b.furo + b.other || ""}</td>
          <td class="text-right px-2 tabular-nums text-stone-500">{b.sessions || ""}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{:else}
  <div class="relative max-w-4xl">
    <svg viewBox="0 0 {W} {H}" class="w-full h-auto select-none" role="img" aria-label={t("stats.timeline")}
         onmouseleave={() => (hover = null)}>
      <!-- grid + y axis -->
      {#each ticks as v}
        <line x1={PAD.l} x2={W - PAD.r} y1={PAD.t + innerH - yScale(v)} y2={PAD.t + innerH - yScale(v)} stroke="#e7e2d9" stroke-width="1" />
        <text x={PAD.l - 6} y={PAD.t + innerH - yScale(v) + 3} font-size="10" text-anchor="end" fill="#8a857c">{v}</text>
      {/each}
      <!-- bars -->
      {#each buckets as b, i (b.key)}
        {@const x = PAD.l + i * slot + (slot - barW) / 2}
        {@const total = b.ro + b.furo + b.other}
        {@const segs = SERIES.filter((s) => b[s] > 0)}
        {#each segs as s, si}
          {@const below = segs.slice(0, si).reduce((a, k) => a + b[k], 0)}
          {@const h = yScale(b[s])}
          {@const y = PAD.t + innerH - yScale(below) - h}
          {#if si === segs.length - 1}
            <path d={topRounded(x, y, barW, Math.max(0, h - (si > 0 ? 2 : 0)))} fill={COLOR[s]} />
          {:else}
            <rect {x} y={y} width={barW} height={Math.max(0, h - 2)} fill={COLOR[s]} />
          {/if}
        {/each}
        <!-- hit target wider than the bar -->
        <rect x={PAD.l + i * slot} y={PAD.t} width={slot} height={innerH} fill="transparent" role="presentation"
              onmouseenter={() => (hover = { b, x: x + barW / 2, y: PAD.t + innerH - yScale(total) })} />
        {#if yearLabel(i)}
          <text x={x + barW / 2} y={H - 10} font-size="10" text-anchor="middle" fill="#8a857c">{yearLabel(i)}</text>
          {#if granularity === "month"}<line x1={PAD.l + i * slot} x2={PAD.l + i * slot} y1={PAD.t + innerH} y2={PAD.t + innerH + 4} stroke="#8a857c" />{/if}
        {/if}
      {/each}
      <line x1={PAD.l} x2={W - PAD.r} y1={PAD.t + innerH} y2={PAD.t + innerH} stroke="#c9c2b6" />
      {#if hover}
        <line x1={hover.x} x2={hover.x} y1={PAD.t} y2={PAD.t + innerH} stroke="#8a857c" stroke-dasharray="2 2" />
      {/if}
    </svg>
    {#if hover}
      <div class="absolute pointer-events-none rounded-md border border-line bg-white px-2 py-1 text-xs shadow-sm"
           style="left: calc({(hover.x / W) * 100}% + 8px); top: {Math.max(0, (hover.y / H) * 100 - 8)}%;">
        <div class="font-mono text-stone-500">{hover.b.key}</div>
        {#each SERIES as s}{#if hover.b[s]}<div><span class="inline-block w-2 h-2 rounded-sm mr-1" style="background:{COLOR[s]}"></span>{t("stats.hearth." + s)} {hover.b[s]}</div>{/if}{/each}
        <div class="text-stone-500">{t("stats.sessionsLabel")}: {hover.b.sessions}</div>
      </div>
    {/if}
  </div>
{/if}
