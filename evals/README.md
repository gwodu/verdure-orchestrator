# evals — the SDK against real speech

The first measurements behind `knowledge/26`'s release gates. Everything here runs on the
founder's 8 GB Intel Mac with no network once the models are cached.

| piece | what |
|---|---|
| `fetch-primock57.sh` | fetches three PriMock57 consultations (Babylon Health, CC BY 4.0): audio outside the repo, reference transcripts into `fixtures/primock57/` |
| `score.py` (+ `test_score.py`) | word error rate against the reference, numbers kept, numbers invented in facts, real-time factor |
| `run-primock.py` | the whole SDK chain through the Python binding: device → `recommend` → `load` → `transcribe` the mixed doctor+patient audio → `extract` facts per ~60 s window → score. `--transcript FILE` skips recognition. |
| `out/` | transcripts, facts and `*.score.json` per run (git-ignored) |

```sh
sh evals/fetch-primock57.sh
CARGO_BUILD_JOBS=2 cargo build -p verdure-ffi --features llama,whisper      # ~6 min cold on the Mac
cargo run -p verdure-ffi --features bindgen --bin uniffi-bindgen -- generate --library target/debug/libverdure.dylib --language python --out-dir sandbox/py
cp target/debug/libverdure.dylib sandbox/py/
python3 -I evals/run-primock.py day1_consultation01
python3 -B -m unittest discover -s evals
```

Model files are expected in `~/Verdure-bisect/models/` (the runner copies them into the SDK's
bundle directory so no download happens); otherwise the SDK downloads them through the manifest.
Results and what they taught us: `knowledge/26`, "The first real run".
