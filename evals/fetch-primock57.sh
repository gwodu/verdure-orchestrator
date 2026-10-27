#!/bin/sh
# Fetch a handful of PriMock57 consultations (Babylon Health, CC BY 4.0;
# https://github.com/babylonhealth/primock57) for the SDK's speech evaluation.
# Audio goes OUTSIDE the repo (it is ~15 MB per channel); the reference
# transcripts (TextGrid) are small and committed next to this script.
#
#   sh evals/fetch-primock57.sh [DEST]   (default ~/Verdure-bisect/fixtures/primock57)
set -eu
DEST="${1:-$HOME/Verdure-bisect/fixtures/primock57}"
HERE="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$DEST"
for c in day1_consultation01 day1_consultation02 day2_consultation03; do
  for side in doctor patient; do
    f="${c}_${side}"
    [ -f "$DEST/$f.wav" ] || curl -sL -o "$DEST/$f.wav" "https://media.githubusercontent.com/media/babylonhealth/primock57/main/audio/$f.wav"
    [ -f "$HERE/fixtures/primock57/$f.TextGrid" ] || curl -sL -o "$HERE/fixtures/primock57/$f.TextGrid" "https://raw.githubusercontent.com/babylonhealth/primock57/main/transcripts/$f.TextGrid"
  done
done
ls -l "$DEST"
