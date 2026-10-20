# Debugging

Debugging is a perfect closed world in which to apply the scientific method: the
system is deterministic (even when the model isn't), every effect has a cause,
and the answer is already inside the machine. This skill is for *locating* a
defect that has already escaped — `testing.md` is for preventing them.

The spine below is David Agans' nine rules (*Debugging: The 9 Indispensable
Rules*), because they are already a checklist and they survive translation to any
stack. Under each, the Verdure-specific practice — most of it paid for in real
time lost on this codebase. Andreas Zeller's vocabulary is worth adopting
alongside it: a **defect** causes an **infection** which **propagates** into a
**failure**. You see failures. You must find defects. Confusing the two is how
symptoms get "fixed."

## The loop

**Hypothesis → prediction → smallest discriminating experiment → observation →
refute or refine.** Write it down as you go. If you can't state what you expect
to see *before* you run the experiment, you're not debugging, you're fiddling.

Fiddling is allowed — time-box it to ~15 minutes. Past that, switch to the loop.

---

## 1. Understand the system

Read `knowledge/22` (chat memory), `knowledge/03` (the model registry boundary),
and `knowledge/21`'s current state before theorising. Verdure has four layers
that fail differently and must be named precisely:

**frontend (`main.ts`) → Tauri command (`lib.rs`) → our inference code
(`llm.rs`) → llama.cpp/whisper.cpp (C++, over FFI) → the model itself.**

Most wasted effort in this project has come from attributing a failure to the
wrong layer. The model is the *last* thing to blame, not the first.

## 2. Make it fail

A bug you cannot reproduce is a bug you cannot verify fixed. **Stimulate the
failure, don't simulate it** — reproduce the real conditions rather than a
convenient approximation.

Verdure's failures are frequently conditional on device state, so control the
variable rather than wishing it away: fill the disk, exhaust memory, deny the
microphone permission, kill the network mid-download, background the app to
provoke jetsam. We cannot observe these in the field (no telemetry, by
principle — `knowledge/06`), so we induce them.

Intermittent means there is an uncontrolled variable, not that the bug is
magic. Find it and it stops being intermittent.

## 3. Quit thinking and look — verify the artifact, not the source

**The single highest-value rule in this codebase.** When the question is "what
does the user experience?", inspect the thing the user actually has.

Six consecutive mobile builds shipped Tauri's logo while the correct icons sat in
`src-tauri/icons/`. Reading those files "confirmed" the icon was right every
time. Both were true: the icons were correct *and* never consulted, because
`gen/` is regenerated from the template on each CI run. One `unzip` of the
shipped `.ipa` ended it.

So, per surface: extract the `.ipa`/`.apk`, mount the `.dmg`, read the file out
of the bundle. For a CI result, read the log for **that commit's SHA**. For model
behaviour, run the model. For a download, check its bytes — size against source,
magic header, and whether the tail is real data rather than zeros.

*If a check would pass whether or not the bug exists, it is not a check.*

## 4. Divide and conquer

Three bisections, cheapest first.

**Over build artifacts.** CI keeps artifacts for 14 days (`retention-days: 14`) — a working binary from
any past commit, at no build cost and no local toolchain. List past builds, map
each to its commit, download from the middle, run it. Two downloads usually beat
an afternoon of reading diffs.

**Over commits.** Once a local build works, `git bisect run <script>` automates
it in O(log n); exit 125 skips a commit that won't build.

**Over inputs (delta debugging).** For a failure triggered by a large input — an
over-long context, a specific transcript — binary-search the input down to a
minimal failing case rather than reasoning about the whole thing. Zeller's
`ddmin` is the formal version; by hand, halve it repeatedly.

## 5. Change one thing at a time — and compare against a known-good

**Differential diagnosis is the workhorse here, because Verdure wraps other
people's engines.** When our wrapper produces wrong output there are exactly two
candidates — our code, or the dependency as we build it — and arguing about which
wastes days. Don't argue: **get upstream's own prebuilt binary, run it on the
same input, on the same machine, and compare.**

That single command (2026-08-21) eliminated in one shot: the machine, available
memory, disk, the model file, the quantization format, the model's size, our
conversation-memory feature, the system prompt, and the sampler configuration.
Upstream answered correctly at 42.5 tok/s from the byte-identical model file that
made our build emit `uids@@@@@`. Everything outside our binary was exonerated at
once — after two days of hypotheses.

The reference build is kept at `~/Verdure-bisect/llama-cpp-reference/` and
includes `llama-cli`, `llama-tokenize`, `llama-bench` and `llama-perplexity`.

Generalised: **hold everything constant except the component you suspect, and get
a known-good version of that component from its author.**

## 6. Keep an audit trail

