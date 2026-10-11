// In-memory stand-in for the Rust backend, used only when running `npm run dev`
// in a plain browser. Enough to click through the UI; not a faithful copy.

import type { Attachment, CoverageRow, CrossCell, EventRow, MonthlyRow, Session, SessionFilter, SessionSummary, Vocab } from "./types";
import sample from "../../samples/sample-data.json";

const attachments: Attachment[] = [];
const profiles = { profiles: [{ id: "default", name: "Me" }, { id: "anna", name: "Anna" }], current: "default", ask_on_start: false };
let nextAtt = 1;
const AXES = ["temae", "alias", "hearth", "hearth_group", "tana", "tea", "katte", "room", "kinin", "host", "teacher", "place", "my_role", "year", "form", "event_kind", "practice", "session_my_role"];
function hg(h: string | null) { return h === "ro" || h === "dairo" || h === "tsurigama" || h === "sukigi" ? "ro" : h === "furo" || h === "nakaoki" ? "furo" : "other"; }
function rg(r: string | null) { return r === "teishu" ? "host" : r == null || r === "-" ? "absent" : "other"; }
// 1x1 grey png
const TINY = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkqAcAAIUAgUW0RjgAAAAASUVORK5CYII=";

// Vocabulary and sessions come from the bundled sample dataset (fictional).
const vocab: Vocab[] = structuredClone(sample.vocab as Vocab[]);
const sessions = new Map<number, Session>();
let nextId = 1;
const settings = new Map<string, string>();
for (const s of sample.sessions as unknown as Session[]) {
  const copy = structuredClone(s);
  copy.id = nextId;
  sessions.set(nextId++, copy);
}

/** participants of a temae: its own, or the session roster (minus host / teacher) */
function effPersons(s: Session, t: Session["temae"][number]) {
  if (t.persons.length) return t.persons;
  return s.persons.filter((p) => p.role !== "teacher" && p.role !== "host").map((p) => ({ ...p, role: p.role === "guest" ? "kyaku" : p.role }));
}
const practiceOk = (f: { practice?: boolean | null }, s: Session) => f.practice == null || s.practice === f.practice;

function blank() {
  return { id: null, time: null, temae_id: null, temae_alias_id: null, temae_free: null, host_id: null, host_free: null, teacher_id: null, my_role: null, my_role_note: null, my_seat: null, hearth: null, katte: null, room: null, tea: null, tana_id: null, tana_free: null, kinin: null, note: null, persons: [], tsukimono: [], tools: [] };
}

function resolve(id: string) {
  const c = vocab.find((x) => x.id === id);
  if (!c) throw new Error(`unknown temae ${id}`);
  const meta = (c.meta ?? {}) as Record<string, unknown>;
  if (typeof meta.alias_of === "string") {
    const m = vocab.find((x) => x.id === meta.alias_of);
    const base = ((m?.meta as Record<string, unknown>)?.defaults ?? {}) as object;
    return { temae_id: meta.alias_of, temae_alias_id: id, axes: { ...base, ...(meta.modifiers as object) } };
  }
  return { temae_id: id, temae_alias_id: null, axes: { ...((meta.defaults ?? {}) as object) } };
}

