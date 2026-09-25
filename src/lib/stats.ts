// Turns CoverageRow cubes from the backend into the coverage table and the
// "not done recently" ranking. Pure functions; no Svelte here.

import type { CoverageRow, Vocab } from "./types";

export type HearthGroup = "ro" | "furo" | "other";

/** Collapse the raw hearth axis into the two seasons plus "other" (tea box etc.). */
export function hearthGroup(h: string | null | undefined): HearthGroup {
  switch (h) {
    case "ro": case "dairo": case "tsurigama": case "sukigi": return "ro";
    case "furo": case "nakaoki": return "furo";
    default: return "other";
  }
}

/** Season for a date: ro = Nov–Apr, furo = May–Oct. */
export function seasonOf(date = new Date()): HearthGroup {
  const m = date.getMonth() + 1;
  return m >= 5 && m <= 10 ? "furo" : "ro";
}

export interface Tally {
  n: number;
  nRecent: number;
  last: string | null;
  first: string | null;
}

const empty = (): Tally => ({ n: 0, nRecent: 0, last: null, first: null });

function add(t: Tally, r: CoverageRow) {
  t.n += r.n;
  t.nRecent += r.n_recent;
  if (r.last_date && (!t.last || r.last_date > t.last)) t.last = r.last_date;
  if (r.first_date && (!t.first || r.first_date < t.first)) t.first = r.first_date;
}

export interface TemaeStat {
  temae: Vocab;
  host: Tally;
  other: Tally;
  byHearth: Record<HearthGroup, { host: Tally; other: Tally }>;
  /** alias id → count as host (for the small breakdown under the name) */
  aliases: Record<string, number>;
}

export interface CategoryBlock {
  category: Vocab | null;
  /** breadcrumb of parents, root first */
  path: Vocab[];
  rows: TemaeStat[];
}

/**
 * Build the coverage table. Every non-archived pack/user temae that is a main entry
 * (not an alias) appears, so unseen temae are visible. Free-text temae are excluded.
 */
export function buildCoverage(
  rows: CoverageRow[],
  temaeList: Vocab[],
  categories: Vocab[],
): CategoryBlock[] {
  const byId = new Map<string, TemaeStat>();
  const isAlias = (v: Vocab) => typeof (v.meta as { alias_of?: string } | null)?.alias_of === "string";

  for (const v of temaeList) {
    if (v.archived || isAlias(v)) continue;
    byId.set(v.id, {
      temae: v,
      host: empty(),
      other: empty(),
      byHearth: {
        ro: { host: empty(), other: empty() },
        furo: { host: empty(), other: empty() },
        other: { host: empty(), other: empty() },
      },
      aliases: {},
    });
  }

  for (const r of rows) {
    if (!r.temae_id) continue;
    const s = byId.get(r.temae_id);
    if (!s) continue; // archived or unknown id
    if (r.role === "absent") continue;
    const tgt = r.role === "host" ? "host" : "other";
    add(s[tgt], r);
    add(s.byHearth[hearthGroup(r.hearth)][tgt], r);
    if (r.role === "host" && r.alias_id) s.aliases[r.alias_id] = (s.aliases[r.alias_id] ?? 0) + r.n;
  }

  // group by category, in pack order; nested categories get a breadcrumb path
  const catById = new Map(categories.map((c) => [c.id, c]));
  const pathOf = (c: Vocab): Vocab[] => {
    const out: Vocab[] = [];
    let cur: Vocab | undefined = c;
    while (cur) {
      out.unshift(cur);
      cur = cur.parent_id ? catById.get(cur.parent_id) : undefined;
    }
    return out;
  };

  const blocks: CategoryBlock[] = [];
  for (const c of categories) {
    const rowsIn = [...byId.values()].filter((s) => s.temae.parent_id === c.id);
    if (rowsIn.length) blocks.push({ category: c, path: pathOf(c), rows: rowsIn });
  }
  const orphans = [...byId.values()].filter((s) => !s.temae.parent_id || !catById.has(s.temae.parent_id));
  if (orphans.length) blocks.push({ category: null, path: [], rows: orphans });
  return blocks;
}

export function daysSince(date: string | null, today = new Date()): number | null {
  if (!date) return null;
  const d = Date.parse(date);
  if (Number.isNaN(d)) return null;
  return Math.floor((today.getTime() - d) / 86400e3);
}

export interface StaleRow {
  stat: TemaeStat;
  last: string | null;
  days: number | null;
  n: number;
  /** last time in the current season's hearth group */
  lastInSeason: string | null;
}

/**
 * Temae done at least once as host, oldest-last-time first.
 * `seasonOnly` restricts to the current season's hearth group.
 */
export function buildStale(blocks: CategoryBlock[], seasonOnly: boolean, today = new Date()): StaleRow[] {
  const season = seasonOf(today);
  const out: StaleRow[] = [];
  for (const b of blocks) for (const s of b.rows) {
    const t = seasonOnly ? s.byHearth[season].host : s.host;
    if (t.n === 0) continue;
    out.push({
      stat: s,
      last: t.last,
      days: daysSince(t.last, today),
      n: t.n,
      lastInSeason: s.byHearth[season].host.last,
    });
  }
  out.sort((a, b) => (b.days ?? -1) - (a.days ?? -1));
  return out;
}
