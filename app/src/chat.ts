// The chat screen's rules for the conversation on disk (knowledge/11, "Mobile
// non-negotiables": the conversation persists; the screen renders from the
// transcript the Rust core keeps). `bridge.ts` owns the mechanism — loading,
// appending, the done event; this module owns the one rule that needs care:
// how what the screen already shows is reconciled with what the transcript
// now holds, so nothing is painted twice and nothing is lost.

/** One line of the transcript, as the screen needs it. */
export type Line = { role: "user" | "assistant"; text: string };

/**
 * Which newly-arrived transcript entries still need painting. The screen
 * echoes what the user typed, and a lookup's answer, the moment they happen
 * (feedback must be immediate), and the transcript records the same lines a
 * beat later. Each pending echo claims the first unpainted entry with the
 * same role and text; everything unclaimed is painted. Echoes that are never
 * recorded (a question that became "which Jackson?") simply stay pending —
 * they match nothing, so they hide nothing.
 */
export function reconcile(pending: Line[], arrived: Line[]): { paint: boolean[]; pending: Line[] } {
  const left = pending.slice();
  const paint = arrived.map((e) => {
    const i = left.findIndex((p) => p.role === e.role && p.text === e.text);
    if (i === -1) return true;
    left.splice(i, 1);
    return false;
  });
  return { paint, pending: left };
}
