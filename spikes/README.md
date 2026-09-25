# Proof-of-concept spikes (THROWAWAY)

Code here answers the go/no-go questions in
`docs/superpowers/specs/2026-09-25-sds-core-design.md` §4.1. It is deliberately
minimal and is **not** the start of the product. Do not import, copy, or build on it;
the product core is written fresh from the spec. Findings live in
`docs/spikes/2026-09-25-poc-results.md`.

| Dir | Spike |
|---|---|
| `smb-lock/` | A — SQLite + lock file on SMB (`smbstress` CLI) |
| `ffi-core/` + `winui/` | B — Rust core ↔ WinUI 3 via UniFFI; D — printing |
| `pdf-render/` | C — Typst → PDF/A-2b |
