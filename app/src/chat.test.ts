import { describe, expect, it } from "vitest";
import { reconcile, type Line } from "./chat";

const u = (text: string): Line => ({ role: "user", text });
const a = (text: string): Line => ({ role: "assistant", text });

describe("reconcile — what the screen paints when the transcript catches up", () => {
  it("paints nothing for lines the screen already echoed", () => {
    const r = reconcile([u("hello"), a("hi")], [u("hello"), a("hi")]);
    expect(r.paint).toEqual([false, false]);
    expect(r.pending).toEqual([]);
  });

  it("paints a reply that finished while the user was away", () => {
    const r = reconcile([u("hello")], [u("hello"), a("hi, welcome back")]);
    expect(r.paint).toEqual([false, true]);
    expect(r.pending).toEqual([]);
  });

  it("paints everything on a fresh screen", () => {
    const r = reconcile([], [u("one"), a("1"), u("two")]);
    expect(r.paint).toEqual([true, true, true]);
  });

  it("an echo that was never recorded hides nothing later", () => {
    // "Which Jackson?" was asked instead of recorded; the next question is real.
    const r = reconcile([u("call jackson"), u("hello")], [u("hello"), a("hi")]);
    expect(r.paint).toEqual([false, true]);
    expect(r.pending).toEqual([u("call jackson")]);
  });

  it("the same words twice are two entries, matched one each", () => {
    const r = reconcile([u("again"), u("again")], [u("again"), a("ok"), u("again")]);
    expect(r.paint).toEqual([false, true, false]);
    expect(r.pending).toEqual([]);
  });
});
