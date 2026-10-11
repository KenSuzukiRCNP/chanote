<script lang="ts">
  // Record editor: the event (date, form, place, teacher, my role, roster) and one card
  // per temae. "Details" toggles the modifier axes, participants, utensils and notes.
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { t, term, axisLabel } from "$lib/i18n.svelte";
  import { active, ranked, get, teachers, vocab, eventKinds } from "$lib/vocab.svelte";
  import { setDraft, takeDraft } from "$lib/draft.svelte";
  import { emptySession, emptyTemae, UNKNOWN, NONE, SESSION_FORMS, MY_ROLES, ROSTER_ROLES } from "$lib/types";
  import type { Session, SessionPerson, Temae, MyRole, SessionRole, TemaePerson } from "$lib/types";
  import TemaeExtras from "./TemaeExtras.svelte";
  import TemaePicker from "./TemaePicker.svelte";
  import PhotoStrip from "./PhotoStrip.svelte";

  let { id }: { id: number | null } = $props();

  let s = $state<Session>(emptySession());
  let loading = $state(true);
  let saving = $state(false);
  let err = $state<string | null>(null);
  let info = $state<string | null>(null);
  let open = $state<Record<number, boolean>>({});
  let showRoster = $state(false);

  onMount(async () => {
    if (id != null) {
      const got = await api.sessionGet(id);
      if (got) s = got;
    } else {
      const draft = takeDraft();
      if (draft) { s = draft; info = t("session.duplicated"); }
      else s.temae = [emptyTemae()];
    }
    showRoster = s.form === "chaji" || s.form === "chakai" || roster().length > 0;
    loading = false;
  });

  function isKagetsu(tm: Temae): boolean {
    const cat = (get(tm.temae_id)?.meta as { category?: string } | null)?.category ?? "";
    return cat === "kagetsu" || cat.endsWith(".kagetsu");
  }

  // people and places: most-recently-used first (self always on top)
  const persons = $derived.by(() => {
    const r = ranked("person");
    const me = r.find((p) => p.id === vocab.selfId);
    return me ? [me, ...r.filter((p) => p.id !== vocab.selfId)] : r;
  });
  const others = $derived(persons.filter((p) => p.id !== vocab.selfId));
  const places = $derived(ranked("place"));
  const tanas = $derived(active("tana"));
  const hearths = $derived(active("hearth"));
  const kinds = $derived(s.form === "chaji" || s.form === "chakai" ? eventKinds(s.form) : []);

  const teacherFromSession = $derived(s.persons.find((p) => p.role === "teacher")?.person_id ?? null);
  /** roster = session persons other than teachers */
  const roster = () => s.persons.filter((p) => p.role !== "teacher");
  const sessionHost = $derived(s.persons.find((p) => p.role === "host")?.person_id ?? null);
  /** what a temae without its own participants inherits (roster minus host) */
  const inherited = $derived<TemaePerson[]>(
    roster().filter((p) => p.role !== "host").map((p) => ({
      person_id: p.person_id, name_free: p.name_free,
      role: p.role === "guest" ? "kyaku" : (p.role as TemaePerson["role"]),
      seat: p.seat, is_last: p.is_last, role_note: p.role_note,
    })),
  );
  const selfIsGuest = $derived(s.my_role === "kyaku" || s.my_role === "shokyaku" || s.my_role === "tsume");

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
    else if (tm.my_role === "teishu" || !tm.my_role) setMyRole(tm, s.my_role && s.my_role !== "teishu" ? s.my_role : "kengaku");
  }

  /** Seat defaults: main guest → 1, other guest → next free seat, host/observer → none. */
  function setMyRole(tm: Temae, role: MyRole | null) {
    tm.my_role = role;
    if (role === "shokyaku") tm.my_seat = 1;
    else if (role === "kyaku" || role === "tsume") { if (tm.my_seat == null) tm.my_seat = nextSeat(tm.persons.length ? tm.persons : inherited, tm.my_seat); }
    else tm.my_seat = null;
  }
  function nextSeat(list: { seat: number | null }[], mine: number | null): number {
    const used = new Set<number>(list.map((p) => p.seat).filter((x): x is number => x != null));
    if (mine != null) used.add(mine);
    let n = 1;
    while (used.has(n)) n++;
    return n;
  }

  /** My role for the whole event; temae rows that follow the event get it too. */
  function setSessionMyRole(role: MyRole | null) {
    const prev = s.my_role;
    s.my_role = role;
    if (role === "shokyaku") s.my_seat = 1;
    else if (role === "kyaku" || role === "tsume") { if (s.my_seat == null) s.my_seat = nextSeat(roster(), s.my_seat); }
    else s.my_seat = null;
    for (const tm of s.temae) {
      if (tm.host_id === vocab.selfId) continue;
      if (tm.my_role == null || tm.my_role === prev) { tm.my_role = role; tm.my_seat = s.my_seat; }
    }
    if (role === "teishu" && !s.persons.some((p) => p.role === "host")) {
      for (const tm of s.temae) if (!tm.host_id) { tm.host_id = vocab.selfId; tm.my_role = "teishu"; tm.my_seat = null; }
    }
  }

  function setForm(form: Session["form"]) {
    s.form = form;
    if (form !== "chaji" && form !== "chakai") s.event_kind = null;
    else showRoster = true;
  }

  function setTeacher(personId: string) {
    s.persons = s.persons.filter((p) => p.role !== "teacher");
    if (personId) s.persons.unshift({ person_id: personId, name_free: null, role: "teacher", seat: null, is_last: false, role_note: null });
  }

  function addRosterPerson() {
    const guests = roster().filter((p) => p.role === "guest");
    s.persons.push({ person_id: null, name_free: null, role: "guest", seat: nextSeat(guests, selfIsGuest ? s.my_seat : null), is_last: false, role_note: null });
  }
  function setRosterRole(p: SessionPerson, role: SessionRole) {
    p.role = role;
    if (role !== "guest") { p.seat = null; p.is_last = false; }
    if (role === "host") {
      for (const tm of s.temae) if (!tm.host_id || tm.host_id === sessionHost) tm.host_id = p.person_id;
    }
  }
  function moveRoster(p: SessionPerson, d: number) {
    const i = s.persons.indexOf(p), j = i + d;
    if (j < 0 || j >= s.persons.length || s.persons[j].role === "teacher") return;
    [s.persons[i], s.persons[j]] = [s.persons[j], s.persons[i]];
  }

  function addTemae() {
    const last = s.temae[s.temae.length - 1];
    const tm = emptyTemae();
    if (last) { tm.hearth = last.hearth; tm.teacher_id = last.teacher_id; }
    // an event has one host and one role for me unless a temae says otherwise
    if (s.my_role === "teishu") { tm.host_id = vocab.selfId; tm.my_role = "teishu"; }
    else {
      tm.host_id = sessionHost ?? last?.host_id ?? null;
      if (tm.host_id !== vocab.selfId) { tm.my_role = s.my_role; tm.my_seat = s.my_seat; }
      else tm.my_role = "teishu";
    }
    s.temae.push(tm);
  }

  async function save() {
    saving = true; err = null;
    try {
      await api.sessionSave($state.snapshot(s));
      location.hash = "#/";
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

  /** New record with everything but the ids and the date. */
  function duplicate() {
    const copy: Session = JSON.parse(JSON.stringify($state.snapshot(s)));
    copy.id = null;
    copy.date = new Date().toISOString().slice(0, 10);
    for (const tm of copy.temae) { tm.id = null; tm.note = null; }
    copy.note = null;
    setDraft(copy);
    location.hash = "#/new";
  }

  const teas = ["koicha", "usucha", "tsuzuki", NONE, UNKNOWN];
  const kattes = ["jun", "gyaku", UNKNOWN];
  const rooms = ["koma", "yojohan", "hiroma", UNKNOWN];
  const kinins = [NONE, "kinindate", "kiyotsugu"];
</script>

{#if loading}
  <p class="text-stone-500">{t("common.loading")}</p>
{:else}
<div class="max-w-3xl flex flex-col gap-4">
  {#if info}<div class="text-sm text-matcha-deep">{info}</div>{/if}
  <div class="card flex flex-col gap-3">
    <div class="grid grid-cols-[auto_1fr_auto_1fr] gap-x-4 gap-y-2 items-center">
      <span class="label">{t("session.date")}</span>
      <span class="flex gap-2">
        <input type="date" class="field" bind:value={s.date} disabled={s.date_prec === "unknown"} />
        <select class="field w-auto" bind:value={s.date_prec}>
          {#each ["day", "month", "year", "unknown"] as p}<option value={p}>{t("session.datePrec." + p)}</option>{/each}
        </select>
      </span>
      <span class="label" title={t("session.formHelp")}>{t("session.form")}</span>
      <span class="flex gap-2 items-center">
        <select class="field w-auto" value={s.form} onchange={(e) => setForm(e.currentTarget.value as Session["form"])} title={t("session.formHelp")}>
          {#each SESSION_FORMS as f}<option value={f}>{t("session.form." + f)}</option>{/each}
        </select>
        {#if s.form === "chaji" || s.form === "chakai"}
          <select class="field" bind:value={s.event_kind}>
            <option value={null}>— {t("session.eventKind")} —</option>
            {#each kinds as k}<option value={k.id}>{term(k)}</option>{/each}
            <option value={UNKNOWN}>{t("value.unknown")}</option>
          </select>
        {/if}
        <label class="text-sm flex items-center gap-1 whitespace-nowrap" title={t("session.practiceHelp")}>
          <input type="checkbox" bind:checked={s.practice} />{t("session.practice")}
        </label>
      </span>

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

      <span class="label">{t("session.myRole")}</span>
      <span class="flex gap-2 items-center">
        <select class="field w-auto" value={s.my_role ?? ""} onchange={(e) => setSessionMyRole((e.currentTarget.value || null) as MyRole | null)}>
          <option value="">—</option>
          {#each MY_ROLES as r}<option value={r}>{t("role." + r)}</option>{/each}
        </select>
        <input type="number" min="1" class="field w-16" placeholder="#" bind:value={s.my_seat} disabled={!selfIsGuest} title={t("temae.mySeat")} />
      </span>
      <span></span>
      <span class="flex justify-end">
        {#if !showRoster}
          <button class="btn py-0.5 text-xs" onclick={() => (showRoster = true)}>＋ {t("session.roster")}</button>
        {/if}
      </span>

      <span class="label">{t("session.note")}</span>
      <textarea class="field col-span-3 resize-y" rows="2" bind:value={s.note}></textarea>
    </div>

    {#if showRoster}
      <div class="pt-2 border-t border-line">
        <div class="flex items-center gap-3 mb-1">
          <span class="label mb-0">{t("session.roster")}</span>
          <span class="text-[11px] text-stone-400">{t("session.rosterHelp")}</span>
          <span class="grow"></span>
          <button class="btn py-0.5 text-xs" onclick={addRosterPerson}>＋ {t("session.addPerson")}</button>
        </div>
        {#each s.persons as p, i (i)}
          {#if p.role !== "teacher"}
            <div class="grid grid-cols-[3rem_1fr_1fr_6rem_auto_1fr_auto] gap-1 items-center mb-1">
              <input type="number" min="1" class="field py-0.5" placeholder="#" bind:value={p.seat} disabled={p.role !== "guest"} title={t("temae.seat")} />
              <select class="field py-0.5" bind:value={p.person_id}>
                <option value={null}>—</option>
                {#each others as v}<option value={v.id}>{term(v)}</option>{/each}
              </select>
              <input class="field py-0.5" placeholder={t("temae.nameFree")} bind:value={p.name_free} />
              <select class="field py-0.5" value={p.role} onchange={(e) => setRosterRole(p, e.currentTarget.value as SessionRole)}>
                {#each ROSTER_ROLES as r}<option value={r}>{t("role." + r)}</option>{/each}
              </select>
              <label class="text-xs flex items-center gap-1 whitespace-nowrap"><input type="checkbox" bind:checked={p.is_last} disabled={p.role !== "guest"} />{t("temae.isLast")}</label>
              <input class="field py-0.5" placeholder={t("temae.roleNote")} bind:value={p.role_note} />
              <span class="flex gap-0.5 text-stone-400">
                <button class="px-1 hover:text-ink" onclick={() => moveRoster(p, -1)} title="↑">↑</button>
                <button class="px-1 hover:text-ink" onclick={() => moveRoster(p, 1)} title="↓">↓</button>
                <button class="px-1 hover:text-red-600" onclick={() => s.persons.splice(i, 1)}>×</button>
              </span>
            </div>
          {/if}
        {/each}
      </div>
    {/if}
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
            {#each MY_ROLES as r}<option value={r}>{t("role." + r)}</option>{/each}
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
          <TemaeExtras temae={tm} isKagetsu={isKagetsu(tm)} {inherited} />
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
  <div class="flex gap-2 justify-end items-center">
    {#if s.id != null}
      <button class="btn text-red-700" onclick={remove}>{t("common.delete")}</button>
      <button class="btn" onclick={duplicate} title={t("session.duplicateHelp")}>{t("session.duplicate")}</button>
    {/if}
    <span class="grow"></span>
    <a href="#/" class="btn">{t("common.cancel")}</a>
    <button class="btn btn-primary" onclick={save} disabled={saving}>{t("common.save")}</button>
  </div>
</div>
{/if}
