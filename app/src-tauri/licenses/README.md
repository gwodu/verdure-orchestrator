# Supplementary licence texts

Licence texts that the published crates do **not** ship, checked in here so
`generate-notices.py` needs no network and the attribution can be audited.

- **`llama.cpp-LICENSE`** — `llama-cpp-sys-2` vendors the entire llama.cpp
  source tree but publishes no LICENSE file for it (verified 2026-08-28 against
  crate 0.1.154: no LICENSE, no README, no copyright headers in the sources).
  llama.cpp is the engine behind Verdure's chat, so this is the single most
  important attribution we carry. Fetched from
  <https://raw.githubusercontent.com/ggml-org/llama.cpp/master/LICENSE>.

  Note the copyright reads **2023-2026**, not the 2023-2024 that whisper.cpp's
  bundled LICENSE shows — they are different files and were assumed identical
  at first. Re-fetch when bumping `llama-cpp-sys-2` rather than copying either.

whisper.cpp needs no entry: `whisper-rs-sys` ships `whisper.cpp/LICENSE`, which
the generator picks up.
