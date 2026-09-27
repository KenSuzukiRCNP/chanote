<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { pickSavePath, pickOpenPaths } from "$lib/dialogs";
  import { loadVocab } from "$lib/vocab.svelte";
  import { i18n, t, setLang, type UiLang, type TermMode } from "$lib/i18n.svelte";
  import type { AppInfo, ImportReport, ProfileInfo } from "$lib/types";
  import ImportDialog from "./ImportDialog.svelte";

  let info = $state<AppInfo | null>(null);
  let msg = $state<string | null>(null);
  let profiles = $state<ProfileInfo | null>(null);
  let newProfile = $state("");
  let editingProfile = $state<string | null>(null);
  let profileDraft = $state("");
  onMount(async () => { info = await api.appInfo(); profiles = await api.profileList(); });

  async function addProfile() {
    if (!newProfile.trim()) return;
    await api.profileCreate(newProfile.trim());
    newProfile = "";
    profiles = await api.profileList();
  }
  async function renameProfile(id: string) {
    await api.profileRename(id, profileDraft);
    editingProfile = null;
    profiles = await api.profileList();
  }
  async function toggleAsk(v: boolean) {
    await api.profileSetAsk(v);
    profiles = await api.profileList();
  }

  async function changeLang(l: UiLang) {
    setLang(l);
    await api.settingSet("ui_lang", l);
    await api.settingSet("term_mode", i18n.termMode);
  }
  async function changeMode(m: TermMode) {
    i18n.termMode = m;
    await api.settingSet("term_mode", m);
  }

  function stamp() {
    return new Date().toISOString().slice(0, 10);
  }

  async function doExport(format: "json" | "csv") {
    msg = null;
    const path = await pickSavePath(`chanote-${stamp()}.${format}`, format);
    if (!path) return;
    try {
      await api.exportWrite(path, format);
      msg = t("settings.exported", { path });
    } catch (e) {
      msg = String(e);
    }
  }

  let importPath = $state<string | null>(null);
  async function doImport() {
    msg = null;
    const [path] = await pickOpenPaths({ ext: ["json"], title: t("settings.import") });
    if (!path) return;
    importPath = path;
  }
  function importClosed(r: ImportReport | null) {
    importPath = null;
    if (r) msg = t("settings.imported", { s: r.sessions_added, ss: r.sessions_skipped, v: r.vocab_added });
  }
  async function rebuildThumbs() {
    msg = null;
    const [ok, bad] = await api.attachmentRebuildThumbs();
    msg = t("settings.thumbsRebuilt", { ok, bad });
  }

  async function doBackup() {
    msg = null;
    const path = await pickSavePath(`chanote-backup-${stamp()}.zip`, "zip");
    if (!path) return;
    try {
      const n = await api.backupZip(path);
      msg = t("settings.backedUp", { path, n });
    } catch (e) {
      msg = String(e);
    }
  }

  let restoreArmed = $state(false);
  async function doRestore() {
    msg = null;
    const [path] = await pickOpenPaths({ ext: ["zip"], title: t("settings.restore") });
    if (!path) return;
    try {
      await api.backupRestore(path);
      msg = t("settings.restored");
      setTimeout(() => api.appRestart(), 800);
    } catch (e) {
      msg = String(e);
    } finally {
      restoreArmed = false;
    }
  }
</script>

