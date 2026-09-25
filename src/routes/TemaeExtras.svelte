<script lang="ts">
  // Participants & seating, utensils, add-ons (付物) for one temae. Loose by design:
  // every field is optional and seats can be partially filled.
  import { t, term } from "$lib/i18n.svelte";
  import { ranked, toolGroups, vocab } from "$lib/vocab.svelte";
  import { TSUKIMONO, type Temae, type MyRole } from "$lib/types";

  let { temae, isKagetsu }: { temae: Temae; isKagetsu: boolean } = $props();

  const persons = $derived(ranked("person").filter((p) => p.id !== vocab.selfId));
  const groups = $derived(toolGroups());
  const partRoles = ["kyaku", "hanto", "mizuya", "kengaku"] as const;
  const myRoles: MyRole[] = ["teishu", "kyaku", "shokyaku", "hanto", "mizuya", "kengaku", "-"];
  const selfIsGuest = $derived(temae.my_role === "kyaku" || temae.my_role === "shokyaku");

  function nextSeat(): number {
    const used = new Set<number>(temae.persons.map((p) => p.seat).filter((x): x is number => x != null));
    if (temae.my_seat != null) used.add(temae.my_seat);
    let n = 1;
    while (used.has(n)) n++;
    return n;
  }
  function addPerson() {
    // guests get the next free seat by default (1 = main guest), the most likely case;
    // if I am a guest without a seat yet, I take the first free one before the newcomer
    if (selfIsGuest && temae.my_seat == null) temae.my_seat = nextSeat();
    temae.persons.push({ person_id: null, name_free: null, role: "kyaku", seat: nextSeat(), is_last: false, role_note: null });
  }
  function setMyRole(role: MyRole | null) {
    temae.my_role = role;
    if (role === "shokyaku") temae.my_seat = 1;
    else if (role === "kyaku") { if (temae.my_seat == null) temae.my_seat = nextSeat(); }
    else temae.my_seat = null;
  }
  function addTool(toolId: string | null = null) {
    temae.tools.push({ tool_id: toolId, name_free: null, external_ref: null });
    // 名物茶入 etc. come with their own 仕覆: add the paired utensil too
    const pair = toolId ? (vocab.byId[toolId]?.meta as { paired_with?: string } | null)?.paired_with : null;
    if (pair && !temae.tools.some((x) => x.tool_id === pair)) {
      temae.tools.push({ tool_id: pair, name_free: null, external_ref: null });
    }
  }
  function toolNote(id: string | null): string | undefined {
    return id ? ((vocab.byId[id]?.meta as { note?: string } | null)?.note ?? undefined) : undefined;
  }
  function toggleTsukimono(k: string) {
    const i = temae.tsukimono.indexOf(k);
    if (i >= 0) temae.tsukimono.splice(i, 1); else temae.tsukimono.push(k);
  }
  function move(i: number, d: number) {
    const j = i + d;
    if (j < 0 || j >= temae.persons.length) return;
    [temae.persons[i], temae.persons[j]] = [temae.persons[j], temae.persons[i]];
  }
</script>