See "Recording what you learn" below. The short version: record every hypothesis
you *killed*, with its evidence, in the `[OQ-XX]` entry as you go — phrased
"**eliminated — do not re-test**". A long-running bug otherwise gets
re-investigated from scratch every session.

Be as willing to kill your own theory as anyone else's. Two confident hypotheses
died in one session on `[OQ-71]` — "the K-quant kernels are broken on x86"
(disproven by the reference binary) and "our tokenizer isn't parsing special
tokens" (disproven by reading the crate source: `parse_special` is hardcoded
`true`). Both felt right. Say plainly when one dies; a wrong theory left standing
steers the next session into a wall.

## 7. Check the plug — question your assumptions, and test the tool

**A null result from a broken tool looks exactly like a clean result.**

`otool -tV` reported zero SIMD instructions in a binary and nearly became
evidence — but it had produced *zero lines of output*, because disassembly needs
Xcode tools that machine lacks. Before treating "nothing found" as a finding,
confirm the tool speaks at all on a case you know is positive.

Assumptions this project has had to re-check the hard way: that the repo's files
are the ones that shipped; that "most recent CI run" is *your* run; that a green
test suite means the code is right; that the founder's Mac is too weak to run
these models (42.5 tok/s says otherwise); that a documented constraint still
holds (the "no Xcode CLT, tight disk" rule outlived the tight disk by months).

**CI-specific:** `gh run list --limit 1` right after you open a PR, dispatch CI or push a tag returns the
*previous* run — GitHub often hasn't registered yours yet. This produced
two "CI is green" reports while the suite was red, and a release tagged from a
red commit. Always resolve by head SHA:

```
gh run list --json headSha,databaseId \
  --jq '.[] | select(.headSha|startswith("<sha>")) | .databaseId'
```

And a job that fails in seconds with **no steps executed** is infrastructure —
a platform outage or exhausted Actions minutes — not your YAML.

## 8. Get a fresh view

Explain the failure out loud, in full sentences, to someone or something that
knows nothing about it. Most stuck debugging sessions end mid-explanation.

If you are the one stuck, say so and hand over the eliminations rather than the
conclusions. `/inspect` is the structured version when the suspicion is
architectural rather than local.

## 9. If you didn't fix it, it ain't fixed

Bugs do not go away by themselves. **Prove the fix caused the recovery**: toggle
it off and confirm the failure returns. Then add the regression test — per
`testing.md`, in the same change.

And ask whether you fixed the defect or the first symptom that reproduced.

---

## The layer that can be "wrong" without any code being broken

Verdure's output is a probabilistic model, so "the answer is bad" is not
automatically a bug. Distinguishing a broken harness from a weak model is a
specific discipline, not a judgement call.

**Isolate the layer with the simplest possible case.** If a reply is wrong on
turn four, retry it as a single turn. Still wrong → the fault is not context
assembly. Our layering makes this cheap: prompt assembly, tokenization, sampling
and the model are separable, and `first_kept_turn` / `format_chat_prompt` are
pure and unit-testable without loading anything.

**Pin everything before comparing.** `temperature=0`, fixed seed, fixed sampler
params, **single sequence and fixed batch size**. Note the deep caveat: temp=0
gives *sampler* determinism (argmax) but **not numerical determinism** —
floating-point non-associativity and batch-size variance mean logits can differ
between batch shapes. Since `llm.rs` now chunks prefill by `n_batch`, that is a
live concern for us, not a theoretical one.

**Read the fingerprint of the garbage — it names the layer:**

| symptom | layer |
|---|---|
| one token repeating (`uids@@@@@`, `uids!!!!!`) | garbage logits, or a sampler with nothing to choose between → decode/sample path |
| multilingual token salad | numerically wrong matmuls → build, quantization, or weights |
| fluent but factually wrong | the model working as intended → capability limit, not a bug |
| literal markup (`**bold**`) in the UI | rendering, nothing to do with the model |
| output degrades only past N turns | context assembly or eviction |

**Fix the prompt before building an evaluator.** Many "the model is bad" failures
are unspecified requirements. If a prompt change removes the failure it was a
spec fault; if it survives, it's a genuine model limitation. (Worked example: the
2026-08-14 anti-confabulation prompt made replies *worse* on the capable 4B —
five clauses where one would do. Instruction tokens compete with the user's
question for attention.)

**Tools available today** (all in the reference build): `llama-tokenize` to
verify our prompt tokenizes to the control tokens we intend; `llama-perplexity`
with `--kl-divergence-base` to measure whether a quantization — or *our build* —
diverges from an f16 baseline; `llama-bench` for throughput comparison.

