# Copywriting

How to write any text a user will read — website, app, error messages, emails. The goal is conversion, and for Verdure's audience the conversion mechanism is trust. Honest, clear, specific copy outperforms persuasion tactics here, because the audience is running local AI precisely to escape hype and manipulation.

This skill is bound by `../knowledge/06-privacy-principles.md`. Where a conversion tactic conflicts with those principles, the principles win. The prohibited tactics are listed at the end of this file — read them before writing.

## The core principle: conversion through trust

Verdure's audience is technical, privacy-conscious, and skeptical of marketing. For them, the failure mode is not "insufficiently persuasive" — it's "sounds like every other overhyped AI product." A lack of clarity undermines trust, and undermined trust is lost conversion. Specific, provable claims lower resistance; vague enthusiasm raises it.

Every piece of copy is doing one job: helping a real person decide whether Verdure will actually help them, and telling them the truth so they can. If the truth is "this won't work great on your device," the copy says so — and earns the trust that converts the next, better-matched user.

## Meet the reader at their awareness stage (Schwartz)

Eugene Schwartz's five stages of awareness are the single most useful framework here, and they map directly onto Verdure's intent-first flow. Before writing any piece of copy, identify where the reader is:

1. **Unaware** — doesn't know local AI is an option, or that their device can run it. Copy's job: name the possibility without jargon. ("You can run AI on the laptop you already own. No subscription, no cloud, no data leaving your machine.")
2. **Problem aware** — knows they want AI but thinks they need a subscription or a powerful machine. Copy's job: reframe. ("You don't need a $20/month plan or a new computer.")
3. **Solution aware** — knows local AI exists but finds it intimidating (Ollama, model files, quantization). Copy's job: position Verdure as the one that removes the complexity.
4. **Product aware** — has heard of Verdure, hasn't acted. Copy's job: the specific, honest recommendation and the clear next step.
5. **Most aware** — ready. Copy's job: get out of the way. One clear action.

Verdure's website naturally spans these. The landing headline speaks to the unaware/problem-aware; the recommendation page speaks to the product-aware/most-aware. Match the copy to the stage. A message that lands for one stage falls flat for another.

A caution from Schwartz that fits Verdure perfectly: copy cannot manufacture desire that isn't there. It can only connect to what the reader already wants. Don't try to make someone want local AI who doesn't — find the ones who already do, and speak clearly to them.

## Benefits over features — but keep the honest numbers

The standard rule: lead with what the user gets, not what the product is. A feature is "runs Gemma 3 4B at Q4_K_M." A benefit is "writes with you at about reading speed, and nothing you type ever leaves your laptop."

Verdure's twist: the benefit and the honest number belong *together*, not in place of each other. The specificity is itself a benefit because it's proof. "About 15 tokens per second — roughly reading speed" gives both the human meaning and the verifiable fact. Stripping the number to sound smoother would weaken the copy, because for this audience the number is the trust.

So: lead with the benefit, ground it in the specific number, translate the number into human terms. All three.

## The slippery slide (Sugarman) — earn the next sentence honestly

Joseph Sugarman's principle: the only job of each element is to get the reader to the next one. The headline earns the first sentence; the first sentence earns the second; and so on to the call to action. The reader slides down because each step is easy and rewarding, not because they're pushed.

Sugarman's Axiom 5 is the one that matters for Verdure: get the reader nodding "yes" to *accurate and truthful statements*. The slide works through agreement with true things, not through pressure. This is exactly the conversion-through-trust mechanism. Build the slide out of honest claims the reader can't help but agree with:

> Running AI usually means a monthly subscription. (Yes.)
> Or a powerful computer you don't have. (Yes.)
> But the laptop you're reading this on can probably do it already. (Wait, really?)
> Tell us what you want to do, and we'll tell you exactly what it can run. (Show me.)

Techniques that grease the slide honestly: short opening sentences, one idea per sentence, "seeds of curiosity" at paragraph ends (e.g., "Here's why that matters."), and conjunctions that carry momentum. Never use a curiosity gap that the payoff doesn't honestly fill. A tease you don't deliver on is a small lie, and small lies cost trust.

## Clarity beats cleverness

The most repeated finding across all the research: for technical and trust-sensitive audiences, clarity is the highest-leverage lever. Write to be understood instantly, not to sound impressive.

