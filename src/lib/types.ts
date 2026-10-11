// Mirrors crates/chanote-core/src/model.rs. Keep in sync.

export type VocabKind =
  | "temae" | "temae-cat" | "place" | "person" | "tana" | "hearth" | "tool" | "tool-cat" | "school" | "event-kind";

export interface Vocab {
  id: string;
  kind: VocabKind | string;
  ja: string;
  romaji?: string | null;
  en?: string | null;
  parent_id?: string | null;
  sort?: number | null;
  origin: string;
  archived: boolean;
  meta?: Record<string, unknown> | null;
}

export interface AxisValues {
  hearth?: string;
  katte?: string;
  room?: string;
  tea?: string;
  tana?: string;
  kinin?: string;
  tsukimono?: string[];
}

export interface ResolvedTemae {
  temae_id: string;
  temae_alias_id: string | null;
  axes: AxisValues;
}

export type DatePrec = "day" | "month" | "year" | "unknown";
/** What happened: ordinary lesson / stand-alone procedures, a chaji, a chakai, a workshop, or unknown. */
export type SessionForm = "temae" | "chaji" | "chakai" | "koshukai" | "?";
export const SESSION_FORMS: SessionForm[] = ["temae", "chaji", "chakai", "koshukai", "?"];
export type MyRole = "teishu" | "hanto" | "mizuya" | "shokyaku" | "kyaku" | "tsume" | "kengaku" | "-";
export const MY_ROLES: MyRole[] = ["teishu", "shokyaku", "kyaku", "tsume", "hanto", "mizuya", "kengaku", "-"];
export type SessionRole = "teacher" | "host" | "guest" | "hanto" | "mizuya" | "kengaku";
/** Roster roles other than teacher (the ones temae rows inherit). */
export const ROSTER_ROLES: SessionRole[] = ["guest", "host", "hanto", "mizuya", "kengaku"];

export interface SessionPerson {
  person_id: string | null;
  name_free: string | null;
  role: SessionRole;
  seat: number | null;
  is_last: boolean;
  role_note: string | null;
}

export interface TemaePerson {
  person_id: string | null;
  name_free: string | null;
  role: "kyaku" | "hanto" | "mizuya" | "kengaku";
  seat: number | null;
  is_last: boolean;
  role_note: string | null;
}

export interface TemaeTool {
  tool_id: string | null;
  name_free: string | null;
  external_ref: string | null;
}

export interface Temae {
  id?: number | null;
  time: string | null;
  temae_id: string | null;
  temae_alias_id: string | null;
  temae_free: string | null;
  host_id: string | null;
  host_free: string | null;
  teacher_id: string | null;
  my_role: MyRole | null;
  my_role_note: string | null;
  my_seat: number | null;
  hearth: string | null;
  katte: string | null;
  room: string | null;
  tea: string | null;
  tana_id: string | null;
  tana_free: string | null;
  kinin: string | null;
  note: string | null;
  persons: TemaePerson[];
  tsukimono: string[];
  tools: TemaeTool[];
}

export interface Session {
  id?: number | null;
  date: string | null;
  date_prec: DatePrec;
  place_id: string | null;
  place_note: string | null;
  form: SessionForm;
  /** vocab id of kind event-kind, "?" or null */
  event_kind: string | null;
  /** lesson / rehearsal (true) or the real thing (false) */
  practice: boolean;
  /** my role at the event as a whole */
  my_role: MyRole | null;
  my_seat: number | null;
  online: boolean;
  note: string | null;
  persons: SessionPerson[];
  temae: Temae[];
}

export interface SessionSummary {
  id: number;
  date: string | null;
  date_prec: DatePrec;
  place_id: string | null;
  form: SessionForm;
  event_kind: string | null;
  practice: boolean;
  my_role: MyRole | null;
  online: boolean;
  temae_count: number;
}

export interface AppInfo {
  version: string;
  db_path: string;
  schema_version: number;
  self_person_id: string;
  profile: string;
}

export interface Profile { id: string; name: string }
export interface ProfileInfo { profiles: Profile[]; current: string; ask_on_start: boolean }

export interface RankRow { id: string; score: number; n: number; last_date: string | null }

export interface ByRow {
  key: string;
  n_sessions: number;
  n_host: number;
  n_other: number;
  first_date: string | null;
  last_date: string | null;
}

/** Reserved ids usable in any axis column. */
export const UNKNOWN = "?";
export const NONE = "-";

export function emptyTemae(): Temae {
  return {
    time: null, temae_id: null, temae_alias_id: null, temae_free: null,
    host_id: null, host_free: null, teacher_id: null,
    my_role: null, my_role_note: null, my_seat: null,
    hearth: null, katte: "jun", room: null, tea: null,
    tana_id: null, tana_free: null, kinin: null, note: null,
    persons: [], tsukimono: [], tools: [],
  };
}

export function emptySession(): Session {
  return {
    date: new Date().toISOString().slice(0, 10), date_prec: "day",
    place_id: null, place_note: null, form: "temae", event_kind: null, practice: true,
    my_role: null, my_seat: null, online: false, note: null,
    persons: [], temae: [],
  };
}

// ---- stats ----------------------------------------------------------------

export interface CoverageRow {
  temae_id: string | null;
  alias_id: string | null;
  hearth: string | null;
  role: "host" | "other" | "absent";
  n: number;
  n_recent: number;
  first_date: string | null;
  last_date: string | null;
}

export interface StatsFilter {
  since?: string | null;
  until?: string | null;
  recent_days?: number | null;
  place_id?: string | null;
  practice?: boolean | null;
}

export interface EventRow {
  form: SessionForm;
  event_kind: string | null;
  practice: boolean;
  my_role: MyRole | null;
  n: number;
  first_date: string | null;
  last_date: string | null;
}

export interface StatsTotals {
  sessions: number;
  temae_as_host: number;
  temae_as_other: number;
  first_date: string | null;
  last_date: string | null;
}

export interface MonthlyRow {
  month: string;             // YYYY-MM
  hearth: "ro" | "furo" | "other";
  role: "host" | "other" | "absent";
  n_temae: number;
  n_sessions: number;
}

export interface CrossCell {
  row: string | null;
  col: string | null;
  n: number;
  last_date: string | null;
}

export interface Attachment {
  id: number;
  session_id: number | null;
  temae_id: number | null;
  path: string;
  caption: string | null;
  created_at: string;
}

export interface ImportReport {
  vocab_added: number;
  vocab_skipped: number;
  sessions_added: number;
  sessions_skipped: number;
}

export const TSUKIMONO = [
  "koicha", "sumi", "kinin-kiyotsugu", "tsubokazari", "musubi-fukusa", "jikukazari", "kou", "nagekomi",
] as const;

export interface SessionFilter {
  temae?: string[];
  since?: string | null;
  until?: string | null;
  place_id?: string | null;
  teacher_id?: string | null;
  persons?: string[];
  tools?: string[];
  my_role?: string | null;
  text?: string | null;
  form?: SessionForm | null;
  event_kind?: string | null;
  practice?: boolean | null;
}

export interface ImportOptions {
  self_as?: string | null;
  original_self_name?: string | null;
  include?: number[] | null;
}
export interface PreviewPerson { id: string; name: string; is_self: boolean; n: number }
export interface PreviewRow {
  index: number; date: string | null; date_prec: string; place: string; form: string; practice: boolean;
  summary: string; n_temae: number; duplicate_of: number | null;
}
export interface ImportPreview { persons: PreviewPerson[]; rows: PreviewRow[]; vocab_new: number; schema_version: number }
