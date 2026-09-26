// UI strings + tea-term display. Svelte 5 runes module.
import ja from "$locales/ja.json";
import en from "$locales/en.json";
import type { Vocab } from "./types";

export type UiLang = "ja" | "en";
export type TermMode = "ja" | "ja-romaji" | "romaji";

const dict: Record<UiLang, Record<string, string>> = { ja, en };

export const i18n = $state({
  lang: "ja" as UiLang,
  termMode: "ja" as TermMode,
});

/** UI string with {placeholders}. Falls back to ja, then the key. */
export function t(key: string, params?: Record<string, string | number>): string {
  let s = dict[i18n.lang][key] ?? dict.ja[key] ?? key;
  if (params) for (const [k, v] of Object.entries(params)) s = s.replaceAll(`{${k}}`, String(v));
  return s;
}

/** Display form of a vocabulary term according to the term mode. */
export function term(v: Vocab | null | undefined): string {
  if (!v) return "";
  let r = v.romaji?.trim();
  if (r === v.ja) r = undefined; // alphabet-only names: don't print "Anna (Anna)"
  switch (i18n.termMode) {
    case "ja": return v.ja;
    case "romaji": return r || v.ja;
    case "ja-romaji": return r ? `${v.ja} (${r})` : v.ja;
  }
}

/** Hover text: English gloss when present. */
export function termTitle(v: Vocab | null | undefined): string | undefined {
  return v?.en ?? undefined;
}

/** Display for an axis value that is a fixed enum ('koicha', 'jun', ...) or reserved ('?', '-'). */
export function axisLabel(axis: string, value: string | null | undefined): string {
  if (value == null || value === "") return t("value.unset");
  if (value === "?") return t("value.unknown");
  if (value === "-") return dict[i18n.lang][`axis.${axis}.-`] ?? t("value.none");
  return dict[i18n.lang][`axis.${axis}.${value}`] ?? value;
}

export function setLang(lang: UiLang) {
  i18n.lang = lang;
  // sensible default term mode per language; user can override in settings
  i18n.termMode = lang === "ja" ? "ja" : "ja-romaji";
}
