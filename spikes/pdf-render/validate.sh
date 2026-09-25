#!/usr/bin/env bash
# Spike C pass criterion: sample SDS validates as PDF/A-2b and renders correctly.
set -euo pipefail
cd "$(dirname "$0")"
cargo run -q --release --bin render-sample
verapdf --flavour 2b --format text out/sample-el.pdf | tee out/verapdf-report.txt
grep -q "^PASS" out/verapdf-report.txt
pdftoppm -r 60 -png out/sample-el.pdf out/page
pdfinfo out/sample-el.pdf | grep -E "Pages|PDF version|Title"
echo "PDF/A-2b: PASS"
