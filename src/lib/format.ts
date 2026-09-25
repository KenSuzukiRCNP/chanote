import type { DatePrec } from "./types";

/** "2026-05-10" + prec → "2026-05-10" | "2026-05" | "2026" | "?" */
export function fmtDate(date: string | null, prec: DatePrec): string {
  if (!date || prec === "unknown") return "—";
  if (prec === "year") return date.slice(0, 4);
  if (prec === "month") return date.slice(0, 7);
  return date;
}
