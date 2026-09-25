#!/usr/bin/env bash
# Spike B criterion: Windows binaries cross-built on macOS must link and later load on Windows.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$(brew --prefix llvm)/bin:$PATH"
export XWIN_ACCEPT_LICENSE=1   # cargo-xwin downloads the Microsoft CRT/SDK headers
for pair in x86_64-pc-windows-msvc:win-x64 aarch64-pc-windows-msvc:win-arm64; do
  triple=${pair%%:*}; rid=${pair##*:}
  cargo xwin build --release --target "$triple" -p spike-ffi -p smbstress
  mkdir -p "winui/native-from-mac/$rid"
  cp "target/$triple/release/spike_ffi.dll" "target/$triple/release/smbstress.exe" "winui/native-from-mac/$rid/"
done
file winui/native-from-mac/*/*
