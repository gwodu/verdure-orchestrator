# The Verdure SDK crates

Read `knowledge/26-sdk.md` first — what this is for, who it is for, and what it
deliberately does not do. Then `skills/architecture.md` for the shape.

| crate | role | I/O |
|---|---|---|
| `verdure-core` | the pure domain: policy, tiers, bundles, licences, engine ports, `extract()` | none |
| `verdure-grammar` | JSON Schema → GBNF, and the validator over the same subset | none |
| `verdure-hw` | reads the device honestly → `DeviceProfile` | OS probes |
| `verdure-download` | **the only crate that opens a socket**: signed manifests, resumable verified downloads | network, disk |
| `verdure-vault` | AES-256-GCM files, crypto-shred delete | disk |
| `verdure-rag` | chunker, SQLite FTS5 + embeddings, fused ranking | disk |
| `verdure-asr` | segmenter, resampling, keyterms/replacements; whisper.cpp behind `--features whisper` | mic data in, text out |
| `verdure-llm` | prompt framing, context arithmetic; llama.cpp behind `--features llama` | model files |
| `verdure-ffi` | the public surface (UniFFI) — the single source of truth for every binding | — |
| `verdure-cli` | `verdure` developer tool: capabilities, licences, manifests, grammars, dry-run recommend | — |

```sh
cargo test --workspace                       # the pure suite, seconds, no models
cargo run -p verdure-cli -- capabilities     # what this machine is
cargo run -p verdure-cli -- grammar bundles/manifests/schema-example-chw.json
```

The engines (`--features llama,whisper`) compile llama.cpp and whisper.cpp — a
multi-gigabyte debug build. On the founder's disk-tight Mac that build is **not**
run locally (knowledge/26, `[OQ-112]`); CI's `sdk` job compile-checks it on Linux.
