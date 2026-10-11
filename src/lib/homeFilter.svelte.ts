// Filter state for the records list. Module-level so it survives navigating to the
// editor and back within a session.
import type { SessionFilter, SessionForm } from "./types";

export const homeFilter = $state({
  /** "temae:<id>" | "cat:<id>" | "" */
  temaeSel: "",
  myRoleOnly: false,
  place_id: null as string | null,
  teacher_id: null as string | null,
  since: "" as string,
  until: "" as string,
  persons: [] as string[],
  tools: [] as string[],
  text: "",
  form: "" as "" | SessionForm,
  /** "" any | "1" lessons | "0" real */
  practice: "" as "" | "1" | "0",
  showNotes: false,
});

export function resetHomeFilter() {
  Object.assign(homeFilter, {
    temaeSel: "", myRoleOnly: false, place_id: null, teacher_id: null, since: "", until: "",
    persons: [], tools: [], text: "", form: "", practice: "",
  });
}

export function homeFilterActive(): boolean {
  const f = homeFilter;
  return !!(f.temaeSel || f.myRoleOnly || f.place_id || f.teacher_id || f.since || f.until || f.persons.length || f.tools.length || f.text.trim() || f.form || f.practice);
}

/** Build the backend filter; `expandCat` turns "cat:<id>" into the temae ids beneath it. */
export function toSessionFilter(expandCat: (catId: string) => string[]): SessionFilter {
  const f = homeFilter;
  let temae: string[] = [];
  if (f.temaeSel.startsWith("temae:")) temae = [f.temaeSel.slice(6)];
  else if (f.temaeSel.startsWith("cat:")) temae = expandCat(f.temaeSel.slice(4));
  return {
    temae,
    my_role: f.myRoleOnly ? "teishu" : null,
    place_id: f.place_id,
    teacher_id: f.teacher_id,
    since: f.since || null,
    until: f.until || null,
    persons: f.persons,
    tools: f.tools,
    text: f.text.trim() || null,
    form: f.form || null,
    practice: f.practice === "" ? null : f.practice === "1",
  };
}