**What we do not yet have:** a golden-output eval suite, tokenizer-parity checks
in CI, or a quantization regression check. Tracked as `[OQ-74]`. Until it exists,
this section describes manual technique, not an automated gate — do not pretend
otherwise in a test report.

## The FFI boundary

Safe Rust does not segfault. If it does, the bug is in an `unsafe` block, in FFI,
or in the toolchain — in that order of likelihood.

- `RUST_BACKTRACE=1` (or `full`); `rust-lldb` formats Rust types better than gdb.
- **Sanitizers, not intuition**, for memory bugs across the boundary: ASan for
  memory errors, TSan for data races, UBSan for undefined behaviour. **Miri
  cannot cross FFI** — it interprets MIR and has no access to the C++ side, so it
  verifies our `unsafe` Rust only, never llama.cpp. That resolves the ambiguity
  in `[OQ-63]`: Miri is not a substitute for the sanitizer gap.
- **Unwinding across FFI is undefined behaviour.** A Rust panic that propagates
  into C++ is a real hazard; guard FFI entry points.
- ggml calls `abort()` on a failed allocation rather than returning an error, so
  a model fault takes the whole GUI down. Until process isolation lands
  (`[OQ-58]`), our mitigations are the available-RAM gate and file validation —
  both *avoidance*, not handling.

## Cross-platform

Verdure ships one Rust core to three targets, and **the failure is often the
build, not the code**. Two rules earned here:

- **The macOS `.dmg` is cross-compiled** (Apple Silicon runner → `x86_64`) while
  every other artifact is native. When a bug appears only on Intel macOS, that
  asymmetry is a prime suspect, and the reference-binary comparison is how you
  test it.
- **`gen/` is regenerated from the template every CI run** and nothing under it is
  committed. Anything you expect to be in the generated iOS/Android project must
  be *put there by a build step* — assume nothing carries over. This is what the
  icon bug taught, and `app/scripts/install-icons.py` exists because of it.

Android native crashes leave tombstones; `ndk-stack -sym <symbols> -dump
crash.log` symbolicates them **from a saved logcat, without the device**. The
`adb logcat --pid=$(adb shell pidof …)` loop is what root-caused the
`ndk_context` mic panic.

## Debugging what you cannot see

We have no telemetry and will not add any (`knowledge/06`). That is a real cost —
some bugs will be found more slowly than a competitor with crash reporting — and
it is accepted deliberately. The replacements:

- **Reproduce locally by induction** (rule 2) rather than observing in the field.
- **Ask for the right facts.** A useful report needs: device + OS version, app
  build number, which model was selected, exact steps, expected vs actual, and
  how often. Only ask for a field if it changes the diagnosis.
- **The user's screenshot is evidence** — the Storage screen and the model chip
  identify the build state precisely, which is how the "duplicate Whisper models"
  question was settled without any instrumentation.
- The eventual answer is an **opt-in, PII-scrubbed, on-device diagnostic bundle**
  the user reviews before sending — Signal's model, never passive collection.
  Tracked as `[OQ-75]`.

## Recording what you learn

Verdure already has four stores, and a fifth would be redundancy that drifts
(`inspection.md` treats that as a smell). So use the existing ones, with one
addition that makes them *retrievable*:

- **`knowledge/open-questions.md`** — the live investigation: hypotheses killed,
  evidence, the next decisive test. This is the working document.
- **`TEST_REPORT.md`** — the regression test that now prevents recurrence, and
  the edge case it encodes.
- **`KNOWLEDGE.md` changelog** — one dated line: what broke, what it actually
  was, what method found it.
- **`knowledge/21-handoff.md`** — only if it's still open at session end.

**The addition: give every non-trivial bug a symptom fingerprint** — the literal
string a future reader would search for (`uids@@@@@`, `ran out of context`,
`android context was not initialized`). Put it in the first line of the `[OQ]`
entry. Fingerprints are what make a past investigation findable by someone
experiencing the symptom rather than someone who already knows the cause.

**Threshold:** any bug that took more than an hour, or that recurred, gets
written up and gets a regression test. No exceptions — that is the bar that keeps
the record honest rather than selective.

**Blamelessly.** Record what the system allowed, not who erred. Most of this
file's rules exist because a check was structurally incapable of catching a
defect, not because someone was careless — and the fix is always the check.

## Self-check before declaring a bug understood

- Can I reproduce it reliably, on demand?
- Did I inspect the artifact the user has, or only the source?
- Is there an upstream reference implementation I could have compared against?
- Have I written down every hypothesis I killed, with its evidence?
- Did I confirm my diagnostic tools work before trusting their silence?
- Did I change one thing at a time?
- If I toggle the fix off, does the failure return?
- Is there a regression test, and a symptom fingerprint someone could search for?
