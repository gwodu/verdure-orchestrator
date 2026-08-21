# Debugging

How to find a bug that shouldn't exist. This is the skill for when something
behaves impossibly — the code looks right, the tests pass, and the app is wrong
anyway.

It is distinct from `testing.md`, which is about *preventing* defects. This is
about *locating* one that already escaped, cheaply, without burning days or CI
minutes on guesses. Every rule below was paid for in real time lost.

## The governing rule: verify the artifact, not the source

**When the question is "what does the user experience?", inspect the thing the
user actually has.** Not the repo, not the build log, not your intent.

Six consecutive mobile builds shipped Tauri's logo instead of Verdure's while the
correct icons sat in `src-tauri/icons/`. Reading those files "confirmed" the icon
was right every time. It was: they were also never consulted, because `gen/` is
regenerated from the template on each CI run. The bug was found in one step by
unzipping the shipped `.ipa` and looking at the PNG inside.

Concretely, for each surface:

- **A shipped app** — extract it. `unzip` the `.ipa`/`.apk`, mount the `.dmg`,
  read the file out of the bundle.
- **A CI result** — read the log of the run for *that commit's SHA* (see below).
- **A model's behaviour** — run the model, don't reason about the registry entry.
- **A downloaded file** — check its bytes: size against the source, magic header,
  and whether the tail is real data rather than zeros.

If a check would pass whether or not the bug exists, it is not a check.

## Differential diagnosis: borrow a reference implementation

The highest-value move in this codebase, by a wide margin.

Verdure wraps other people's engines (llama.cpp, whisper.cpp). When our wrapper
produces wrong output, there are two candidate faults — **our code** or **the
dependency as we build it** — and arguing about which wastes days. So don't
argue: **get the upstream project's own prebuilt binary, run it on the same
input, on the same machine, and compare.**

That single command (2026-08-21) eliminated in one shot: the machine, available
memory, disk, the model file, the quantization format, the model's size, our
conversation-memory feature, the system prompt, and the sampler configuration —
after two days of hypotheses. Upstream answered correctly at 42.5 tok/s from the
byte-identical model file that made our build emit `uids@@@@@@`. Everything
outside our binary was exonerated at once.

The pattern generalizes: **hold everything constant except the one component you
suspect, and get a known-good version of that component from its author.**
Upstream ships prebuilt binaries for exactly this reason.

## Bisect over artifacts you already have

CI keeps build artifacts for 90 days. That is a free bisection space — a working
binary from any past commit, at no build cost and no toolchain.

When a regression appears, list the past builds, map each to its commit, download
one from the middle, and run it. Two downloads usually halve the search space
faster than any amount of reading diffs. It also survives the case where you
*can't* build locally.

Record which builds were tested and what they showed, so the next session doesn't
repeat the downloads.

## Eliminate loudly, and write down what's dead

A hypothesis that has been tested and disproven is an asset — but only if it's
recorded. Otherwise it gets re-tested, and long-running bugs get re-investigated
from scratch every session.

Keep the running eliminations in the `[OQ-XX]` entry as you go, phrased as
"**eliminated — do not re-test**", with the evidence. `[OQ-71]` is the worked
example: by the third session its entry had ruled out eight explanations, which
is what made the remaining two obvious.

Be equally willing to kill *your own* theory. Two confident hypotheses died in
one session here — "the K-quant kernels are broken on x86" (disproven by the
reference binary) and "our tokenizer isn't parsing special tokens" (disproven by
reading the crate source: `parse_special` is hardcoded `true`). Both felt right.
Say plainly when one dies; a wrong theory left standing steers the next session.

## Read the fingerprint

Garbage output has shape, and the shape names the fault:

- **One token repeating** (`uids@@@@@`, `uids!!!!!`) — the model is receiving
  garbage logits, or the sampler has nothing to choose between. Look at the
  decode/sample path, not the prompt.
- **Multilingual token salad** — numerically wrong matmuls; suspect the build,
  the quantization, or the weights.
- **Structurally plausible but wrong facts** — the model is fine and working; this
  is a capability limit, not a bug.
- **Literal markup in the UI** (`**bold**`) — a rendering gap, not a model problem.

Match the symptom to the layer before forming a theory. It is much cheaper than
bisecting.

## Trust the tool only if you've seen it work

A null result from a broken tool looks exactly like a clean result.

`otool -tV` reported zero SIMD instructions in a binary, which nearly became
evidence for a theory — but it had produced **zero lines of output**, because
disassembly needs the Xcode tools that machine lacks. Before treating "nothing
found" as a finding, confirm the tool produces output at all on a case you know
is positive.

## CI-specific: look runs up by SHA

`gh run list --limit 1` immediately after a push returns the **previous**
commit's run — GitHub often hasn't registered the new one yet. This produced two
"CI is green" reports while the suite was red, and a release tagged from a red
commit.

Always resolve by head SHA:

```
gh run list --json headSha,databaseId \
  --jq '.[] | select(.headSha|startswith("<sha>")) | .databaseId'
```

And when a job fails in seconds with **no steps executed**, it is infrastructure,
not your YAML — a platform outage, or exhausted Actions minutes. Check
[githubstatus.com](https://www.githubstatus.com) and the billing page before
debugging a workflow that never ran.

## Know when the bug is in the test

A failing test is not automatically a failing product. Two of this project's red
builds were bad assertions: a reserve test that claimed 4000 of 4096 tokens would
force eviction (96 tokens remained; three small turns legitimately fit).

When a test fails, ask which is wrong before fixing either — and if it's the
test, say so explicitly rather than quietly adjusting the number until it passes.

## Self-check before declaring a bug understood

- Did I inspect the artifact the user actually has, or only the source?
- Is there an upstream reference implementation I could compare against?
- Have I written down every hypothesis I eliminated, with its evidence?
- Did I confirm my diagnostic tools actually work before trusting their silence?
- If this was a regression: did I bisect over existing artifacts rather than guess?
- Is the fix addressing the cause, or the first symptom that reproduced?
