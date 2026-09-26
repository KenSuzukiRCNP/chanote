<script lang="ts">
  // Cross table over two axes (temae × tana, temae × hearth, place × teacher, ...).
  import { t, term, axisLabel } from "$lib/i18n.svelte";
  import { vocab } from "$lib/vocab.svelte";
  import type { CrossCell } from "$lib/types";

  let { cells, rowAxis, colAxis }: { cells: CrossCell[]; rowAxis: string; colAxis: string } = $props();

  function label(axis: string, v: string | null): string {
    if (v == null) return t("value.unset");
    if (v === "?") return t("value.unknown");
    switch (axis) {
      case "temae": case "alias": case "tana": case "host": case "teacher": case "place":
        if (v === "-") return axisLabel(axis === "tana" ? "tana" : "x", "-");
        return term(vocab.byId[v]) || v;
      case "hearth": return term(vocab.byId["hearth." + v]) || v;
      case "hearth_group": return t("stats.hearth." + v);
      case "tea": case "katte": case "room": case "kinin": return axisLabel(axis, v);
      case "my_role": return t("role." + v);
      case "session_kind": return t("session.kind." + v);
      default: return v;
    }
  }

  // Sort keys in a natural order: pack/list order for vocabulary axes, fixed order
  // for enum axes, alphabetical otherwise. Empty/unknown go last.
  const FIXED: Record<string, string[]> = {
    hearth_group: ["ro", "furo", "other"],
    hearth: ["ro", "dairo", "tsurigama", "sukigi", "furo", "nakaoki", "binkake", "other"],
    tea: ["koicha", "usucha", "tsuzuki", "-"],
    katte: ["jun", "gyaku"],
    room: ["koma", "yojohan", "hiroma"],
    kinin: ["-", "kinindate", "kiyotsugu"],
    my_role: ["teishu", "shokyaku", "kyaku", "hanto", "mizuya", "kengaku", "-"],
    session_kind: ["keiko", "koshukai", "honban"],
  };
  const VOCAB_KIND: Record<string, string> = { temae: "temae", alias: "temae", tana: "tana", host: "person", teacher: "person", place: "place" };
  function orderKeys(axis: string, keys: string[]): string[] {
    const rank = (k: string): number => {
      if (!k || k === "?") return 1e9;
      if (FIXED[axis]) { const i = FIXED[axis].indexOf(k); return i < 0 ? 1e8 : i; }
      const kind = VOCAB_KIND[axis];
      if (kind) { const i = (vocab.byKind[kind] ?? []).findIndex((v) => v.id === k); return i < 0 ? 1e8 : i; }
      return 0;
    };
    return [...keys].sort((a, b) => rank(a) - rank(b) || label(axis, a || null).localeCompare(label(axis, b || null), "ja"));
  }
  const rowsKeys = $derived(orderKeys(rowAxis, [...new Set(cells.map((c) => c.row ?? ""))]));
  const colKeys = $derived(orderKeys(colAxis, [...new Set(cells.map((c) => c.col ?? ""))]));
  const grid = $derived.by(() => {
    const m = new Map<string, CrossCell>();
    for (const c of cells) m.set(`${c.row ?? ""}|${c.col ?? ""}`, c);
    return m;
  });
  const rowTotals = $derived(Object.fromEntries(rowsKeys.map((r) => [r, cells.filter((c) => (c.row ?? "") === r).reduce((a, c) => a + c.n, 0)])));
  const colTotals = $derived(Object.fromEntries(colKeys.map((k) => [k, cells.filter((c) => (c.col ?? "") === k).reduce((a, c) => a + c.n, 0)])));
  const maxN = $derived(Math.max(1, ...cells.map((c) => c.n)));

  // single-hue sequential fill (blue, light → dark)
  function fill(n: number): string {
    if (!n) return "transparent";
    const a = 0.12 + 0.55 * (n / maxN);
    return `rgba(42,120,214,${a.toFixed(2)})`;
  }
</script>

{#if cells.length === 0}
  <p class="text-stone-500">{t("stats.noData")}</p>
{:else}
  <div class="overflow-x-auto">
    <table class="text-sm">
      <thead class="text-xs text-stone-500">
        <tr class="border-b border-line">
          <th class="text-left py-1 pr-3 font-normal">{t("stats.axis." + rowAxis)} ＼ {t("stats.axis." + colAxis)}</th>
          {#each colKeys as k}<th class="text-center py-1 px-2 font-normal whitespace-nowrap">{label(colAxis, k || null)}</th>{/each}
          <th class="text-right py-1 px-2 font-normal">{t("stats.total")}</th>
        </tr>
      </thead>
      <tbody>
        {#each rowsKeys as r}
          <tr class="border-b border-line/60">
            <td class="py-0.5 pr-3 whitespace-nowrap">{label(rowAxis, r || null)}</td>
            {#each colKeys as k}
              {@const c = grid.get(`${r}|${k}`)}
              <td class="text-center px-2 tabular-nums" style="background:{fill(c?.n ?? 0)}" title={c?.last_date ? `${t("stats.last")}: ${c.last_date}` : ""}>
                {c?.n ?? ""}
              </td>
            {/each}
            <td class="text-right px-2 tabular-nums text-stone-500">{rowTotals[r]}</td>
          </tr>
        {/each}
        <tr class="text-stone-500">
          <td class="py-0.5 pr-3">{t("stats.total")}</td>
          {#each colKeys as k}<td class="text-center px-2 tabular-nums">{colTotals[k]}</td>{/each}
          <td class="text-right px-2 tabular-nums">{cells.reduce((a, c) => a + c.n, 0)}</td>
        </tr>
      </tbody>
    </table>
  </div>
{/if}
