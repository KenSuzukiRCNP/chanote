<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { t, term } from "$lib/i18n.svelte";
  import { get } from "$lib/vocab.svelte";
  import { fmtDate } from "$lib/format";
  import type { SessionSummary, Session } from "$lib/types";
  import TemaeLine from "./TemaeLine.svelte";

  let rows = $state<SessionSummary[]>([]);
  let detail = $state<Record<number, Session>>({});
  let photos = $state<Record<number, number>>({});
  let thumbs = $state<Record<number, string>>({});

  onMount(async () => {
    rows = await api.sessionList(200, 0);
    for (const [sid, n] of await api.attachmentCounts()) photos[sid] = n;
    for (const sid of Object.keys(photos).map(Number)) {
      const [first] = await api.attachmentList(sid);
      if (first) thumbs[sid] = (await api.attachmentThumb(first.id)) ?? "";
    }
    // eager-load temae lines for the visible cards (cheap for a personal log)
    for (const r of rows) {
      const s = await api.sessionGet(r.id);
      if (s) detail[r.id] = s;
    }
  });
</script>

<h1 class="text-lg font-semibold mb-4">{t("home.title")}</h1>

{#if rows.length === 0}
  <p class="text-stone-500">{t("home.empty")}</p>
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
          <ul class="mt-2 pl-4 text-sm flex flex-col gap-0.5 grow">
            {#each detail[r.id].temae as tm}
              <li><TemaeLine temae={tm} /></li>
            {/each}
          </ul>
        {/if}
        {#if thumbs[r.id]}
          <img src={thumbs[r.id]} alt="" class="mt-2 w-20 h-14 object-cover rounded border border-line shrink-0" />
        {/if}
      </div>
    </a>
  {/each}
</div>
