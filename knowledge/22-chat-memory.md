# 22 — Chat memory: how Verdure remembers a conversation

**Status:** 🟡 Provisional (design settled 2026-08-12; the per-model KV cost constants are estimates awaiting on-device calibration)

Resolves the core of `[OQ-59]`. Companion to `03-model-matrix.md` (which models we run) and `18-unified-interface.md` (the chat front door). The *why* lives here; `app/src-tauri/src/llm.rs` implements it.

## The problem, stated correctly

Until 2026-08-12 the chat had **no memory at all** — and not because the context window was too small. `llm::decide()` took a single `user_message` and built a prompt of exactly `SYSTEM + that one message`. The 1024-token context sat nearly empty every turn. Ask "what did I just say?" and the model genuinely had never seen it.

So the first fix costs nothing from the memory budget: **send the turns we already have.**

## The design — three parts of one mechanism

**1. Send the transcript.** Prior turns are rendered into the model's own chat template (ChatML or Gemma — `PromptStyle` already travels with each model) ahead of the new message. Newest turns have priority.

**2. Keep the model resident.** The weights stay loaded in Tauri state across messages instead of being re-read from disk (0.4–1.4 GB) every single turn. Without this, sending history would make each message *slower*, not just smarter.

**3. Evict whole turns when the budget fills.** Oldest complete exchanges are dropped until the prompt fits. **Turn-level, never token-level:** llama.cpp's built-in context shift would cut mid-turn and leave a dangling `<|im_start|>`, corrupting the structure the model depends on. A half-turn is worse than no turn.

These ship together because they are one mechanism: (1) creates the possibility of overflow, (3) is the answer to it, (2) is what makes the whole thing affordable. Shipping (1) alone would mean an overflow either aborts the process (ggml `abort()` takes the whole GUI down — `[OQ-58]`) or silently corrupts the prompt.

**Deliberately not built: summarization.** The 2026 research consensus for long agent sessions is *anchored iterative summarization* — fold evicted turns into a running summary rather than discarding them. Correct for agents; wrong for us today, because each fold costs an extra model call, and on a 0.5B model on an 8 GB machine that is seconds of latency to save kilobytes. Revisit when users report noticing the forgetting. Tracked as `[OQ-68]`.

## Sizing the budget — arithmetic, not a guess

KV cache cost is exact and knowable:

```
bytes_per_token = 2 × n_layers × n_kv_heads × head_dim × dtype_bytes
```

(2 = one Key + one Value tensor.) Our models differ by more than 10×, which is the entire argument for computing this per model rather than hardcoding one context length:

| model | layers × kv_heads × head_dim | ≈ per token | 1024 ctx | 4096 ctx |
|---|---|---|---|---|
| Qwen2.5 0.5B | 24 × 2 × 64 | ~12 KB | ~13 MB | ~50 MB |
| Qwen2.5 1.5B | 28 × 2 × 128 | ~28 KB | ~29 MB | ~115 MB |
| Qwen3 4B | 36 × 8 × 128 | ~144 KB | ~147 MB | ~590 MB |

Computed from published architectures at f16 KV — **provisional until measured on-device** (same discipline as the transcription predictor, `19`). They live as `kv_kb_per_token` in the model registry beside `approx_mb`/`runtime_mb`, because that registry is already the place a model's device-facing costs are declared (`03`, "the in-app chat registry").

*Why constants rather than reading GGUF metadata:* the pinned `llama-cpp-2` may or may not expose the architecture fields, and an unverified API is a poor foundation. The constants are honest, documented, and calibratable; reading them from the file is a refinement, not a prerequisite.

The context we actually allocate:

```
spare        = available_ram − model weights − headroom for the OS/WebView
affordable   = spare ÷ kv_bytes_per_token
n_ctx        = clamp(affordable, MIN_CONTEXT, model's trained context)
```

Below `MIN_CONTEXT` we do not shrink further — a context too small to hold a system prompt plus one exchange is not a degraded chat, it is a broken one. Better to decline honestly, which the RAM gate already does.

