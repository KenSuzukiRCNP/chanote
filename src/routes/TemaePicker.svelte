<script lang="ts">
  // A <select> over the category tree. Categories are selectable ("(すべて)" rows) when
  // `withCategories` is set; nested categories indent. Values: "temae:<id>" / "cat:<id>".
  import { t, term } from "$lib/i18n.svelte";
  import { categoryTree, uncategorisedTemae, type CatNode } from "$lib/vocab.svelte";

  let {
    value = $bindable(""),
    withCategories = false,
    placeholder = "—",
    class: cls = "field",
    onchange,
  }: { value?: string; withCategories?: boolean; placeholder?: string; class?: string; onchange?: (value: string) => void } = $props();

  const tree = $derived(categoryTree());
  const orphans = $derived(uncategorisedTemae());

  type Row = { value: string; label: string; depth: number; isCat: boolean };
  function flatten(nodes: CatNode[], depth: number, out: Row[]) {
    for (const n of nodes) {
      const label = term(n.cat);
      if (withCategories) out.push({ value: "cat:" + n.cat.id, label: `${label}（${t("picker.all")}）`, depth, isCat: true });
      else out.push({ value: "", label, depth, isCat: true });
      for (const v of n.items) out.push({ value: "temae:" + v.id, label: term(v), depth: depth + 1, isCat: false });
      flatten(n.children, depth + 1, out);
    }
  }
  const rows = $derived.by(() => {
    const out: Row[] = [];
    flatten(tree, 0, out);
    if (orphans.length) {
      out.push({ value: "", label: t("stats.uncategorized"), depth: 0, isCat: true });
      for (const v of orphans) out.push({ value: "temae:" + v.id, label: term(v), depth: 1, isCat: false });
    }
    return out;
  });
  const pad = (d: number) => "　".repeat(d);
</script>

<select class={cls} bind:value onchange={() => onchange?.(value)}>
  <option value="">{placeholder}</option>
  {#each rows as r}
    {#if r.isCat && !withCategories}
      <option disabled class="font-medium">{pad(r.depth)}{r.label}</option>
    {:else}
      <option value={r.value} class:font-medium={r.isCat}>{pad(r.depth)}{r.label}</option>
    {/if}
  {/each}
</select>
