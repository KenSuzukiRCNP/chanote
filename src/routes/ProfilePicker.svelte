<script lang="ts">
  // Startup screen when several profiles exist and "ask on start" is on.
  import { api } from "$lib/api";
  import { t } from "$lib/i18n.svelte";
  import type { ProfileInfo } from "$lib/types";

  let { info, onpick }: { info: ProfileInfo; onpick: (id: string) => void } = $props();
  let newName = $state("");

  async function create() {
    if (!newName.trim()) return;
    const p = await api.profileCreate(newName.trim());
    newName = "";
    onpick(p.id);
  }
</script>

<div class="h-full flex items-center justify-center">
  <div class="card w-96 flex flex-col gap-3">
    <div class="font-semibold text-matcha-deep">{t("app.name")}</div>
    <div class="text-sm text-stone-600">{t("profile.who")}</div>
    <div class="flex flex-col gap-1">
      {#each info.profiles as p (p.id)}
        <button class="btn justify-start text-base py-2" class:bg-stone-100={p.id === info.current} onclick={() => onpick(p.id)}>{p.name}</button>
      {/each}
    </div>
    <div class="flex gap-2 pt-2 border-t border-line">
      <input class="field" placeholder={t("profile.newName")} bind:value={newName} onkeydown={(e) => e.key === "Enter" && create()} />
      <button class="btn" onclick={create}>＋</button>
    </div>
  </div>
</div>
