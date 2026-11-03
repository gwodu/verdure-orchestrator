#!/bin/sh
# The speech bake-off ([OQ-120]): speech only, every consultation, then a summary
# table (evals/summarise.py). Rows, and why:
#   whisper-base         the floor the phone ran — the baseline
#   whisper-small        the next size up, same engine, nothing new to build
#   whisper-small-kt     + the clinical keyterms list (3 consultations only: the
#                        first run showed the 40-word prompt HURTS — see [OQ-121])
#   afrispeech-medium    Intron's clinical Whisper medium, converted to ggml
#                        (3 consultations; --overcommit: ~2 GB on an 8 GB Mac)
# Run from the repo root. Existing *.score.json are skipped, so it resumes.
set -u
ALL="day1_consultation01 day1_consultation02 day1_consultation03 day2_consultation01 day2_consultation03 day3_consultation01 day3_consultation02 day4_consultation01 day4_consultation02 day5_consultation01"
THREE="day1_consultation01 day1_consultation02 day1_consultation03"
run() { # consultation tag bundle [extra args...]
  c="$1"; tag="$2"; bundle="$3"; shift 3
  [ -f "evals/out/$c.$tag.score.json" ] && return 0
  python3 -I evals/run-primock.py "$c" --bundle "$bundle" --asr-only --tag "$tag" "$@" > "evals/out/$c.$tag.log" 2>&1 || echo "FAILED $c $tag (see evals/out/$c.$tag.log)"
  echo "done $c $tag $(date +%H:%M)"
}
for c in $ALL; do run "$c" whisper-base asr-whisper-base; done
for c in $ALL; do run "$c" whisper-small asr-whisper-small; done
for c in $THREE; do run "$c" whisper-small-kt asr-whisper-small --keyterms evals/keyterms-clinical.txt; done
if [ -f "$HOME/Verdure-bisect/models/ggml-afrispeech-medium.bin" ]; then
  for c in $THREE; do run "$c" afrispeech-medium asr-afrispeech-medium --overcommit; done
fi
python3 -I evals/summarise.py
