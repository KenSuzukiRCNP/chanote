<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { i18n, t, setLang, type UiLang, type TermMode } from "$lib/i18n.svelte";
  import { loadVocab, vocab } from "$lib/vocab.svelte";
  import Home from "./routes/Home.svelte";
  import Editor from "./routes/Editor.svelte";
  import Stats from "./routes/Stats.svelte";
  import Lists from "./routes/Lists.svelte";
  import Settings from "./routes/Settings.svelte";
  import ProfilePicker from "./routes/ProfilePicker.svelte";
  import type { ProfileInfo } from "$lib/types";

  // Tiny hash router: #/  #/new  #/edit/12  #/stats  #/lists  #/settings
  let hash = $state(location.hash || "#/");
  const route = $derived.by(() => {
    const p = hash.replace(/^#/, "").split("/").filter(Boolean);
    if (p[0] === "new") return { name: "editor", id: null as number | null };
    if (p[0] === "edit") return { name: "editor", id: Number(p[1]) };
    if (p[0] === "stats") return { name: "stats", id: null };
    if (p[0] === "lists") return { name: "lists", id: null };
    if (p[0] === "settings") return { name: "settings", id: null };
    return { name: "home", id: null };
  });

  let ready = $state(false);
  let error = $state<string | null>(null);
  let profiles = $state<ProfileInfo | null>(null);
  let booted = false;
  let picking = $state(false);
  let switcherOpen = $state(false);

  const currentProfileName = $derived(profiles?.profiles.find((p) => p.id === profiles?.current)?.name ?? "");

  async function pickProfile(id: string) {
    if (profiles && id !== profiles.current) await api.profileSwitch(id);
    picking = false;
    switcherOpen = false;
    ready = false;
    location.hash = "#/";
    await boot();
  }

  onMount(() => {
    const onHash = () => (hash = location.hash || "#/");
    window.addEventListener("hashchange", onHash);
    void boot();
    return () => window.removeEventListener("hashchange", onHash);
  });

  async function boot() {
    try {
      profiles = await api.profileList();
      if (!ready && profiles.ask_on_start && profiles.profiles.length > 1 && !picking && !booted) {
        picking = true;
        booted = true;
        return;
      }
      booted = true;
      const lang = (await api.settingGet("ui_lang")) as UiLang | null;
      if (lang) setLang(lang);
      const mode = (await api.settingGet("term_mode")) as TermMode | null;
      if (mode) i18n.termMode = mode;
      const info = await api.appInfo();
      vocab.selfId = info.self_person_id;
      await loadVocab();
      ready = true;
    } catch (e) {
      error = String(e);
    }
  }

  const nav = [
    { href: "#/", key: "nav.home", match: "home" },
    { href: "#/stats", key: "nav.stats", match: "stats" },
    { href: "#/lists", key: "nav.lists", match: "lists" },
    { href: "#/settings", key: "nav.settings", match: "settings" },
  ];
</script>

{#if picking && profiles}
  <ProfilePicker info={profiles} onpick={pickProfile} />
{:else}
<div class="h-full flex">
  <aside class="w-44 shrink-0 border-r border-line bg-white/60 flex flex-col p-3 gap-1">
    <div class="px-2 py-2 mb-2 font-semibold tracking-wide text-matcha-deep">{t("app.name")}</div>
    <a href="#/new" class="btn btn-primary justify-center mb-3">{t("nav.new")}</a>
    {#each nav as n}
      <a
        href={n.href}
        class="px-2 py-1.5 rounded-md text-sm hover:bg-stone-100"
        class:bg-stone-200={route.name === n.match}
      >{t(n.key)}</a>
    {/each}
    <div class="grow"></div>
    {#if profiles}
      <div class="relative">
        <button class="w-full text-left px-2 py-1.5 rounded-md text-sm hover:bg-stone-100 flex items-center gap-1" onclick={() => (switcherOpen = !switcherOpen)} title={t("profile.switch")}>
          <span class="inline-block w-5 h-5 rounded-full bg-matcha/20 text-matcha-deep text-[11px] text-center leading-5">{currentProfileName.slice(0, 1)}</span>
          <span class="truncate">{currentProfileName}</span>
          <span class="ml-auto text-stone-400 text-xs">{switcherOpen ? "▾" : "▸"}</span>
        </button>
        {#if switcherOpen}
          <div class="absolute bottom-full left-0 right-0 mb-1 card p-1 flex flex-col">
            {#each profiles.profiles as p (p.id)}
              <button class="text-left px-2 py-1 rounded text-sm hover:bg-stone-100" class:font-medium={p.id === profiles.current} onclick={() => pickProfile(p.id)}>{p.name}</button>
            {/each}
            <a href="#/settings" class="text-left px-2 py-1 rounded text-xs text-stone-500 hover:bg-stone-100" onclick={() => (switcherOpen = false)}>{t("profile.manage")}</a>
          </div>
        {/if}
      </div>
    {/if}
    <div class="px-2 text-[10px] text-stone-400">{i18n.lang} · {i18n.termMode}</div>
  </aside>

  <main class="grow overflow-auto p-6">
    {#if error}
      <div class="card border-red-300 text-red-700">{error}</div>
    {:else if !ready}
      <div class="text-stone-500">{t("common.loading")}</div>
    {:else if route.name === "home"}
      <Home />
    {:else if route.name === "editor"}
      {#key route.id}<Editor id={route.id} />{/key}
    {:else if route.name === "stats"}
      <Stats />
    {:else if route.name === "lists"}
      <Lists />
    {:else if route.name === "settings"}
      <Settings />
    {/if}
  </main>
</div>
{/if}
