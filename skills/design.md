# Design

Human-centered design as procedural craft, not as an attitude. The perspective lives in `../CLAUDE.md` — what matters here is the *moves* that turn that perspective into actual interfaces.

## Assume zero technical knowledge

This is the governing assumption behind every design decision, not a nice-to-have. The person using Verdure knows nothing about computers — not RAM, not models, not "GGUF," not "GPU," not "quantization." They know English, and they know what they want to *do*: "write stories," "turn my recordings into text," "make a picture for my blog." Design for exactly that person, every time. The technical user is a happy side effect, never the target. If a screen only works for someone who already understands the tech, it has failed Verdure's actual audience.

Concretely:

- **No jargon on the surface.** Never make the user read a word they would have to look up. "How much memory does your computer have?" not "RAM capacity." Model names (Whisper, SDXL, Llama) are jargon too — so frame them, don't explain them: the headline is the plain benefit, and the model name appears as the clearly-labeled *answer* ("Use this: Whisper") with a "learn more" link for the curious.
- **Lead with the verb, but never bury the solution.** The user came to *do something*, so the headline is the verb — "Turn speech into text." But the recommendation itself (what to use, and how to get it) is the *answer they came for*: make it bold, large, and the next thing they see — never a small grey aside they have to hunt for, and never below a "coming soon" CTA. Verb-first framing and a prominent solution are not in tension; demoting the *framing* of the technology must not mean demoting the *visibility* of the answer. (Learned the hard way: a first pass shrank the model to an ignorable line and users couldn't find what to do.)
- **The relative test.** Every screen should pass: "would a non-technical relative understand this without asking me a question?" If not, it isn't done.
- **When a technical fact is unavoidable, wrap it in lived experience.** Asking about the device is the hard case: don't ask for specs. Ask what they can answer from memory — the brand, roughly how old it is, whether it's a gaming machine — and infer the rest. "I don't know" must always be a first-class answer, never a dead end.

## The vocabulary (from Norman)

Use these terms precisely. They make design decisions discussable.

- **Affordance** — what an interface invites you to do. A button affords clicking. A scrollable area affords scrolling. If the user can't tell what something affords, the design has failed.
- **Signifier** — the visible cue that an affordance exists. A button looks like a button because its shape, shadow, and color signify "clickable." Affordance without signifier is invisible capability.
- **Feedback** — the system tells the user what happened. The recommendation page loaded. The form submission succeeded. The intent was parsed as X. Silent systems destroy trust because users can't tell whether their input was received.
- **Mapping** — controls correspond naturally to outcomes. The "easiest" install path is visually larger or more prominent than the "technical" one. If a slider goes left to decrease and right to increase, that's natural mapping; the reverse is hostile.
- **Constraints** — what the design prevents. Disabling the submit button until the form is valid is a constraint that protects the user from a guaranteed error.

## Apply these to Verdure's recommendation page

A worked example. For the four-screen flow described in `../knowledge/05-website-spec.md`:

- The three install paths must *afford* their relative simplicity. "Install Verdure" should look easier than "use LM Studio"; "use Ollama" should look more technical than both. Same visual weight for all three = no signal.
- The device picker cards must *signify* what each one means. "Older laptop (5+ years old, basic specs)" needs an icon, a short description, and ideally an example device, so the user can pick with confidence.
- The intent parse must *feed back*. "We understood: you want help with creative writing." Then a confirm button. The user is never left guessing what Claude thought they meant.
- The mapping from intent to recommendation must feel natural — creative writing → a model good at creative writing, with a short reason. If the user can't see why the recommendation matches, the mapping has failed.

## User-impact statement, written before the code

Before building any user-facing change, write one sentence: "After this change, the user will be able to [X] more easily because [Y]." If you can't fill in both halves, you don't yet have a clear enough idea to start. If [Y] is hand-wavy, the change probably won't deliver.

After building, the same sentence becomes the acceptance check.

## Design for the floor

Verdure's primary audience runs on Tier L1 / P1 hardware and is the least technical user. Every design decision is for them first:

- Plain language, not jargon. "How much memory does your computer have?" not "RAM capacity."
- Visual cues over text where possible.
- The most common path is the most obvious path.
- The "I don't know" option is a first-class choice, not a dead end.
- Honest performance numbers in human terms — "about reading speed" alongside "15 tokens/sec" beats either alone for the floor user.

If a design only works for the technical user, it has failed Verdure's audience even if the technical user is delighted. See `../knowledge/02-hardware-tiers.md` for tier definitions.

## The simplicity check — applied at every step

After any user-facing change, ask: did this remove a step or add one? If it added a step, justify the addition explicitly. The default is to remove. Steps the user has to take are friction; friction is failure.

## The visual self-review loop

After building any UI, render it and look at it. For the Phase 0 website (React):

1. Run the app (or use Playwright headless).
2. Screenshot the page at mobile and desktop viewports.
3. Look at the screenshots.
4. Compare to the design intent. Does the device picker actually look scannable? Does the recommendation feel honest or marketing-y? Is the "easiest" path actually visually easiest?
5. If something is off, iterate. The loop closes inside the session.

The user-impact statement isn't done when written. It's done when verified against what actually shipped.

## Honest performance numbers — the trust loop

The "honest performance expectations" principle in `../knowledge/06-privacy-principles.md` is a design commitment, not just a policy. Every quantitative claim on the recommendation page ("about 15 tokens/sec," "uses around 3 GB") is a promise.

To keep that promise honest *without* violating the no-telemetry principle:

- Verdure benchmarks its own physical devices (see `[OQ-03]`). This is the primary source.
- Users may *opt in* to share their results explicitly — a button, a PR, never passive collection.
- The Phase 1 binary can show users a local-only comparison ("predicted 15 tok/s, you got 17") that stays on their machine.

Never default to passive data collection. Refusing telemetry must never produce a worse recommendation.

## Self-check before declaring a UI change done

- Have I written the user-impact statement?
- Did I render and look at the result?
- Does the change remove a step or add one?
- Is the floor user (Tier L1 / P1, least technical) still served well?
- Are any performance claims still honest?