<h1 class="text-lg font-semibold mb-4">{t("settings.title")}</h1>
<div class="card max-w-xl grid grid-cols-[auto_1fr] gap-x-6 gap-y-3 items-center text-sm">
  <span class="label">{t("settings.uiLang")}</span>
  <select class="field w-auto" value={i18n.lang} onchange={(e) => changeLang(e.currentTarget.value as UiLang)}>
    <option value="ja">日本語</option>
    <option value="en">English</option>
  </select>

  <span class="label">{t("settings.termMode")}</span>
  <select class="field w-auto" value={i18n.termMode} onchange={(e) => changeMode(e.currentTarget.value as TermMode)}>
    {#each ["ja", "ja-romaji", "romaji"] as m}<option value={m}>{t("settings.termMode." + m)}</option>{/each}
  </select>
</div>

<h2 class="text-base font-semibold mt-6 mb-2">{t("profile.title")}</h2>
<div class="card max-w-xl flex flex-col gap-2 text-sm">
  <p class="text-xs text-stone-500">{t("profile.help")}</p>
  {#if profiles}
    {#each profiles.profiles as p (p.id)}
      <div class="flex items-center gap-2">
        {#if editingProfile === p.id}
          <input class="field w-48" bind:value={profileDraft} onkeydown={(e) => e.key === "Enter" && renameProfile(p.id)} />
          <button class="btn py-0.5" onclick={() => renameProfile(p.id)}>{t("common.save")}</button>
          <button class="btn py-0.5" onclick={() => (editingProfile = null)}>{t("common.cancel")}</button>
        {:else}
          <span class:font-medium={p.id === profiles.current}>{p.name}</span>
          {#if p.id === profiles.current}<span class="text-[10px] rounded px-1 bg-matcha/15 text-matcha-deep">{t("profile.current")}</span>{/if}
          <button class="text-xs text-stone-400 hover:text-ink" onclick={() => { editingProfile = p.id; profileDraft = p.name; }}>{t("common.edit")}</button>
        {/if}
      </div>
    {/each}
    <div class="flex gap-2">
      <input class="field w-48" placeholder={t("profile.newName")} bind:value={newProfile} onkeydown={(e) => e.key === "Enter" && addProfile()} />
      <button class="btn" onclick={addProfile}>＋ {t("profile.add")}</button>
    </div>
    <label class="flex items-center gap-1 text-sm"><input type="checkbox" checked={profiles.ask_on_start} onchange={(e) => toggleAsk(e.currentTarget.checked)} />{t("profile.askOnStart")}</label>
  {/if}
</div>

<h2 class="text-base font-semibold mt-6 mb-2">{t("settings.data")}</h2>
<div class="card max-w-xl flex flex-col gap-3 text-sm">
  <div class="flex gap-2 flex-wrap">
    <button class="btn" onclick={() => doExport("json")}>{t("settings.exportJson")}</button>
    <button class="btn" onclick={() => doExport("csv")}>{t("settings.exportCsv")}</button>
    <span class="grow"></span>
    <button class="btn" onclick={doImport}>{t("settings.import")}</button>
  </div>
  <p class="text-xs text-stone-500">{t("settings.exportHelp")}</p>
  <p class="text-xs text-stone-500">{t("settings.importHelp")}</p>
  <div class="flex gap-2 flex-wrap items-center pt-2 border-t border-line">
    <button class="btn" onclick={doBackup}>{t("settings.backup")}</button>
    <span class="grow"></span>
    {#if restoreArmed}
      <span class="text-xs text-clay">{t("settings.restoreConfirm")}</span>
      <button class="btn text-red-700" onclick={doRestore}>{t("settings.restoreGo")}</button>
      <button class="btn" onclick={() => (restoreArmed = false)}>{t("common.cancel")}</button>
    {:else}
      <button class="btn" onclick={() => (restoreArmed = true)}>{t("settings.restore")}</button>
    {/if}
  </div>
  <p class="text-xs text-stone-500">{t("settings.backupHelp")}</p>
  <div class="flex gap-2 items-center pt-2 border-t border-line">
    <button class="btn" onclick={rebuildThumbs}>{t("settings.rebuildThumbs")}</button>
    <span class="text-xs text-stone-500">{t("settings.rebuildThumbsHelp")}</span>
  </div>
  {#if msg}<p class="text-xs text-matcha-deep break-all">{msg}</p>{/if}
  {#if info}
    <div class="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1 pt-2 border-t border-line">
      <span class="label">{t("settings.dbPath")}</span><span class="font-mono text-xs break-all">{info.db_path}</span>
      <span class="label">{t("settings.version")}</span><span>{info.version}</span>
      <span class="label">{t("settings.schema")}</span><span>{info.schema_version}</span>
    </div>
  {/if}
</div>

{#if importPath}
  <ImportDialog path={importPath} onclose={importClosed} />
{/if}
