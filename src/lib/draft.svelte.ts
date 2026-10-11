// A session waiting to be edited as a new record ("duplicate"). Module state so it
// survives the hash navigation from the editor to #/new.
import type { Session } from "./types";

let pending: Session | null = null;

export function setDraft(s: Session) {
  pending = s;
}

/** Take the draft (once). */
export function takeDraft(): Session | null {
  const d = pending;
  pending = null;
  return d;
}
