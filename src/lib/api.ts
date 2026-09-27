// Thin typed wrapper over Tauri commands. In a plain browser (vite dev without Tauri)
// `invoke` is unavailable; we fall back to a small in-memory mock so the UI can be
// developed and screenshotted without the Rust side.

import type {
  AppInfo, Attachment, AxisValues, ByRow, CoverageRow, CrossCell, ImportOptions, ImportPreview,
  ImportReport, MonthlyRow, Profile, ProfileInfo, RankRow, ResolvedTemae, Session,
  SessionFilter, SessionSummary, StatsFilter, StatsTotals, Vocab,
} from "./types";

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

let invoke: Invoke;
const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

if (isTauri) {
  const core = await import("@tauri-apps/api/core");
  invoke = core.invoke as Invoke;
} else {
  const mock = await import("./mock");
  invoke = mock.invoke as Invoke;
}

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),

  settingGet: (key: string) => invoke<string | null>("setting_get", { key }),
  settingSet: (key: string, value: string) => invoke<void>("setting_set", { key, value }),

  vocabList: (kind: string, includeArchived = false) =>
    invoke<Vocab[]>("vocab_list", { kind, includeArchived }),
  vocabUpsert: (vocab: Vocab) => invoke<void>("vocab_upsert", { vocab }),
  vocabArchive: (id: string, archived: boolean) => invoke<void>("vocab_archive", { id, archived }),
  vocabDelete: (id: string) => invoke<void>("vocab_delete", { id }),
  packResetRow: (id: string) => invoke<boolean>("pack_reset_row", { id }),
  vocabUsage: (id: string) => invoke<number>("vocab_usage", { id }),
  resolveTemae: (id: string) => invoke<ResolvedTemae>("resolve_temae", { id }),
  temaeDefaults: (id: string) => invoke<AxisValues>("temae_defaults", { id }),

  sessionList: (limit = 100, offset = 0) =>
    invoke<SessionSummary[]>("session_list", { limit, offset }),
  sessionSearch: (filter: SessionFilter, limit = 500, offset = 0) =>
    invoke<SessionSummary[]>("session_search", { filter, limit, offset }),
  sessionGet: (id: number) => invoke<Session | null>("session_get", { id }),
  sessionSave: (session: Session) => invoke<number>("session_save", { session }),
  sessionDelete: (id: number) => invoke<void>("session_delete", { id }),

  statsCoverage: (filter: StatsFilter = {}) => invoke<CoverageRow[]>("stats_coverage", { filter }),
  statsTotals: (filter: StatsFilter = {}) => invoke<StatsTotals>("stats_totals", { filter }),

  /** Write an export file; `path` comes from the native save dialog. */
  exportWrite: (path: string, format: "json" | "csv") =>
    invoke<number>("export_write", { path, format }),
  backupWrite: (path: string) => invoke<void>("backup_write", { path }),
  backupZip: (path: string) => invoke<number>("backup_zip", { path }),
  backupRestore: (path: string) => invoke<number>("backup_restore", { path }),
  appRestart: () => invoke<void>("app_restart"),
  importRead: (path: string) => invoke<ImportReport>("import_read", { path }),
  importPreview: (path: string, options: ImportOptions) => invoke<ImportPreview>("import_preview", { path, options }),
  importApply: (path: string, options: ImportOptions) => invoke<ImportReport>("import_apply", { path, options }),

  statsMonthly: (filter: StatsFilter = {}) => invoke<MonthlyRow[]>("stats_monthly", { filter }),
  statsCross: (rowAxis: string, colAxis: string, hostOnly: boolean, filter: StatsFilter = {}) =>
    invoke<CrossCell[]>("stats_cross", { rowAxis, colAxis, hostOnly, filter }),
  statsAxes: () => invoke<string[]>("stats_axes"),
  statsBy: (dimension: string, filter: StatsFilter = {}) => invoke<ByRow[]>("stats_by", { dimension, filter }),
  vocabRanking: (kind: string) => invoke<RankRow[]>("vocab_ranking", { kind }),

  profileList: () => invoke<ProfileInfo>("profile_list"),
  profileCreate: (name: string) => invoke<Profile>("profile_create", { input: { name } }),
  profileRename: (id: string, name: string) => invoke<void>("profile_rename", { id, name }),
  profileSetAsk: (ask: boolean) => invoke<void>("profile_set_ask", { ask }),
  profileSwitch: (id: string) => invoke<void>("profile_switch", { id }),

  attachmentAdd: (sessionId: number, paths: string[]) =>
    invoke<Attachment[]>("attachment_add", { sessionId, paths }),
  attachmentList: (sessionId: number) => invoke<Attachment[]>("attachment_list", { sessionId }),
  attachmentThumb: (id: number) => invoke<string | null>("attachment_thumb", { id }),
  attachmentCaption: (id: number, caption: string | null) =>
    invoke<void>("attachment_caption", { id, caption }),
  attachmentDelete: (id: number) => invoke<void>("attachment_delete", { id }),
  attachmentPath: (id: number) => invoke<string | null>("attachment_path", { id }),
  attachmentCounts: () => invoke<[number, number][]>("attachment_counts"),
  attachmentRebuildThumbs: () => invoke<[number, number]>("attachment_rebuild_thumbs"),
};

export { isTauri };
