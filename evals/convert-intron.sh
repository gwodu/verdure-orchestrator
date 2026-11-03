#!/bin/sh
# Convert intronhealth/afrispeech-whisper-medium-all (Hugging Face format,
# Apache-2.0) to whisper.cpp's ggml f16 so the SDK's own engine can run it
# ([OQ-120]). Needs: ~/Verdure-bisect/intron/ (the HF files),
# ~/Verdure-bisect/convert/ (whisper.cpp's convert-h5-to-ggml.py + OpenAI's
# mel_filters.npz), ~/Verdure-bisect/venv (python3.11, torch 2.2.2,
# transformers 4.46). Output: ~/Verdure-bisect/models/ggml-afrispeech-medium.bin
set -eu
B="$HOME/Verdure-bisect"
mkdir -p "$B/convert/whisper/assets" "$B/convert/out"
cp -n "$B/convert/mel_filters.npz" "$B/convert/whisper/assets/" 2>/dev/null || true
"$B/venv/bin/python" -I "$B/convert/convert-h5-to-ggml.py" "$B/intron" "$B/convert" "$B/convert/out"
mv "$B/convert/out/ggml-model.bin" "$B/models/ggml-afrispeech-medium.bin"
shasum -a 256 "$B/models/ggml-afrispeech-medium.bin"
ls -l "$B/models/ggml-afrispeech-medium.bin"
