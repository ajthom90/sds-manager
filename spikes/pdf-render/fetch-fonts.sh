#!/usr/bin/env bash
# Noto Sans (SIL OFL 1.1) — Latin, Greek and Cyrillic cover every EU language.
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p assets/fonts
base=https://github.com/notofonts/notofonts.github.io/raw/main/fonts/NotoSans/hinted/ttf
for f in NotoSans-Regular NotoSans-Bold; do
  curl -sfL -o "assets/fonts/$f.ttf" "$base/$f.ttf"
done
ls -l assets/fonts
