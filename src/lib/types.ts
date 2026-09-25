// Mirrors crates/chanote-core/src/model.rs. Keep in sync.

export type VocabKind =
  | "temae" | "temae-cat" | "place" | "person" | "tana" | "hearth" | "tool" | "school";

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
export type SessionKind = "keiko" | "koshukai" | "honban";
export type MyRole = "teishu" | "hanto" | "mizuya" | "kyaku" | "shokyaku" | "kengaku" | "-";

export interface SessionPerson {
  person_id: string | null;
  name_free: string | null;
  role: "teacher" | "guest";
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
  kind: SessionKind;
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
  kind: SessionKind;
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
    place_id: null, place_note: null, kind: "keiko", online: false, note: null,
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
