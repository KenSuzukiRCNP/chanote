<script lang="ts">
  // Settings → Sync: choose the shared folder, create or join a diary there, see status
  // and concurrent-edit conflicts.
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { t, term } from "$lib/i18n.svelte";
  import { pickFolder } from "$lib/dialogs";
  import { platform, isIOS } from "$lib/platform.svelte";
  import { loadVocab, get } from "$lib/vocab.svelte";
  import { sync, refreshSyncInfo, startAutoSync, syncNow } from "$lib/sync.svelte";
  import type { DiaryInfo, SyncConflict, SyncReport } from "$lib/types";

  let { profileName }: { profileName: string } = $props();

  let folder = $state<string | null>(null);
  let diaries = $state<DiaryInfo[] | null>(null);
  let newName = $state("");
  let busy = $state(false);
  let msg = $state<string | null>(null);
  let conflicts = $state<SyncConflict[]>([]);
  let confirmOff = $state(false);

  onMount(async () => {
    await refreshSyncInfo();
    newName = profileName;
    await loadConflicts();
  });
  async function loadConflicts() {
    conflicts = sync.info?.enabled ? await api.syncConflicts() : [];
  }

  function reportText(r: SyncReport): string {
    return t("sync.report", { sent: r.sent, received: r.received, conflicts: r.conflicts }) +
      (r.adopted ? " " + t("sync.adopted", { n: r.adopted }) : "") +
      (r.missing_files ? " " + t("sync.missingFiles", { n: r.missing_files }) : "") +
      (r.pending ? " " + t("sync.pendingN", { n: r.pending }) : "");
  }

  async function choose() {
    msg = null;
    try {
      let path: string | null = null;
      if (!isIOS()) {
        path = await pickFolder(t("sync.chooseFolder"));
        if (!path) return;
      }
      busy = true;
      const [p, list] = await api.syncChooseFolder(path);
      folder = p; diaries = list;
    } catch (e) {
      if (String(e) !== "cancelled") msg = String(e);
    } finally { busy = false; }
  }

  async function setup(id: string | null) {
    busy = true; msg = null;
    try {
      const r = await api.syncSetup(id, id ? null : newName.trim() || profileName);
      msg = t("sync.started") + " " + reportText(r);
      diaries = null;
      await loadVocab();
      sync.tick++;
      await startAutoSync();
      await loadConflicts();
    } catch (e) { msg = String(e); } finally { busy = false; }
  }

  async function now() {
    msg = null;
    const r = await syncNow();
    if (r) msg = reportText(r);
    else if (sync.error) msg = sync.error;
    await loadConflicts();
  }

  async function off() {
    await api.syncDisable();
    confirmOff = false; folder = null; diaries = null;
    await refreshSyncInfo();
    await startAutoSync();
    msg = t("sync.stopped");
  }

  async function resolve(c: SyncConflict, restore: boolean) {
    await api.syncConflictResolve(c.id, restore);
    if (restore) { await loadVocab(); sync.tick++; }
    await loadConflicts();
    await refreshSyncInfo();
  }

  /** One-line description of the version that was not kept. */
  function describe(c: SyncConflict): string {
    const ev = c.lost as Record<string, unknown>;
    if (ev.op === "session_put") {
      const s = ev.session as { date?: string; place_id?: string; note?: string; temae?: { temae_id?: string; temae_alias_id?: string; note?: string }[] };
      const names = (s.temae ?? []).map((x) => term(get(x.temae_alias_id ?? x.temae_id))).filter(Boolean).join(" / ");
      const notes = [s.note, ...(s.temae ?? []).map((x) => x.note)].filter(Boolean).join(" … ");
      return `${s.date ?? ""} ${term(get(s.place_id))} ${names}${notes ? " — " + notes : ""}`;
    }
    if (ev.op === "vocab_put") return String((ev.vocab as { ja?: string }).ja ?? c.key);
    if (String(ev.op).endsWith("_del")) return t("sync.deletedVersion");
    return c.label ?? c.key;
  }
</script>

