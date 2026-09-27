<script lang="ts">
  // Phase 0/1 editor: session header + temae rows. "Details" toggles the modifier axes.
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { t, term, axisLabel } from "$lib/i18n.svelte";
  import { active, ranked, get, teachers, vocab } from "$lib/vocab.svelte";
  import { emptySession, emptyTemae, UNKNOWN, NONE } from "$lib/types";
  import type { Session, Temae, Vocab, MyRole } from "$lib/types";
  import TemaeExtras from "./TemaeExtras.svelte";
  import TemaePicker from "./TemaePicker.svelte";
  import PhotoStrip from "./PhotoStrip.svelte";

  let { id }: { id: number | null } = $props();

  let s = $state<Session>(emptySession());
  let loading = $state(true);
  let saving = $state(false);
  let err = $state<string | null>(null);
  let open = $state<Record<number, boolean>>({});

  onMount(async () => {
    if (id != null) {
      const got = await api.sessionGet(id);
      if (got) s = got;
    } else {
      s.temae = [emptyTemae()];
    }
    loading = false;
  });


  // people and places: most-recently-used first (self always on top)
  function isKagetsu(tm: Temae): boolean {
    const cat = (get(tm.temae_id)?.meta as { category?: string } | null)?.category ?? "";
    return cat === "kagetsu" || cat.endsWith(".kagetsu");
  }

  const persons = $derived.by(() => {
    const r = ranked("person");
    const me = r.find((p) => p.id === vocab.selfId);
    return me ? [me, ...r.filter((p) => p.id !== vocab.selfId)] : r;
  });
  const places = $derived(ranked("place"));
  const tanas = $derived(active("tana"));
  const hearths = $derived(active("hearth"));

  const teacherFromSession = $derived(s.persons.find((p) => p.role === "teacher")?.person_id ?? null);

  async function pickTemae(tm: Temae, chosen: string) {
    if (!chosen) { tm.temae_id = null; tm.temae_alias_id = null; return; }
    const r = await api.resolveTemae(chosen);
    tm.temae_id = r.temae_id;
    tm.temae_alias_id = r.temae_alias_id;
    // prefill axes only where the user hasn't set anything yet
    if (r.axes.tea && !tm.tea) tm.tea = r.axes.tea;
    if (r.axes.tana && !tm.tana_id) tm.tana_id = r.axes.tana;
    if (r.axes.hearth && !tm.hearth) tm.hearth = r.axes.hearth;
    if (r.axes.katte && !tm.katte) tm.katte = r.axes.katte;
    if (r.axes.room && !tm.room) tm.room = r.axes.room;
    if (r.axes.kinin && !tm.kinin) tm.kinin = r.axes.kinin;
    if (!tm.katte) tm.katte = "jun";
    if (r.axes.tsukimono && tm.tsukimono.length === 0) tm.tsukimono = [...r.axes.tsukimono];
    if (!tm.hearth) tm.hearth = seasonHearth(s.date);
  }

  function seasonHearth(date: string | null): string | null {
    if (!date) return null;
    const m = Number(date.slice(5, 7));
    return m >= 5 && m <= 10 ? "furo" : "ro";
  }

  function setHost(tm: Temae, hostId: string) {
    tm.host_id = hostId || null;
    if (hostId === vocab.selfId) setMyRole(tm, "teishu");
    else if (tm.my_role === "teishu" || !tm.my_role) setMyRole(tm, "kengaku");
  }

  /** Seat defaults: main guest → 1, other guest → next free seat, host/observer → none. */
  function setMyRole(tm: Temae, role: MyRole | null) {
    tm.my_role = role;
    if (role === "shokyaku") tm.my_seat = 1;
    else if (role === "kyaku") { if (tm.my_seat == null) tm.my_seat = nextSeat(tm); }
    else tm.my_seat = null;
  }
  function nextSeat(tm: Temae): number {
    const used = new Set<number>(tm.persons.map((p) => p.seat).filter((x): x is number => x != null));
    if (tm.my_seat != null) used.add(tm.my_seat);
    let n = 1;
    while (used.has(n)) n++;
    return n;
  }

  function setTeacher(personId: string) {
    s.persons = s.persons.filter((p) => p.role !== "teacher");
    if (personId) s.persons.unshift({ person_id: personId, name_free: null, role: "teacher" });
  }

  function addTemae() {
    const last = s.temae[s.temae.length - 1];
    const tm = emptyTemae();
    if (last) { tm.hearth = last.hearth; tm.teacher_id = last.teacher_id; }
    s.temae.push(tm);
  }

  async function save() {
    saving = true; err = null;
    try {
      const newId = await api.sessionSave($state.snapshot(s));
      location.hash = "#/";
      void newId;
    } catch (e) {
      err = String(e);
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (s.id == null) return;
    await api.sessionDelete(s.id);
    location.hash = "#/";
  }

  const myRoles: MyRole[] = ["teishu", "kyaku", "shokyaku", "hanto", "mizuya", "kengaku", "-"];
  const teas = ["koicha", "usucha", "tsuzuki", NONE, UNKNOWN];
  const kattes = ["jun", "gyaku", UNKNOWN];
  const rooms = ["koma", "yojohan", "hiroma", UNKNOWN];
  const kinins = [NONE, "kinindate", "kiyotsugu"];
</script>

{#if loading}
  <p class="text-stone-500">{t("common.loading")}</p>
{:else}
<div class="max-w-3xl flex flex-col gap-4">
  <div class="card grid grid-cols-[auto_1fr_auto_1fr] gap-x-4 gap-y-2 items-center">
    <span class="label">{t("session.date")}</span>
    <span class="flex gap-2">
      <input type="date" class="field" bind:value={s.date} disabled={s.date_prec === "unknown"} />
      <select class="field w-auto" bind:value={s.date_prec}>
        {#each ["day", "month", "year", "unknown"] as p}<option value={p}>{t("session.datePrec." + p)}</option>{/each}
      </select>
    </span>
    <span class="label">{t("session.kind.keiko")}/{t("session.kind.honban")}</span>
    <select class="field" bind:value={s.kind}>
      {#each ["keiko", "koshukai", "honban"] as k}<option value={k}>{t("session.kind." + k)}</option>{/each}
    </select>

    <span class="label">{t("session.place")}</span>
    <span class="flex gap-3 items-center">
      <select class="field" bind:value={s.place_id}>
        <option value={null}>—</option>
        {#each places as p}<option value={p.id}>{term(p)}</option>{/each}
      </select>
      <label class="text-sm flex items-center gap-1 whitespace-nowrap"><input type="checkbox" bind:checked={s.online} />{t("session.online")}</label>
    </span>
    <span class="label">{t("session.teacher")}</span>
    <select class="field" value={teacherFromSession ?? ""} onchange={(e) => setTeacher(e.currentTarget.value)}>
      <option value="">—</option>
      {#each teachers() as p}<option value={p.id}>{term(p)}</option>{/each}
    </select>

    <span class="label">{t("session.note")}</span>
    <textarea class="field col-span-3 resize-y" rows="2" bind:value={s.note}></textarea>
  </div>

  {#each s.temae as tm, i (i)}
    <div class="card flex flex-col gap-2">
      <div class="grid grid-cols-[1fr_1fr_auto_auto] gap-2 items-end">
        <label>
          <div class="label">{t("temae.temae")}</div>
          <TemaePicker value={tm.temae_alias_id ?? tm.temae_id ? "temae:" + (tm.temae_alias_id ?? tm.temae_id) : ""}
                       onchange={(v) => pickTemae(tm, v.replace(/^temae:/, ""))} />
        </label>
        <label>
          <div class="label">{t("temae.host")}</div>
          <select class="field" value={tm.host_id ?? ""} onchange={(e) => setHost(tm, e.currentTarget.value)}>
            <option value="">—</option>
            {#each persons as p}<option value={p.id}>{term(p)}</option>{/each}
          </select>
        </label>
        <label>
          <div class="label">{t("temae.myRole")}</div>
          <select class="field" value={tm.my_role ?? ""} onchange={(e) => setMyRole(tm, (e.currentTarget.value || null) as MyRole | null)}>
            <option value="">—</option>
            {#each myRoles as r}<option value={r}>{t("role." + r)}</option>{/each}
          </select>
        </label>
        <button class="btn" onclick={() => (open[i] = !open[i])}>{t("temae.details")} {open[i] ? "▴" : "▾"}</button>
      </div>

      {#if open[i]}
        <div class="grid grid-cols-4 gap-2 pt-2 border-t border-line">
          <label><div class="label">{t("temae.hearth")}</div>
            <select class="field" bind:value={tm.hearth}>
              <option value={null}>—</option>
              {#each hearths as h}<option value={h.id.replace("hearth.", "")}>{term(h)}</option>{/each}
              <option value={UNKNOWN}>{t("value.unknown")}</option>
            </select></label>
          <label><div class="label">{t("temae.tea")}</div>
            <select class="field" bind:value={tm.tea}>
              <option value={null}>—</option>
              {#each teas as x}<option value={x}>{axisLabel("tea", x)}</option>{/each}
            </select></label>
          <label><div class="label">{t("temae.tana")}</div>
            <select class="field" bind:value={tm.tana_id}>
              <option value={null}>—</option>
              <option value={NONE}>{axisLabel("tana", NONE)}</option>
              <option value={UNKNOWN}>{t("value.unknown")}</option>
              {#each tanas as x}<option value={x.id}>{x.parent_id ? "　" : ""}{term(x)}</option>{/each}
            </select></label>
          <label><div class="label">{t("temae.katte")}</div>
            <select class="field" bind:value={tm.katte}>
              <option value={null}>—</option>
              {#each kattes as x}<option value={x}>{axisLabel("katte", x)}</option>{/each}
            </select></label>
          <label><div class="label">{t("temae.room")}</div>
            <select class="field" bind:value={tm.room}>
              <option value={null}>—</option>
              {#each rooms as x}<option value={x}>{axisLabel("room", x)}</option>{/each}
            </select></label>
          <label><div class="label">{t("temae.kinin")}</div>
            <select class="field" bind:value={tm.kinin}>
              <option value={null}>—</option>
              {#each kinins as x}<option value={x}>{axisLabel("kinin", x)}</option>{/each}
            </select></label>
          <label><div class="label">{t("session.teacher")}</div>
            <select class="field" bind:value={tm.teacher_id}>
              <option value={null}>—</option>
              {#each teachers() as p}<option value={p.id}>{term(p)}</option>{/each}
            </select></label>
          <label><div class="label">{t("temae.time")}</div>
            <input type="time" class="field" bind:value={tm.time} /></label>
          <label class="col-span-2"><div class="label">{t("temae.myRole")} memo</div>
            <input class="field" bind:value={tm.my_role_note} placeholder="月 / 花 / 香 …" /></label>
          <label class="col-span-4"><div class="label">{t("temae.note")}</div>
            <textarea class="field min-h-16 resize-y" rows="3" bind:value={tm.note} placeholder={t("temae.notePlaceholder")}></textarea></label>
          <TemaeExtras temae={tm} isKagetsu={isKagetsu(tm)} />
        </div>
      {/if}
      <div class="flex justify-end">
        <button class="text-xs text-stone-400 hover:text-red-600" onclick={() => s.temae.splice(i, 1)}>{t("common.delete")}</button>
      </div>
    </div>
  {/each}

  <button class="btn self-start" onclick={addTemae}>{t("temae.add")}</button>

  <div class="card">
    {#if s.id != null}
      <PhotoStrip sessionId={s.id} />
    {:else}
      <span class="label mb-0">{t("photo.title")}</span>
      <p class="text-xs text-stone-400">{t("photo.saveFirst")}</p>
    {/if}
  </div>

  {#if err}<div class="text-red-700 text-sm">{err}</div>{/if}
  <div class="flex gap-2 justify-end">
    {#if s.id != null}<button class="btn text-red-700" onclick={remove}>{t("common.delete")}</button>{/if}
    <a href="#/" class="btn">{t("common.cancel")}</a>
    <button class="btn btn-primary" onclick={save} disabled={saving}>{t("common.save")}</button>
  </div>
</div>
{/if}