## Why this is one implementation for three platforms

The mechanism is platform-neutral: it is arithmetic plus prompt assembly, and it lands in the shared Rust core. Platform difference enters at exactly one seam that **already exists** — `device::system_resources().available_ram_mb`, which each platform computes its own honest way (macOS `vm_stat`; iOS `os_proc_available_memory()`, the app's real headroom before jetsam; Android `MemAvailable` with a foreground-claimable ceiling). A tighter device gets a smaller window from the same formula. No per-platform memory code.

**The one genuine mobile risk: residency versus the OS's right to reclaim.** Holding weights resident makes a backgrounded app a more attractive kill target for iOS jetsam and Android's LMK. This is a real trade, not a hypothetical. The first version accepts it, because the failure mode is benign — the OS kills a backgrounded app, the user reopens it, and the model reloads exactly as it does today. Conversation history is unaffected (it lives in the frontend, not in the resident model). Releasing the resident model on background/suspend is the obvious refinement if this proves annoying in practice; tracked as `[OQ-69]`.

## What is deliberately *not* in the resident session

Only the **model weights** are held across turns, not the llama context/KV cache. Keeping the KV cache alive would let each turn prefill only its new tokens — the larger optimization — but `LlamaContext` borrows both the backend and the model, so storing it requires a self-referential struct (an `ouroboros`-style dependency or `unsafe`). That complexity is not justified before the simple version is measured: re-reading 0.4–1.4 GB from disk was the dominant cost, and that is what residency removes. Re-prefilling a bounded ~1000-token window is comparatively cheap. Revisit with a measurement in hand, not a guess.

## Two gates, deliberately separate (the orchestration seam)

Whether a model can be **downloaded** and whether it can be **run** are different
questions, and Verdure keeps them apart on purpose:

- **Download** depends on *disk* — a durable fact. Gate: `fits_disk`.
- **Run** depends on *memory free at that moment* — a live fact, re-evaluated on
  every message. Gate: `chat_model_choice(model_id, available_ram_mb)`.

So the picker never blocks a choice on memory. You may select any model your disk
can hold; if memory is short when you actually send a message, the backend
declines with the real numbers and what to do about it ("needs about 2.4 GB of
free memory, but only 1.9 GB is free right now"). The picker's memory note is a
*heads-up*, not a refusal — phrased as an action, because "Not enough free
memory" sitting under "7.0 GB free space" read as a contradiction (founder
report, 2026-08-21).

**Keep these separate when either side changes.** A runtime decision that
re-reads live memory per request is the foundation any future per-task model
orchestration would need (`20`); collapsing it back into a single install-time
check would have to be undone later.

## Cross-session memory (deliberately separate)

Surviving an app restart is a *different* change: persist the transcript as JSON in the app data dir and reload it at launch. No vector store, no embeddings, no RAG. It touches disk rather than the inference loop and carries its own privacy question (a conversation kept on disk is a conversation someone else with the device could read), which deserves its own decision and its own user-facing control. It is the honest precursor to the personal context core (`[OQ-56]`), not a competitor to it.

## The user-facing surface: none

There is **no context-length setting**, and this is a deliberate refusal. "Context length" is exactly the jargon knob Verdure exists to delete (`skills/design.md` — assume zero technical knowledge; `06` — honest expectations, not configuration). A user who knows what to set it to is not our floor user; one who does not would either cripple the model or push it into an OOM. The window is computed silently from the device.

If a control is ever added it should be about **behavior, not tokens** — "Remember our conversations: on/off" — which is a privacy choice the user genuinely owns and can answer. That belongs with cross-session persistence, not here.

## Open questions

- `[OQ-68]` — summarize evicted turns instead of dropping them (deferred; needs evidence users notice).
- `[OQ-69]` — release the resident model when the app is backgrounded on iOS/Android.
- `[OQ-70]` — calibrate `kv_kb_per_token` and the headroom constant against real on-device measurements.
