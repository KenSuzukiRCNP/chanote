// Vocabulary cache shared by all screens. Loaded once at startup, refreshed after edits.
import { api } from "./api";
import type { Vocab } from "./types";

const KINDS = ["temae", "temae-cat", "place", "person", "tana", "hearth", "tool-cat", "tool", "school", "event-kind"] as const;
const RANKED = ["person", "place", "tana", "tool"] as const;

export const vocab = $state({
  byKind: {} as Record<string, Vocab[]>,
  byId: {} as Record<string, Vocab>,
  loaded: false,
  selfId: "person.self",
  /** recency-weighted usage score per id (see core ranking.rs) */
  rank: {} as Record<string, number>,
});

export async function loadVocab(includeArchived = true) {
  const byKind: Record<string, Vocab[]> = {};
  const byId: Record<string, Vocab> = {};
  for (const k of KINDS) {
    const rows = await api.vocabList(k, includeArchived);
    byKind[k] = rows;
    for (const r of rows) byId[r.id] = r;
  }
  vocab.byKind = byKind;
  vocab.byId = byId;
  vocab.loaded = true;
  await loadRanking();
}

export async function loadRanking() {
  const rank: Record<string, number> = {};
  for (const k of RANKED) {
    try {
      for (const r of await api.vocabRanking(k)) rank[r.id] = r.score;
    } catch { /* ranking is a convenience only */ }
  }
  vocab.rank = rank;
}

/** Active rows of a kind, most-used-recently first; unused rows keep their list order. */
export function ranked(kind: string): Vocab[] {
  const rows = active(kind);
  return rows
    .map((v, i) => ({ v, i, s: vocab.rank[v.id] ?? 0 }))
    .sort((a, b) => b.s - a.s || a.i - b.i)
    .map((x) => x.v);
}

/** Tools grouped by category (pack order), each group ranked. */
export function toolGroups(): { cat: Vocab | null; items: Vocab[] }[] {
  const cats = active("tool-cat");
  const all = ranked("tool");
  const groups: { cat: Vocab | null; items: Vocab[] }[] = cats
    .map((cat) => ({ cat: cat as Vocab | null, items: all.filter((v) => v.parent_id === cat.id) }))
    .filter((g) => g.items.length);
  const orphan = all.filter((v) => !v.parent_id || !cats.some((c) => c.id === v.parent_id));
  if (orphan.length) groups.push({ cat: null, items: orphan });
  return groups;
}

export function get(id: string | null | undefined): Vocab | undefined {
  return id ? vocab.byId[id] : undefined;
}

export function active(kind: string): Vocab[] {
  return (vocab.byKind[kind] ?? []).filter((v) => !v.archived);
}

export function teachers(): Vocab[] {
  return active("person").filter((v) => (v.meta as { teacher?: boolean } | null)?.teacher);
}

/** Category tree for pickers: each category with its temae (aliases included) and its
 *  sub-categories, in pack order. */
export interface CatNode { cat: Vocab; items: Vocab[]; children: CatNode[] }
export function categoryTree(): CatNode[] {
  const cats = active("temae-cat");
  const all = active("temae");
  const build = (parent: string | null): CatNode[] =>
    cats.filter((c) => (c.parent_id ?? null) === parent).map((c) => ({
      cat: c, items: all.filter((v) => v.parent_id === c.id), children: build(c.id),
    }));
  return build(null);
}

/** All temae ids (main and alias) under a category, recursively. */
export function temaeUnderCategory(catId: string): string[] {
  const cats = active("temae-cat");
  const all = active("temae");
  const out: string[] = [];
  const walk = (id: string) => {
    for (const v of all) if (v.parent_id === id) out.push(v.id);
    for (const c of cats) if (c.parent_id === id) walk(c.id);
  };
  walk(catId);
  return out;
}

/** Uncategorised temae (user-added without a category). */
export function uncategorisedTemae(): Vocab[] {
  const catIds = new Set(active("temae-cat").map((c) => c.id));
  return active("temae").filter((v) => !v.parent_id || !catIds.has(v.parent_id));
}

/** Display name of the pack a row came from ("裏千家"), or null for user rows. */
export function packName(v: Vocab): string | null {
  if (!v.origin.startsWith("pack:")) return null;
  const school = vocab.byId["school." + v.origin.slice(5)];
  return school ? school.ja : v.origin.slice(5);
}

/** Event kinds (正午の茶事, 初釜 …) for a form, pack order. */
export function eventKinds(form: string): Vocab[] {
  return active("event-kind").filter((v) => (v.meta as { form?: string } | null)?.form === form);
}
