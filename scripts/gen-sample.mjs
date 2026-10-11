// Generates samples/sample-data.json: three years of fictional practice records in
// chanote's export format, so anyone can try the app (Settings → Import JSON) and so the
// browser mock (npm run dev) has realistic data. Deterministic (seeded).
//
//   node scripts/gen-sample.mjs

import fs from "node:fs";
import path from "node:path";
import { parse } from "yaml";

const root = path.resolve(new URL(".", import.meta.url).pathname, "..");
const pack = parse(fs.readFileSync(path.join(root, "packs/urasenke.yaml"), "utf8"));

// ---- deterministic PRNG -----------------------------------------------------
let seed = 20260901;
const rand = () => ((seed = (seed * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff);
const pick = (a) => a[Math.floor(rand() * a.length)];
const chance = (p) => rand() < p;

// ---- pack vocab (same flattening as crates/chanote-core/src/pack.rs) ----------
const origin = `pack:${pack.id}`;
const vocab = [];
const mk = (id, kind, x, parent, sort, meta) => vocab.push({
  id, kind, ja: x.ja, romaji: x.romaji ?? null, en: x.en ?? null, parent_id: parent ?? null,
  sort, origin, archived: false, meta: meta ?? null,
});
mk(`school.${pack.id}`, "school", pack.name, null, 0, { version: pack.version });
// ids are pack-scoped (see crates/chanote-core/src/pack.rs)
const P = pack.id;
const tid = (id) => `temae.${P}.${id}`, cid = (id) => `temae-cat.${P}.${id}`, tanaId = (id) => `tana.${P}.${id.replace(/^tana\./, "")}`;
const axes = (a) => a ? { ...a, ...(a.tana && a.tana !== "-" && a.tana !== "?" ? { tana: tanaId(a.tana) } : {}) } : a;
pack.categories.forEach((c, i) => mk(cid(c.id), "temae-cat", c, c.parent ? cid(c.parent) : null, i));
pack.temae.forEach((t, i) => mk(tid(t.id), "temae", t, t.category ? cid(t.category) : null, i, {
  ...(t.category ? { category: t.category } : {}),
  ...(t.defaults ? { defaults: axes(t.defaults) } : {}),
  ...(t.alias_of ? { alias_of: tid(t.alias_of) } : {}),
  ...(t.modifiers ? { modifiers: axes(t.modifiers) } : {}),
}));
pack.tana.forEach((x, i) => mk(tanaId(x.id), "tana", x, x.parent ? tanaId(x.parent) : null, i));
pack.hearth.forEach((x, i) => mk(`hearth.${x.id}`, "hearth", x, null, i));
(pack.tool_categories ?? []).forEach((x, i) => mk(`tool-cat.${x.id}`, "tool-cat", x, null, i));
(pack.event_kinds ?? []).forEach((x, i) => mk(`event.${P}.${x.id}`, "event-kind", x, null, i, { form: x.form }));
const evId = (id) => `event.${P}.${id}`;

// ---- fictional people, places, tools -------------------------------------------
const user = (id, kind, ja, romaji, extra = {}) => vocab.push({
  id, kind, ja, romaji, en: null, parent_id: extra.parent ?? null, sort: extra.sort ?? null,
  origin: "user", archived: false, meta: extra.meta ?? null,
});
user("person.self", "person", "自分", "jibun", { sort: 0, meta: { is_self: true } });
user("person.yamada", "person", "山田先生", "Yamada-sensei", { meta: { teacher: true } });
user("person.kimura", "person", "木村先生", "Kimura-sensei", { meta: { teacher: true } });
user("person.sato", "person", "佐藤さん", "Sato-san");
user("person.tanaka", "person", "田中さん", "Tanaka-san");
user("person.anna", "person", "Anna", "Anna");
user("person.marco", "person", "Marco", "Marco");
user("person.suzuki", "person", "鈴木さん", "Suzuki-san");
user("place.shofu-an", "place", "松風庵", "Shofu-an");
user("place.seigetsu-an", "place", "清月庵", "Seigetsu-an");
user("place.kaikan", "place", "文化会館 和室", "Bunka Kaikan");
user("place.home", "place", "自宅", "jitaku");
user("tool.chawan-kuro", "tool", "黒楽茶碗 銘「山里」", "kuro raku chawan", { parent: "tool-cat.chawan", meta: { note: "楽家 十五代 直入。箱書あり。" } });
user("tool.chawan-hagi", "tool", "萩茶碗", "hagi chawan", { parent: "tool-cat.chawan" });
user("tool.natsume", "tool", "利休形 中棗", "rikyu-gata chu-natsume", { parent: "tool-cat.usuki" });
user("tool.chaire-katatsuki", "tool", "肩衝茶入 銘「初霜」", "katatsuki chaire", { parent: "tool-cat.koichaki", meta: { note: "瀬戸 江戸初期。仕覆と組。", paired_with: "tool.shifuku-hatsushimo" } });
user("tool.shifuku-hatsushimo", "tool", "仕覆 二重蔓牡丹唐草緞子", "shifuku", { parent: "tool-cat.shifuku", meta: { paired_with: "tool.chaire-katatsuki" } });
user("tool.chashaku-1", "tool", "茶杓 銘「若草」", "chashaku wakakusa", { parent: "tool-cat.chashaku" });
user("tool.mizusashi-1", "tool", "染付 水指", "sometsuke mizusashi", { parent: "tool-cat.mizusashi" });

// ---- sessions --------------------------------------------------------------------
const temaeIds = pack.temae.map((t) => t.id);
const byCat = (c) => pack.temae.filter((t) => t.category === c).map((t) => t.id);
const hira = byCat("hira"), konarai = [...byCat("konarai.mae8"), ...byCat("konarai.ato8")];
const shikaden = byCat("shikaden"), okuden = byCat("okuden"), kagetsu = byCat("kagetsu");
const chabako = byCat("chabako"), ouyou = byCat("ouyou"), sumi = byCat("sumi");
const seasonHearth = (m) => (m >= 5 && m <= 10 ? "furo" : "ro");
const resolve = (id) => {
  const t = pack.temae.find((x) => x.id === id);
  const main = t.alias_of ? pack.temae.find((x) => x.id === t.alias_of) : t;
  const ax = axes({ ...(main.defaults ?? {}), ...(t.modifiers ?? {}) });
  return { temae_id: tid(main.id), temae_alias_id: t.alias_of ? tid(t.id) : null, axes: ax };
};
const REMARKS = [
  "柄杓を置くとき手首が上がる。肘から動かすこと。",
  "茶巾のたたみ直しが雑。角を揃えて。",
  "茶杓を清めるあと、帛紗の扱いを丁寧に。",
  "水指の蓋を開けるとき音を立てない。",
  "濃茶の練りが固い。湯を少し多めに。",
  "居ずまいを直すタイミングが遅い。",
  "建水を引くとき膝行が浅い。",
  "茶碗を出す位置が近すぎる。畳の縁から十六目。",
  "拝見に出すときの向きを確認。",
  "帛紗さばきの四方さばきをもう一度復習。",
];
const tanas = ["marujoku", "kuwa-kojoku", "koko-dana", "yoshino-dana", "nagaita", "tabi-dansu"].map(tanaId);
const students = ["person.sato", "person.tanaka", "person.anna", "person.marco", "person.suzuki"];

function temaeRow(id, date, host, mine) {
  const r = resolve(id);
  const m = Number(date.slice(5, 7));
  const hearth = r.axes.hearth ?? seasonHearth(m);
  let tana = r.axes.tana ?? (chance(0.6) ? pick(tanas) : "-");
  const tea = r.axes.tea ?? (chance(0.5) ? "koicha" : "usucha");
  const row = {
    id: null, time: null, temae_id: r.temae_id, temae_alias_id: r.temae_alias_id, temae_free: null,
    host_id: host, host_free: null, teacher_id: null,
    my_role: mine ? "teishu" : pick(["kyaku", "kyaku", "shokyaku", "kengaku", "hanto"]),
    my_role_note: null, my_seat: null,
    hearth, katte: chance(0.08) ? "gyaku" : "jun", room: chance(0.3) ? pick(["hiroma", "yojohan", "koma"]) : null,
    tea, tana_id: tana, tana_free: null, kinin: r.axes.kinin ?? null, note: null,
    persons: [], tsukimono: r.axes.tsukimono ?? [], tools: [],
  };
  if (row.my_role === "shokyaku") row.my_seat = 1;
  else if (row.my_role === "kyaku") row.my_seat = pick([2, 3]);
  if (mine && chance(0.5)) {
    const guests = students.filter((s) => s !== host).slice(0, 2 + Math.floor(rand() * 2));
    row.persons = guests.map((p, i) => ({ person_id: p, name_free: null, role: "kyaku", seat: i + 1, is_last: i === guests.length - 1, role_note: null }));
  }
  if (mine && tea === "koicha" && chance(0.4)) {
    row.tools.push({ tool_id: "tool.chaire-katatsuki", name_free: null, external_ref: null });
    row.tools.push({ tool_id: "tool.shifuku-hatsushimo", name_free: null, external_ref: null });
    row.tools.push({ tool_id: pick(["tool.chawan-kuro", "tool.chawan-hagi"]), name_free: null, external_ref: null });
  }
  if (r.temae_id === tid("kagetsu") && mine) row.my_role_note = pick(["月", "花", "一", "二", "三"]);
  if (mine && chance(0.35)) row.note = pick(REMARKS);
  return row;
}

const sp = (person_id, role) => ({ person_id, name_free: null, role, seat: null, is_last: false, role_note: null });
const sessions = [];
const start = new Date(2023, 8, 2); // Sep 2023
const end = new Date(2026, 8, 20);
for (let d = new Date(start); d <= end; d.setDate(d.getDate() + 1)) {
  const dow = d.getDay();
  // regular practice on Saturdays (~85%), sometimes Wednesday evening; a workshop every ~2 months
  const isSat = dow === 6 && chance(0.85);
  const isWed = dow === 3 && chance(0.25);
  if (!isSat && !isWed) continue;
  const date = d.toISOString().slice(0, 10);
  const m = d.getMonth() + 1;
  const workshop = isSat && chance(0.12);
  const online = isWed && chance(0.5);
  const place = isWed ? "place.seigetsu-an" : workshop ? "place.kaikan" : "place.shofu-an";
  const teacher = workshop ? "person.kimura" : chance(0.85) ? "person.yamada" : "person.kimura";
  const pool = workshop
    ? [...okuden, ...shikaden, ...kagetsu]
    : m === 8 ? [...chabako, ...hira, ...ouyou]
    : [...hira, ...hira, ...konarai, ...konarai, ...shikaden, ...kagetsu, ...ouyou, ...sumi];
  const temae = [];
  const n = workshop ? 3 + Math.floor(rand() * 2) : 1 + Math.floor(rand() * 3);
  for (let i = 0; i < n; i++) {
    const mine = i === 0 ? chance(0.8) : chance(0.35);
    const host = mine ? "person.self" : pick(students);
    let id = pick(pool);
    if ((id === "shozumi" || id === "gozumi") && m >= 5 && m <= 10 && chance(0.5)) id = pick(hira);
    temae.push(temaeRow(id, date, host, mine));
  }
  sessions.push({
    id: null, date, date_prec: "day", place_id: place, place_note: null,
    form: workshop ? "koshukai" : "temae", event_kind: null, practice: true,
    my_role: null, my_seat: null, online, note: workshop ? "講習会" : null,
    persons: [sp(teacher, "teacher")],
    temae,
  });
}

// ---- self practice at home (no teacher), a few times a year ------------------------
for (const date of ["2023-11-23", "2024-02-11", "2024-05-03", "2024-09-16", "2025-01-13", "2025-04-29", "2025-08-11", "2025-11-03", "2026-02-11", "2026-05-04", "2026-08-11"]) {
  const row = temaeRow(pick([...hira, ...konarai]), date, "person.self", true);
  row.persons = []; row.tools = []; row.note = "自主稽古。";
  sessions.push({
    id: null, date, date_prec: "day", place_id: "place.home", place_note: null,
    form: "temae", event_kind: null, practice: true, my_role: "teishu", my_seat: null, online: false,
    note: null, persons: [], temae: [row],
  });
}

// ---- practice chaji (稽古茶事) with the teacher, roughly three a year ----------------
const CHAJI_KINDS = ["shogo", "shogo", "shogo", "yobanashi", "asa", "hango", "kuchikiri"];
function chaji(date, kind, practice, host, myRole, mySeat, place, note) {
  const m = Number(date.slice(5, 7));
  const guests = students.filter((p) => p !== host).slice(0, 3);
  const persons = [];
  if (practice) persons.push(sp("person.yamada", "teacher"));
  if (host !== "person.self") persons.push(sp(host, "host"));
  // seats: me first if I am main guest, then the others
  let seat = 1;
  if (mySeat === 1) seat = 2;
  guests.forEach((g, i) => {
    const last = i === guests.length - 1 && myRole !== "tsume";
    persons.push({ ...sp(g, "guest"), seat: seat === mySeat ? ++seat : seat, is_last: last });
    seat++;
  });
  if (host === "person.self") persons.push(sp(pick(["person.suzuki", "person.tanaka"]), "hanto"));
  const sumi = m >= 5 && m <= 10 ? "shozumi" : "shozumi";
  const seq = kind === "asa" ? [sumi, "hiradate.koicha", "gozumi", "hiradate.usucha"]
            : kind === "hango" ? ["hiradate.koicha", "hiradate.usucha"]
            : [sumi, "hiradate.koicha", "gozumi", "hiradate.usucha"];
  const temae = seq.map((id) => {
    const r = temaeRow(id, date, host, host === "person.self");
    r.persons = []; r.tools = [];
    if (host !== "person.self") { r.my_role = myRole; r.my_seat = mySeat; }
    if (id === "hiradate.koicha" && host === "person.self") {
      r.tools.push({ tool_id: "tool.chaire-katatsuki", name_free: null, external_ref: null });
      r.tools.push({ tool_id: "tool.shifuku-hatsushimo", name_free: null, external_ref: null });
      r.tools.push({ tool_id: "tool.chawan-kuro", name_free: null, external_ref: null });
    }
    if (practice && host === "person.self" && chance(0.5)) r.note = pick(REMARKS);
    return r;
  });
  return {
    id: null, date, date_prec: "day", place_id: place, place_note: null,
    form: "chaji", event_kind: evId(kind), practice, my_role: myRole, my_seat: mySeat, online: false,
    note, persons, temae,
  };
}
const practiceChaji = [
  ["2023-12-09", "shogo", "person.self", "teishu", null],
  ["2024-03-23", "shogo", "person.sato", "shokyaku", 1],
  ["2024-07-13", "asa", "person.anna", "kyaku", 2],
  ["2024-11-16", "kuchikiri", "person.self", "teishu", null],
  ["2025-02-22", "yobanashi", "person.tanaka", "tsume", 4],
  ["2025-06-14", "shogo", "person.self", "teishu", null],
  ["2025-10-11", "hango", "person.marco", "shokyaku", 1],
  ["2026-01-24", "yobanashi", "person.self", "teishu", null],
  ["2026-05-16", "asa", "person.sato", "kyaku", 3],
  ["2026-09-12", "shogo", "person.anna", "mizuya", null],
];
for (const [date, kind, host, role, seat] of practiceChaji) {
  sessions.push(chaji(date, kind, true, host, role, seat, "place.shofu-an", "稽古茶事。"));
}
// real chaji: invited twice, hosted once at home
sessions.push(chaji("2024-10-26", "shogo", false, "person.sato", "shokyaku", 1, "place.seigetsu-an", "佐藤さんの正午の茶事に正客で招かれる。"));
sessions.push(chaji("2025-12-06", "yobanashi", false, "person.anna", "kyaku", 2, "place.seigetsu-an", null));
sessions.push(chaji("2026-06-20", "shogo", false, "person.self", "teishu", null, "place.home", "自宅で初めての茶事。"));

// ---- chakai (real gatherings): hatsugama every January, a spring gathering, tea for guests at home
for (const [date, kind, place, role, seat, note] of [
  ["2024-01-14", "hatsugama", "place.shofu-an", "kyaku", 3, "初釜。"],
  ["2025-01-12", "hatsugama", "place.shofu-an", "hanto", null, "初釜。半東を務める。"],
  ["2026-01-11", "hatsugama", "place.shofu-an", "teishu", null, "初釜。濃茶席の点前。"],
  ["2024-04-14", "oyose", "place.kaikan", "mizuya", null, "春の大寄せ。水屋。"],
  ["2025-11-02", "robiraki", "place.shofu-an", "kyaku", 2, "炉開き。"],
  ["2025-05-18", null, "place.home", "teishu", null, "友人に一服。"],
  ["2026-03-15", null, "place.home", "teishu", null, "両親に一服。"],
]) {
  const mine = role === "teishu";
  const id = kind === "hatsugama" && mine ? "hiradate.koicha" : "hiradate.usucha";
  const row = temaeRow(id, date, mine ? "person.self" : "person.yamada", mine);
  row.persons = []; row.note = null;
  if (!mine) { row.my_role = role; row.my_seat = seat; }
  const persons = [];
  if (place !== "place.home") persons.push(sp("person.yamada", "teacher"));
  if (place === "place.home") persons.push({ ...sp(null, "guest"), name_free: pick(["友人2名", "両親"]), seat: 1 });
  else if (mine) persons.push(...students.slice(0, 3).map((g, i) => ({ ...sp(g, "guest"), seat: i + 1, is_last: i === 2 })));
  sessions.push({
    id: null, date, date_prec: "day", place_id: place, place_note: null,
    form: "chakai", event_kind: kind ? evId(kind) : null, practice: false, my_role: role, my_seat: seat, online: false,
    note, persons, temae: [row],
  });
}
// a vaguely-remembered old record
sessions.push({
  id: null, date: "2023-06-01", date_prec: "month", place_id: "place.shofu-an", place_note: null,
  form: "temae", event_kind: null, practice: true, my_role: null, my_seat: null,
  online: false, note: "初めての風炉の稽古。詳細は覚えていない。",
  persons: [sp("person.yamada", "teacher")],
  temae: [{ ...temaeRow("hiradate.usucha", "2023-06-01", "person.self", true), tana_id: "?", hearth: "furo", persons: [], tools: [] }],
});
sessions.sort((a, b) => a.date.localeCompare(b.date));

const doc = { app: "chanote", schema_version: 3, exported_at: "2026-10-11T00:00:00Z", vocab, sessions };
fs.mkdirSync(path.join(root, "samples"), { recursive: true });
fs.writeFileSync(path.join(root, "samples/sample-data.json"), JSON.stringify(doc, null, 1) + "\n");
console.log(`sessions: ${sessions.length}, temae: ${sessions.reduce((a, s) => a + s.temae.length, 0)}, vocab: ${vocab.length}`);