export async function invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  await new Promise((r) => setTimeout(r, 10));
  switch (cmd) {
    case "app_info":
      return { version: "0.4.0-mock", db_path: "(in-memory mock)", schema_version: 4, self_person_id: "person.self", profile: profiles.current } as T;
    case "profile_list":
      return structuredClone(profiles) as T;
    case "profile_create": {
      const name = (args.input as { name: string }).name;
      const p = { id: name.toLowerCase().replace(/[^a-z0-9]+/g, "-") || "user", name };
      profiles.profiles.push(p);
      return p as T;
    }
    case "profile_rename": {
      const p = profiles.profiles.find((x) => x.id === args.id); if (p) p.name = args.name as string;
      return undefined as T;
    }
    case "profile_set_ask":
      profiles.ask_on_start = args.ask as boolean;
      return undefined as T;
    case "profile_switch":
      profiles.current = args.id as string;
      return undefined as T;
    case "vocab_ranking": {
      const score = new Map<string, { n: number; last: string | null; score: number }>();
      const bump = (id: string | null, d: string | null) => {
        if (!id) return;
        const s = score.get(id) ?? { n: 0, last: null, score: 0 };
        s.n++; if (d && (!s.last || d > s.last)) s.last = d;
        s.score += d ? Math.exp(-((Date.now() - Date.parse(d)) / 86400e3) / 365 * Math.LN2) : 0.25;
        score.set(id, s);
      };
      for (const s of sessions.values()) {
        if (args.kind === "place") bump(s.place_id, s.date);
        for (const t of s.temae) {
          if (args.kind === "person") { bump(t.host_id, s.date); bump(t.teacher_id, s.date); for (const p of t.persons) bump(p.person_id, s.date); }
          if (args.kind === "temae") bump(t.temae_alias_id ?? t.temae_id, s.date);
          if (args.kind === "tana") bump(t.tana_id, s.date);
        }
        if (args.kind === "person") for (const p of s.persons) bump(p.person_id, s.date);
      }
      return [...score.entries()].map(([id, v]) => ({ id, score: v.score, n: v.n, last_date: v.last })).sort((a, b) => b.score - a.score) as T;
    }
    case "stats_by": {
      const m = new Map<string, { n_sessions: Set<number>; n_host: number; n_other: number; first: string | null; last: string | null }>();
      const add = (k: string | null, sid: number, d: string | null, host: number, oth: number) => {
        if (!k) return;
        const r = m.get(k) ?? { n_sessions: new Set(), n_host: 0, n_other: 0, first: null, last: null };
        r.n_sessions.add(sid); r.n_host += host; r.n_other += oth;
        if (d) { if (!r.first || d < r.first) r.first = d; if (!r.last || d > r.last) r.last = d; }
        m.set(k, r);
      };
      for (const s of sessions.values()) for (const t of s.temae) {
        if (!practiceOk(args.filter as { practice?: boolean | null }, s)) continue;
        const host = t.my_role === "teishu" ? 1 : 0, oth = t.my_role && t.my_role !== "teishu" && t.my_role !== "-" ? 1 : 0;
        switch (args.dimension) {
          case "teacher": add(t.teacher_id ?? s.persons.find((p) => p.role === "teacher")?.person_id ?? null, s.id!, s.date, host, oth); break;
          case "place": add(s.place_id, s.id!, s.date, host, oth); break;
          case "host": if (t.my_role && t.my_role !== "teishu") add(t.host_id, s.id!, s.date, 0, oth); break;
          case "with": if (t.my_role === "teishu") for (const p of effPersons(s, t)) add(p.person_id ?? (p.name_free ? "free:" + p.name_free : null), s.id!, s.date, 1, 0); break;
        }
      }
      return [...m.entries()].map(([key, r]) => ({ key, n_sessions: r.n_sessions.size, n_host: r.n_host, n_other: r.n_other, first_date: r.first, last_date: r.last })) as T;
    }
    case "setting_get":
      return (settings.get(args.key as string) ?? null) as T;
    case "setting_set":
      settings.set(args.key as string, args.value as string);
      return undefined as T;
    case "vocab_list":
      return vocab.filter((x) => x.kind === args.kind && (args.includeArchived || !x.archived)) as T;
    case "vocab_upsert": {
      const nv = args.vocab as Vocab;
      const i = vocab.findIndex((x) => x.id === nv.id);
      if (i >= 0) vocab[i] = nv; else vocab.push(nv);
      return undefined as T;
    }
    case "vocab_archive": {
      const x = vocab.find((x) => x.id === args.id);
      if (x) x.archived = args.archived as boolean;
      return undefined as T;
    }
    case "vocab_delete": {
      const i = vocab.findIndex((x) => x.id === args.id);
      if (i >= 0 && vocab[i].origin === "user") vocab.splice(i, 1); else throw new Error("pack entries cannot be deleted; archive instead");
      return undefined as T;
    }
    case "vocab_usage":
      return 0 as T;
    case "pack_reset_row": {
      const orig = (sample.vocab as Vocab[]).find((x) => x.id === args.id);
      const i = vocab.findIndex((x) => x.id === args.id);
      if (orig && i >= 0) vocab[i] = structuredClone(orig);
      return !!orig as T;
    }
    case "resolve_temae":
      return resolve(args.id as string) as T;
    case "temae_defaults":
      return resolve(args.id as string).axes as T;
    case "session_list": {
      const out: SessionSummary[] = [...sessions.values()]
        .sort((a, b) => (b.date ?? "").localeCompare(a.date ?? "") || (b.id ?? 0) - (a.id ?? 0))
        .map((s) => ({ id: s.id!, date: s.date, date_prec: s.date_prec, place_id: s.place_id, form: s.form, event_kind: s.event_kind, practice: s.practice, my_role: s.my_role, online: s.online, temae_count: s.temae.length }));
      return out as T;
    }
    case "session_search": {
      const f = args.filter as SessionFilter;
      const has = (s: Session, p: string) =>
        s.persons.some((x) => x.person_id === p) || s.temae.some((t) => t.host_id === p || t.teacher_id === p || t.persons.some((y) => y.person_id === p));
      const out: SessionSummary[] = [...sessions.values()]
        .filter((s) => (!f.since || (s.date ?? "") >= f.since) && (!f.until || (s.date ?? "") <= f.until))
        .filter((s) => !f.place_id || s.place_id === f.place_id)
        .filter((s) => !f.form || s.form === f.form)
        .filter((s) => f.practice == null || s.practice === f.practice)
        .filter((s) => !f.event_kind || s.event_kind === f.event_kind)
        .filter((s) => !f.teacher_id || s.persons.some((p) => p.role === "teacher" && p.person_id === f.teacher_id) || s.temae.some((t) => t.teacher_id === f.teacher_id))
        .filter((s) => !f.temae?.length || s.temae.some((t) => (f.temae!.includes(t.temae_id ?? "") || f.temae!.includes(t.temae_alias_id ?? "")) && (!f.my_role || t.my_role === f.my_role)))
        .filter((s) => f.temae?.length || !f.my_role || s.my_role === f.my_role || s.temae.some((t) => t.my_role === f.my_role))
        .filter((s) => (f.persons ?? []).every((p) => has(s, p)))
        .filter((s) => (f.tools ?? []).every((tl) => s.temae.some((t) => t.tools.some((x) => x.tool_id === tl))))
        .filter((s) => !f.text || (s.note ?? "").includes(f.text) || s.temae.some((t) => (t.note ?? "").includes(f.text!)))
        .sort((a, b) => (b.date ?? "").localeCompare(a.date ?? "") || (b.id ?? 0) - (a.id ?? 0))
        .map((s) => ({ id: s.id!, date: s.date, date_prec: s.date_prec, place_id: s.place_id, form: s.form, event_kind: s.event_kind, practice: s.practice, my_role: s.my_role, online: s.online, temae_count: s.temae.length }));
      return out as T;
    }
    case "session_get":
      return (sessions.get(args.id as number) ?? null) as T;
    case "session_save": {
      const s = structuredClone(args.session as Session);
      const id = s.id ?? nextId++;
      s.id = id;
      sessions.set(id, s);
      return id as T;
    }
    case "session_delete":
      sessions.delete(args.id as number);
      return undefined as T;
    case "stats_coverage": {
      const cube = new Map<string, CoverageRow>();
      for (const s of sessions.values()) for (const t of s.temae) {
        const role = t.my_role === "teishu" ? "host" : (t.my_role == null || t.my_role === "-") ? "absent" : "other";
        const key = [t.temae_id, t.temae_alias_id, t.hearth, role].join("|");
        const row = cube.get(key) ?? { temae_id: t.temae_id, alias_id: t.temae_alias_id, hearth: t.hearth, role, n: 0, n_recent: 0, first_date: null, last_date: null };
        row.n++;
        if (s.date) {
          if (!row.first_date || s.date < row.first_date) row.first_date = s.date;
          if (!row.last_date || s.date > row.last_date) row.last_date = s.date;
          if (Date.now() - Date.parse(s.date) < 365 * 86400e3) row.n_recent++;
        }
        cube.set(key, row);
      }
      return [...cube.values()] as T;
    }
    case "stats_events": {
      const cube = new Map<string, EventRow>();
      for (const s of sessions.values()) {
        if (!practiceOk(args.filter as { practice?: boolean | null }, s)) continue;
        const key = [s.form, s.event_kind, s.practice, s.my_role].join("|");
        const row = cube.get(key) ?? { form: s.form, event_kind: s.event_kind, practice: s.practice, my_role: s.my_role, n: 0, first_date: null, last_date: null };
        row.n++;
        if (s.date) { if (!row.first_date || s.date < row.first_date) row.first_date = s.date; if (!row.last_date || s.date > row.last_date) row.last_date = s.date; }
        cube.set(key, row);
      }
      return [...cube.values()] as T;
    }
    case "stats_totals": {
      const all = [...sessions.values()].filter((s) => practiceOk(args.filter as { practice?: boolean | null }, s));
      const t = all.flatMap((s) => s.temae);
      const dates = all.map((s) => s.date).filter(Boolean) as string[];
      return { sessions: all.length, temae_as_host: t.filter((x) => x.my_role === "teishu").length,
        temae_as_other: t.filter((x) => x.my_role && x.my_role !== "teishu" && x.my_role !== "-").length,
        first_date: dates.sort()[0] ?? null, last_date: dates.sort().at(-1) ?? null } as T;
    }
    case "export_write":
      console.log("mock export", args);
      return 0 as T;
    case "import_read":
      return { vocab_added: 0, vocab_skipped: 12, sessions_added: 0, sessions_skipped: 3 } as T;
    case "import_preview":
      return {
        persons: vocab.filter((v) => v.kind === "person").map((v) => ({ id: v.id, name: v.ja, is_self: !!(v.meta as { is_self?: boolean } | null)?.is_self, n: 5 })),
        rows: [...sessions.values()].slice(0, 6).map((s, i) => ({ index: i, date: s.date, date_prec: s.date_prec, place: vocab.find((v) => v.id === s.place_id)?.ja ?? "", form: s.form, practice: s.practice, summary: s.temae.map((t) => vocab.find((v) => v.id === (t.temae_alias_id ?? t.temae_id))?.ja ?? "").join(" / "), n_temae: s.temae.length, duplicate_of: i % 2 === 0 ? s.id : null })),
        vocab_new: 0, schema_version: 3,
      } as T;
    case "import_apply":
      return { vocab_added: 0, vocab_skipped: 12, sessions_added: (args.options as { include?: number[] }).include?.length ?? 3, sessions_skipped: 3 } as T;
    case "attachment_rebuild_thumbs":
      return [attachments.length, 0] as T;
    case "stats_axes":
      return AXES as T;
    case "stats_monthly": {
      const cube = new Map<string, MonthlyRow>();
      for (const s of sessions.values()) {
        if (!s.date) continue;
        const seen = new Set<string>();
        for (const t of s.temae) {
          const key = [s.date.slice(0, 7), hg(t.hearth), rg(t.my_role)].join("|");
          const row = cube.get(key) ?? { month: s.date.slice(0, 7), hearth: hg(t.hearth), role: rg(t.my_role), n_temae: 0, n_sessions: 0 };
          row.n_temae++;
          if (!seen.has(key)) { row.n_sessions++; seen.add(key); }
          cube.set(key, row);
        }
      }
      return [...cube.values()].sort((a, b) => a.month.localeCompare(b.month)) as T;
    }
    case "stats_cross": {
      const pick = (ax: string, s: Session, t: Session["temae"][number]): string | null => {
        switch (ax) {
          case "temae": return t.temae_id; case "alias": return t.temae_alias_id ?? t.temae_id;
          case "hearth": return t.hearth; case "hearth_group": return hg(t.hearth); case "tana": return t.tana_id;
          case "tea": return t.tea; case "katte": return t.katte; case "room": return t.room; case "kinin": return t.kinin;
          case "host": return t.host_id; case "teacher": return t.teacher_id ?? s.persons.find((p) => p.role === "teacher")?.person_id ?? null;
          case "place": return s.place_id; case "my_role": return t.my_role; case "year": return s.date?.slice(0, 4) ?? null;
          case "form": return s.form; case "event_kind": return s.event_kind; case "practice": return s.practice ? "practice" : "real";
          case "session_my_role": return s.my_role; default: return null;
        }
      };
      const cube = new Map<string, CrossCell>();
      for (const s of sessions.values()) for (const t of s.temae) {
        if (args.hostOnly && t.my_role !== "teishu") continue;
        if (!practiceOk(args.filter as { practice?: boolean | null }, s)) continue;
        const r = pick(args.rowAxis as string, s, t), c = pick(args.colAxis as string, s, t);
        const key = `${r}|${c}`;
        const cell = cube.get(key) ?? { row: r, col: c, n: 0, last_date: null };
        cell.n++;
        if (s.date && (!cell.last_date || s.date > cell.last_date)) cell.last_date = s.date;
        cube.set(key, cell);
      }
      return [...cube.values()] as T;
    }
    case "attachment_add": {
      const out: Attachment[] = [];
      for (const p of args.paths as string[]) {
        const a = { id: nextAtt++, session_id: args.sessionId as number, temae_id: null, path: `attachments/2026/att-${nextAtt}.jpg`, caption: p.split("/").pop() ?? null, created_at: new Date().toISOString() };
        attachments.push(a); out.push(a);
      }
      return out as T;
    }
    case "attachment_list":
      return attachments.filter((a) => a.session_id === args.sessionId) as T;
    case "attachment_thumb":
      return TINY as T;
    case "attachment_caption": {
      const a = attachments.find((x) => x.id === args.id); if (a) a.caption = args.caption as string | null;
      return undefined as T;
    }
    case "attachment_delete": {
      const i = attachments.findIndex((x) => x.id === args.id); if (i >= 0) attachments.splice(i, 1);
      return undefined as T;
    }
    case "attachment_path":
      return null as T;
    case "attachment_counts": {
      const m = new Map<number, number>();
      for (const a of attachments) if (a.session_id != null) m.set(a.session_id, (m.get(a.session_id) ?? 0) + 1);
      return [...m.entries()] as T;
    }
    case "backup_write":
      return undefined as T;
    case "backup_zip":
      return 3 as T;
    case "backup_restore":
      return 3 as T;
    case "app_restart":
      location.reload();
      return undefined as T;
    default:
      throw new Error(`mock: unknown command ${cmd}`);
  }
}
