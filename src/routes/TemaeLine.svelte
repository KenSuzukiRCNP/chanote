<script lang="ts">
  // One temae as a compact line, mirroring the bullet style of the paper log:
  //   真之行台子 風炉 佐藤さん 山田先生 [客]
  import { t, term, axisLabel } from "$lib/i18n.svelte";
  import { get, vocab } from "$lib/vocab.svelte";
  import type { Temae } from "$lib/types";

  let { temae }: { temae: Temae } = $props();

  const name = $derived(
    term(get(temae.temae_alias_id) ?? get(temae.temae_id)) || temae.temae_free || "—",
  );
  const host = $derived(
    temae.host_id === vocab.selfId ? t("temae.myRole") : term(get(temae.host_id)) || temae.host_free || "",
  );
</script>

<span class="font-medium">{name}</span>
{#if temae.hearth && temae.hearth !== "?"}
  <span class="text-stone-500">{term(get("hearth." + temae.hearth)) || temae.hearth}</span>
{/if}
{#if temae.tea && temae.tea !== "-" && temae.tea !== "?"}
  <span class="text-stone-500">{axisLabel("tea", temae.tea)}</span>
{/if}
{#if host}
  <span class:text-matcha-deep={temae.host_id === vocab.selfId}>{host}</span>
{/if}
{#if temae.teacher_id}
  <span class="text-stone-500">{term(get(temae.teacher_id))}</span>
{/if}
{#if temae.my_role && temae.my_role !== "teishu"}
  <span class="text-xs rounded px-1 bg-stone-100 text-stone-600">{t("role." + temae.my_role)}{temae.my_role_note ? "・" + temae.my_role_note : ""}</span>
{:else if temae.my_role_note}
  <span class="text-xs rounded px-1 bg-stone-100 text-stone-600">{temae.my_role_note}</span>
{/if}