<div class="card max-w-xl flex flex-col gap-3 text-sm">
  {#if sync.info && !sync.info.supported}
    <p class="text-xs text-stone-500">{t("sync.androidLater")}</p>
  {:else if sync.info?.enabled}
    <div class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 items-baseline">
      <span class="label mb-0">{t("sync.diary")}</span><span class="font-medium">{sync.info.diary_name}</span>
      <span class="label mb-0">{t("sync.folder")}</span><span class="text-xs break-all">{sync.info.folder ?? (isIOS() ? t("sync.iosFolder") : "—")}</span>
      <span class="label mb-0">{t("sync.last")}</span><span class="text-xs">{sync.info.last_sync ?? "—"}{sync.running ? " · " + t("sync.running") : ""}</span>
      {#if sync.info.outbox || sync.info.pending}
        <span class="label mb-0">{t("sync.waiting")}</span>
        <span class="text-xs">{t("sync.outboxN", { n: sync.info.outbox })}{sync.info.pending ? " · " + t("sync.pendingN", { n: sync.info.pending }) : ""}</span>
      {/if}
    </div>
    <div class="flex gap-2 flex-wrap items-center">
      <button class="btn btn-primary" onclick={now} disabled={sync.running}>{t("sync.now")}</button>
      <span class="grow"></span>
      {#if confirmOff}
        <span class="text-xs text-clay">{t("sync.offConfirm")}</span>
        <button class="btn text-red-700" onclick={off}>{t("sync.off")}</button>
        <button class="btn" onclick={() => (confirmOff = false)}>{t("common.cancel")}</button>
      {:else}
        <button class="btn" onclick={() => (confirmOff = true)}>{t("sync.off")}</button>
      {/if}
    </div>
    {#if conflicts.length}
      <div class="pt-2 border-t border-line flex flex-col gap-2">
        <span class="label mb-0">{t("sync.conflicts")}</span>
        <p class="text-xs text-stone-500">{t("sync.conflictsHelp")}</p>
        {#each conflicts as c (c.id)}
          <div class="flex flex-col gap-1 border border-line rounded-md p-2">
            <span class="text-xs text-stone-400">{c.at.slice(0, 16).replace("T", " ")} · {t("sync.kind." + c.kind)}</span>
            <span class="text-sm">{describe(c)}</span>
            <span class="flex gap-2 justify-end">
              <button class="btn py-0.5 text-xs" onclick={() => resolve(c, true)}>{t("sync.restoreThis")}</button>
              <button class="btn py-0.5 text-xs" onclick={() => resolve(c, false)}>{t("sync.keepCurrent")}</button>
            </span>
          </div>
        {/each}
      </div>
    {/if}
  {:else}
    <p class="text-xs text-stone-500">{t("sync.help")}</p>
    {#if isIOS()}<p class="text-xs text-stone-500">{t("sync.iosHelp")}</p>{/if}
    <div class="flex gap-2 items-center flex-wrap">
      <button class="btn btn-primary" onclick={choose} disabled={busy}>{t("sync.chooseFolder")}</button>
      {#if folder}<span class="text-xs break-all text-stone-500">{folder}</span>{/if}
    </div>
    {#if diaries}
      {#if diaries.length}
        <div class="flex flex-col gap-1">
          <span class="label mb-0">{t("sync.found")}</span>
          {#each diaries as d (d.id)}
            <div class="flex items-center gap-2">
              <span class="font-medium">{d.name}</span>
              <span class="text-xs text-stone-400">{t("sync.devicesN", { n: d.devices })} · {d.created_at.slice(0, 10)}</span>
              <span class="grow"></span>
              <button class="btn py-0.5 text-xs" onclick={() => setup(d.id)} disabled={busy}>{t("sync.join")}</button>
            </div>
          {/each}
          <p class="text-xs text-stone-500">{t("sync.joinHelp")}</p>
        </div>
      {/if}
      <div class="flex gap-2 items-center flex-wrap pt-1">
        <input class="field w-48" bind:value={newName} placeholder={t("sync.newName")} />
        <button class="btn" onclick={() => setup(null)} disabled={busy}>{t("sync.create")}</button>
      </div>
      <p class="text-xs text-stone-500">{t("sync.createHelp")}</p>
    {/if}
  {/if}
  {#if busy}<p class="text-xs text-stone-500">{t("sync.working")}</p>{/if}
  {#if msg}<p class="text-xs text-matcha-deep break-all">{msg}</p>{/if}
  {#if platform.mobile && !sync.info?.enabled && sync.info?.supported}
    <p class="text-[11px] text-stone-400">{t("sync.mobileNote")}</p>
  {/if}
</div>
