// Pure picker logic (no DOM, no Tauri), so it is testable; see picker.test.ts.
import type { ChatModelOption } from "./bridge";

/**
 * The choice to pre-select and label "Recommended for this device" on first
 * run: the most capable STANDARD model that fits this device's free memory and
 * disk right now. `standardIds` is the picker's tier order, smallest first;
 * anything outside it (the iPad experiment) is never recommended. Falls back to
 * the smallest standard choice, so a first-run user always has one obvious tap
 * (design.md: zero configuration, the common path is the obvious one).
 */
export function recommendedChoice(options: ChatModelOption[], standardIds: string[]): string | null {
  const offered = standardIds.filter((id) => options.some((o) => o.id === id));
  if (offered.length === 0) return null;
  const fits = offered.filter((id) => {
    const o = options.find((x) => x.id === id)!;
    return o.fitsMemory && o.fitsDisk;
  });
  return fits.length > 0 ? fits[fits.length - 1] : offered[0];
}