- Plain language over jargon. "How much memory does your computer have?" not "Specify your RAM allocation." This also serves the floor user (Tier L1/P1, least technical) — see `design.md`.
- One idea per sentence. Short sentences for the important beats.
- Concrete over abstract. "Your other apps keep working" beats "minimal resource contention."
- Scannable. The reader skims first. Headlines and first words of paragraphs should carry the message on their own.
- Identify the single goal of each page before writing it. If a page has two goals, it has none. Verdure's recommendation page has one: get the user to a confident next step.

## Specificity is proof (Cashvertising / Scientific Advertising)

Specific claims are believed; vague claims are discounted. "About 3 GB of memory while running" is credible. "Lightweight and efficient" is marketing noise the audience filters out. Wherever a number, a name, or a concrete detail can replace an adjective, replace it. This is the same instinct as Verdure's honest-numbers principle — here it does double duty as the most effective persuasion available.

## Social proof — only when real

Testimonials and usage numbers work, but for Verdure they must be real and verifiable. No invented quotes, no inflated counts. When Verdure has real users and real benchmarks contributed by real people (opt-in only — see `design.md` and `../knowledge/06-privacy-principles.md`), use them. Until then, don't fabricate the appearance of a crowd. A made-up testimonial discovered is a brand-ending event for a trust-first product.

## Calls to action — clear, low-friction, honest about what happens next

The reader should always know exactly what clicking does. Verdure's three install paths (see `../knowledge/05-website-spec.md`) are themselves a CTA design: ranked honestly by simplicity, each labeled for what it actually is. The "easiest" path is a notify-me button right now, and the copy says so plainly — it does not imply the binary already exists. Never let a CTA promise more than the click delivers.

Lower the friction of the action, not the honesty of the description. "Get notified when one-click install launches" is low-friction and truthful. "Install now" when there's nothing to install is friction-free and a lie.

## A/B test, measure honestly (Scientific Advertising)

Copy is tested, not guessed. But Verdure tests within its privacy constraints: privacy-respecting analytics only (Plausible or Fathom, per `../knowledge/05-website-spec.md`), measuring whether the user completed the flow, clicked an install path, or joined the launch list. No invasive tracking, no behavioral surveillance to optimize copy. Test the headline, test the recommendation framing, keep what converts honestly.

## Voice

Verdure's voice is the knowledgeable friend who happens to be an expert and refuses to bullshit you. Warm but not bubbly. Confident but not hyped. Technical when precision helps, plain when it doesn't. It respects the reader's intelligence and their time. It would rather tell you a model won't run well on your device than sell you a disappointment.

## Prohibited tactics — do not use these in Verdure copy

These conventional conversion tactics conflict with `../knowledge/06-privacy-principles.md` and with the audience's trust. They are off the table regardless of how well they work elsewhere:

- **Manufactured scarcity** — "only 3 spots left," fake countdown timers. Verdure has no inventory; any scarcity would be a lie.
- **False urgency / FOMO** — pressure to act now with no real deadline.
- **Decoy options** — deliberately inferior choices designed to manipulate selection. Verdure's three install paths are honestly ranked, not decoys.
- **Padded or hyped numbers** — inflating performance, rounding favorably, "blazing fast." Use real numbers or ranges, marked provisional when not yet measured (see `research.md`).
- **Curiosity gaps with no honest payoff** — teasing something the copy doesn't deliver.
- **Manipulation via cognitive bias** — any technique that works by exploiting the reader against their own interest. Verdure's premise is that the user is the source of truth; copy that tricks them violates the premise.
- **Fabricated social proof** — invented testimonials, inflated user counts.

If a future task seems to call for one of these, that is a signal to stop and reconsider the framing, not a reason to proceed. Surface it as an open question if genuinely unsure.

## Self-check before declaring copy done

- What awareness stage is this reader at, and does the copy match it?
- Does every claim survive the honesty test — is it true, specific, and provable?
- Did I lead with a benefit and ground it in a concrete number or detail?
- Does each sentence earn the next one without a pressure tactic or unfilled tease?
- Could the floor user (least technical) understand this instantly?
- Does the CTA describe exactly what happens on click?
- Did I reach for any prohibited tactic? If so, reframe.
- Would this copy make a skeptical, privacy-conscious reader trust Verdure more, or less?