<!-- 付物 -->
<div class="col-span-4 flex flex-wrap gap-x-4 gap-y-1 items-center pt-1">
  <span class="label mb-0">{t("temae.tsukimono")}{#if !isKagetsu}<span class="text-stone-300"> ({t("temae.kagetsuOnly")})</span>{/if}</span>
  {#each TSUKIMONO as k}
    <label class="text-sm flex items-center gap-1" class:opacity-50={!isKagetsu}>
      <input type="checkbox" checked={temae.tsukimono.includes(k)} onchange={() => toggleTsukimono(k)} />
      {t("tsukimono." + k)}
    </label>
  {/each}
</div>

<!-- 参加者・席順 -->
<div class="col-span-4 pt-2 border-t border-line">
  <div class="flex items-center gap-3 mb-1">
    <span class="label mb-0">{t("temae.participants")}</span>
    <span class="text-[11px] text-stone-400">{t("temae.seatHelp")}</span>
    <span class="grow"></span>
    <button class="btn py-0.5 text-xs" onclick={addPerson}>＋ {t("temae.addPerson")}</button>
  </div>
  <!-- self: always the first row, bound to my_role / my_seat -->
  <div class="grid grid-cols-[3rem_1fr_1fr_6rem_auto_1fr_auto] gap-1 items-center mb-1 text-matcha-deep">
    <input type="number" min="1" class="field py-0.5" placeholder="#" bind:value={temae.my_seat} disabled={!selfIsGuest} title={t("temae.mySeat")} />
    <span class="px-2 text-sm font-medium">{t("temae.myRole")}</span>
    <span></span>
    <select class="field py-0.5" value={temae.my_role ?? ""} onchange={(e) => setMyRole((e.currentTarget.value || null) as MyRole | null)}>
      <option value="">—</option>
      {#each myRoles as r}<option value={r}>{t("role." + r)}</option>{/each}
    </select>
    <span></span>
    <input class="field py-0.5" placeholder={t("temae.roleNote")} bind:value={temae.my_role_note} />
    <span></span>
  </div>
  {#each temae.persons as p, i (i)}
    <div class="grid grid-cols-[3rem_1fr_1fr_6rem_auto_1fr_auto] gap-1 items-center mb-1">
      <input type="number" min="1" class="field py-0.5" placeholder="#" bind:value={p.seat} title={t("temae.seat")} />
      <select class="field py-0.5" bind:value={p.person_id}>
        <option value={null}>—</option>
        {#each persons as v}<option value={v.id}>{term(v)}</option>{/each}
      </select>
      <input class="field py-0.5" placeholder={t("temae.nameFree")} bind:value={p.name_free} />
      <select class="field py-0.5" bind:value={p.role}>
        {#each partRoles as r}<option value={r}>{t("role." + r)}</option>{/each}
      </select>
      <label class="text-xs flex items-center gap-1 whitespace-nowrap"><input type="checkbox" bind:checked={p.is_last} />{t("temae.isLast")}</label>
      <input class="field py-0.5" placeholder={t("temae.roleNote")} bind:value={p.role_note} />
      <span class="flex gap-0.5 text-stone-400">
        <button class="px-1 hover:text-ink" onclick={() => move(i, -1)} title="↑">↑</button>
        <button class="px-1 hover:text-ink" onclick={() => move(i, 1)} title="↓">↓</button>
        <button class="px-1 hover:text-red-600" onclick={() => temae.persons.splice(i, 1)}>×</button>
      </span>
    </div>
  {/each}
</div>

<!-- 道具 -->
<div class="col-span-4 pt-2 border-t border-line">
  <div class="flex items-center gap-3 mb-1">
    <span class="label mb-0">{t("temae.tools")}</span>
    <span class="grow"></span>
    <button class="btn py-0.5 text-xs" onclick={() => addTool()}>＋ {t("temae.addTool")}</button>
  </div>
  {#each temae.tools as tool, i (i)}
    <div class="grid grid-cols-[1fr_1fr_8rem_auto] gap-1 items-center mb-1">
      <select class="field py-0.5" value={tool.tool_id ?? ""} title={toolNote(tool.tool_id)}
              onchange={(e) => { const id = e.currentTarget.value || null; tool.tool_id = id; const pair = id ? (vocab.byId[id]?.meta as { paired_with?: string } | null)?.paired_with : null; if (pair && !temae.tools.some((x) => x.tool_id === pair)) temae.tools.push({ tool_id: pair, name_free: null, external_ref: null }); }}>
        <option value="">—</option>
        {#each groups as g}
          <optgroup label={g.cat ? term(g.cat) : t("stats.uncategorized")}>
            {#each g.items as v}<option value={v.id}>{term(v)}</option>{/each}
          </optgroup>
        {/each}
      </select>
      <input class="field py-0.5" placeholder={t("temae.toolFree")} bind:value={tool.name_free} />
      <input class="field py-0.5" placeholder={t("temae.toolRef")} bind:value={tool.external_ref} />
      <button class="px-1 text-stone-400 hover:text-red-600" onclick={() => temae.tools.splice(i, 1)}>×</button>
    </div>
  {/each}
</div>
