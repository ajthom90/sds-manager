# SDS Manager — System Architecture & Core Design

- **Date:** 2026-09-25
- **Status:** Draft for review
- **Scope of this spec:** the overall system architecture and roadmap (§1–§4), and the detailed design of
  **Sub-project 1: the Rust core** (§5–§11). Every later sub-project gets its own spec → plan → implementation cycle.
- **Research basis:** [`docs/research/2026-09-25-sds-regulations.md`](../../research/2026-09-25-sds-regulations.md).
  Claims marked there as *[unverified]* or *(from memory)* are **not** treated as facts in this spec; see §11.

---

## 1. Purpose

A **free, open-source** desktop program that companies use to author, classify, approve, and publish **Safety Data
Sheets (SDS)** for their chemical products, compliant with:

| Profile family | Regulation |
|---|---|
| United States | OSHA Hazard Communication Standard, 29 CFR 1910.1200 (2024 rule) |
| Canada | WHMIS — Hazardous Products Regulations (2022 amendments) |
| European Union | REACH Annex II (Reg. 2020/878) + CLP (Reg. 1272/2008 incl. 2023/707, ATPs) |
| Great Britain | GB CLP + UK REACH (2015/830-style Annex II) |
| Australia | WHS Regulations, GHS Rev 7 |

**Success looks like:** a small company with no in-house regulatory software installs the app on Mac or Windows,
points it at a database file (local, or on a shared network drive for a small team), enters its products'
ingredients, gets an automatically calculated GHS classification with a transparent derivation, and publishes
print-ready PDF SDSs in the languages each market requires.

The software **does not certify legal compliance**. The app shows a clear disclaimer on first run and in About; the
responsible person reviews and approves every SDS.

## 2. Decisions made during brainstorming

| Topic | Decision |
|---|---|
| Jurisdictions | All five profile families supported by the architecture from day one |
| UI | **Truly native per platform**: SwiftUI (macOS) and WinUI 3 (Windows) |
| Windows architectures | **x64 and arm64** |
| Minimum OS | macOS 14; Windows 10 22H2 / Windows 11 |
| Shared logic | One **Rust core** (storage, rules, classification, rendering) used by both apps via FFI |
| Storage | One SQLite database file; local (single-user) or on an SMB share (small team, ≈ ≤10 concurrent users) |
| Concurrency | App-level write lock + record **check-out** (others view read-only) |
| Classification | **Automatic mixture classification** with recorded, justified overrides |
| Ingredient data | **Bundled starter library** from public/reusable sources + optional online PubChem lookup |
| Languages | Official phrases auto-filled in every language; free text stored per language; publishing blocked when a required translation is missing; standard-sentence library (EN + FR first) |
| Users | Identified by OS login name (no passwords); roles Viewer / Author / Approver / Admin (new users on a shared DB start as Viewer); approval workflow; audit log |
| Output | PDF (PDF/A-2b) export and printing |
| License | Open source, permissive: **MIT OR Apache-2.0** dual license (Rust-ecosystem convention; compatible with Typst (Apache-2.0) and Noto fonts (OFL)) |

## 3. System architecture

```
┌────────────────────────────┐      ┌─────────────────────────────┐
│ macOS app (SwiftUI)        │      │ Windows app (WinUI 3, C#)   │
│  PDFKit preview & print    │      │  Windows.Data.Pdf preview,  │
│                            │      │  PrintManager print         │
└─────────────┬──────────────┘      └──────────────┬──────────────┘
              │ UniFFI Swift bindings               │ uniffi-bindgen-cs C# bindings
              └──────────────────┬──────────────────┘
                     ┌───────────▼───────────┐
                     │   Rust core (sds-*)   │
                     │ session · workflow ·  │
                     │ classify · render ·   │
                     │ store · profiles      │
                     └─────┬───────────┬─────┘
                           │           │
          ┌────────────────▼──┐     ┌──▼─────────────────────────┐
          │ reference.sdsref  │     │ company.sdsdb (+ .writelock,│
          │ read-only, ships  │     │ backups/) — local disk or   │
          │ with the app      │     │ SMB share                   │
          └───────────────────┘     └─────────────────────────────┘
```

- **All business logic lives in the core.** The apps are UI shells: they display data, collect edits, and show,
  export and print PDFs that the core produces.
- **FFI:** UniFFI, pinned to the version supported by `uniffi-bindgen-cs` (currently UniFFI 0.31.x with
  uniffi-bindgen-cs v0.11.0, released 2026-06-23). **Fallback** if the C# generator stalls: a hand-written C ABI
  (`extern "C"`) consumed via P/Invoke (C#) and a bridging header (Swift).
- **Build hosts:** the Rust core and macOS app build on macOS (Apple Silicon). Rust cross-compiles to
  `x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` (via `cargo-xwin`). The WinUI 3 app requires Windows (a
  Windows 11 ARM VM for development; GitHub Actions `windows-latest` and `windows-11-arm` runners for CI and releases).

## 4. Sub-projects and roadmap

| # | Sub-project | Deliverable | Spec |
|---|---|---|---|
| 0 | **Proof-of-concept spikes** (throwaway code) | Go/no-go answers for the four riskiest assumptions (§4.1) | Short plan only |
| 1 | **Rust core** | Crates in §5–§10, `sdsctl` CLI, reference-file builder, fully specified US-HCS-2024 profile | **This spec** |
| 2 | **Regulatory data pipeline & library** | Importers that turn OSHA App C, CLP Annexes III/V/VI, HPR, GB MCL, OSHA/NIOSH limits, DOT table, regulatory lists into reference data with provenance/licence metadata; profile data for CA/EU/GB/AU; standard-sentence library; PubChem lookup | Spec 2 |
| 3 | **macOS app** | SwiftUI app on the core | Spec 3 |
| 4 | **Windows app** | WinUI 3 app on the core, x64 + arm64 | Spec 4 |
| 5 | **Packaging & releases** | Signed/notarized DMG; signed MSIX (and/or MSI) for x64 + arm64; GitHub Actions release pipeline; update check | Spec 5 |

Order: 0 → 1 → 2 → (3 ∥ 4) → 5. Sub-projects 3 and 4 may start once the core's FFI API (§9) is stable.

### 4.1 Sub-project 0 — spikes and pass criteria

| Spike | Pass criterion | If it fails |
|---|---|---|
| **A. SQLite + lock-file on SMB** | 3+ clients (at least one macOS, one Windows) run randomized read/write/check-out load against one `.sdsdb` for ≥ 60 minutes, **on each of: a Windows-hosted share and a Samba (Linux/NAS) share** (create-exclusive, rename and delete of open files behave differently on each). Passes when: `PRAGMA integrity_check` = ok; no lost committed writes; **readers never observe a state that was not a committed transaction** (checked with an invariant, e.g. rows carrying a checksum of their transaction's full write set); stale-lock recovery works when a client is killed mid-transaction | Switch storage design to a folder of per-user change files ("change-log folder") before core work starts; re-spec §6 |
| **B. Rust core ↔ WinUI 3** | A UniFFI-exported object with methods, records, errors and a callback interface is called from a WinUI 3 app on Windows arm64 (native) and x64 (under emulation); binaries cross-built from macOS link and load | Use the C ABI fallback (§3) |
| **C. Typst PDF rendering** | Embedded Typst renders a sample SDS with Latin, Greek and Cyrillic text, a pictogram row, a multi-page table, "Page X of Y" headers, and outputs **PDF/A-2b** that passes veraPDF validation; identical input → byte-identical output | Evaluate `krilla` (direct PDF) or `printpdf` + own layout |
| **D. Printing on Windows** | A multi-page PDF prints with correct page count and scaling via WinUI 3 `PrintManager` (interop) on x64 and arm64 | Print via WebView2's built-in PDF viewer |

---

## 5. Core — crate layout

| Crate | Responsibility | Depends on |
|---|---|---|
| `sds-model` | Plain data types shared by all crates (IDs, enums, records). No I/O. | — |
| `sds-profiles` | Profile schema; loads and validates jurisdiction profiles from the reference file | `sds-model` |
| `sds-classify` | Pure, deterministic classification engine and label-element derivation | `sds-model`, `sds-profiles` |
| `sds-store` | SQLite access, schema migrations, write-lock protocol, check-outs, sessions, backups, audit writes | `sds-model` |
| `sds-render` | Document assembly (product + profile + language → document tree) and Typst → PDF/A | `sds-model`, `sds-profiles` |
| `sds-core` | Session facade, workflow, validation, permissions, revision snapshots, change notification | all of the above |
| `sds-ffi` | UniFFI interface definitions and conversions; no logic | `sds-core` |
| `sds-cli` | `sdsctl` command-line tool for testing and scripting | `sds-core` |
| `sds-refbuild` | Compiles `profiles/*.toml` + `data/**` into `reference.sdsref` | `sds-model`, `sds-profiles` |

Toolchain: latest stable Rust, pinned via `rust-toolchain.toml`; `rusqlite` with the `bundled` feature (identical
SQLite version on every platform); UUIDv7 for all IDs; `typst` + `typst-pdf` crates (0.15.x) for rendering.

## 6. Core — data model

### 6.1 Two database files

**`reference.sdsref`** — read-only SQLite file shipped with (and updated with) the app. Built by `sds-refbuild`.
Contains: `meta` (data version, build date, source manifest), jurisdiction profiles, hazard class/category
definitions, phrases (code × language × profile family), pictograms (SVG), standard sentences, substance library,
regulatory lists, transport table. **Every row carries `source_id`**, referencing a `sources` table with citation
URL, retrieval date and licence.

**`company.sdsdb`** — the company's working database; may live on a network share. All IDs UUIDv7; all timestamps
UTC (ISO-8601); calendar dates (e.g. revision date) stored as `YYYY-MM-DD` interpreted in the company time zone
setting.

### 6.2 Company database entities

- **`db_meta`**: schema version, database UUID, `min_reference_version` (§6.3), created-at.
- **`company_settings`**: company time zone, logo (PNG/SVG blob), enabled profiles, enabled output languages
  per profile, `approval_required` (bool; asked when the database is created — "Will more than one person use this
database?" — and changeable later by an Admin), Canada output mode
  (`bilingual_combined` | `separate`), optional Section 16 disclaimer text (per language).
- **`supplier_entity`**: legal name, address, phone, email, VAT number (EU, used for UFI), and which profiles it
  serves (e.g. US importer, Canadian supplier, EU supplier).
- **`emergency_contact`**: phone number, hours, language notes, per profile.
- **`user`**: OS login name (unique, case-insensitive), display name, roles (set of `Viewer`, `Author`, `Approver`, `Admin`),
  active flag. The first user to create a database becomes Admin. Unknown OS users opening a shared DB are added with
  the `Viewer` role and see a notice that an Admin must grant Author/Approver rights to edit; the audit log records
  their first access.
- **`audit_log`** (append-only): timestamp, user, machine, session ID, entity type, entity ID, action, JSON diff
  (before/after of changed fields).
- **`substance`** (company-owned): identifiers (CAS RN — user-entered, EC number, index number, PubChem CID,
  InChIKey), names (localized), `library_origin_id` (nullable; set when copied from a bundled library substance),
  flags (SVHC, ED, PBT/vPvB, PMT/vPvM, nanoform), source/notes.
  - **`substance_classification`**: per classification scheme (e.g. `GHS-US-self`, `EU-harmonised`, `EU-self`,
    `GB-MCL`, `CA-self`, `AU-self`): hazard class, category, route (where relevant), specific concentration limits,
    M-factors (acute/chronic), ATE per route, source.
  - **`exposure_limit`**: jurisdiction/source (OSHA PEL, Cal/OSHA, NIOSH REL, ACGIH TLV — **user-entered only**,
    EU IOELV, national), type (TWA/STEL/Ceiling), value, unit, notes, source.
  - **`substance_property`**: property key, value, unit, method, source.
  - Bundled library substances are never edited in place; "Edit" creates a company copy linked via
    `library_origin_id`, so library updates never silently overwrite company data. The UI can show "library has a
    newer classification for this substance" as a review prompt.
- **`product`**: name, product code (unique), synonyms/other identifiers, identified uses, uses advised against,
  physical form, `is_substance` flag (single-substance product).
  - **`composition_line`**: substance, concentration as exact % or range (min–max), trade-secret flag, generic
    chemical name for trade-secret disclosure, per-profile disclosed range (validated against that profile's
    permitted ranges).
  - **`product_property`**: the 18 physical/chemical properties (OSHA App D §9 / REACH 9.1–9.2) plus any
    classification-relevant test data (flash point, initial boiling point, pH, kinematic viscosity at 40 °C,
    particle characteristics, measured ATEs, etc.). Each: value, unit, method, or explicit state `no_data` /
    `not_applicable` (with optional reason).
  - **`transport_entry`**: per mode (`DOT`, `ADR`, `RID`, `ADN`, `IMDG`, `IATA`, `ADG`): UN number, proper shipping
    name, class, subsidiary risks, packing group, environmental hazard/marine pollutant, special precautions, IMO bulk.
  - **`product_market`**: per profile/market: UFI (EU), market-specific supplier entity, notes.
- **`localized_text`**: `(owner_type, owner_id, field_key, language)` → text, or reference to a standard sentence
  (`sentence_id`) whose translations come from the reference file. `field_key` values are defined by the profiles'
  section templates (e.g. `first_aid.inhalation`, `handling.precautions`).
- **`classification_result`**: per product × profile: engine version, reference data version, input hash,
  calculated classifications (class, category, route), **derivation trace** (structured steps), label elements
  (pictograms, signal word, H/EUH/supplementary statements, proposed P-statements), user's P-statement selection.
  - **`classification_override`**: target (add/remove/change a class/category, or a label element), reason
    (required, free text), kind (`expert_judgement`, `mixture_test_data`, `bridging_principle` — with referenced
    source product, `other`), user, timestamp.
- **`sds_document`**: per product × profile: SDS number (default `{product code}-{profile code}`), current published
  revision pointer.
  - **`sds_revision`**: integer version (1, 2, 3…), status (§8.3), created/submitted/approved/published timestamps
    and users, revision date, first-compilation date (copied from version 1), change summary (per language;
    feeds EU Section 16 "changes flagged"), reviewer comments.
  - **`sds_revision_snapshot`** (created at publish, immutable): full JSON of every input used (product,
    composition, substances, classification, texts, company/supplier data, profile ID + version, reference data
    version, engine version), the exact phrase texts used, and the rendered PDFs (one per output document) with
    SHA-256 hashes.
- **`checkout`**, **`checkout_draft`**, **`session`**: see §7.
- **`revision_trigger`**: product × profile flagged because an input changed after the last publish (e.g. an
  ingredient's classification changed), with the detected change and the profile's revision deadline.

### 6.3 Version gating on a shared database

- `db_meta.schema_version` — a client whose code supports a lower schema version opens the DB **read-only** and
  shows "update required".
- `db_meta.min_reference_version` — raised when a client with newer reference data performs a write. Clients with
  older reference data open read-only. This prevents two app versions from classifying the same product differently.
- A client with a *newer* schema version migrates the DB (§7.6).

## 7. Core — storage, locking, and multi-user

### 7.1 Files

```
<dir>/company.sdsdb             SQLite database; journal_mode=DELETE, synchronous=FULL, foreign_keys=ON
<dir>/company.sdsdb.writelock   present only while a client holds the write lock
<dir>/backups/                  rotating backups
```

WAL mode is **never** used (it does not work on network filesystems). The same code path serves local and network
locations.

### 7.2 Write-lock protocol

- **Acquire:** create `company.sdsdb.writelock` with create-exclusive semantics (`O_CREAT|O_EXCL` / Windows
  `CREATE_NEW`). Contents (JSON): user, machine, PID, session ID, acquired-at, heartbeat counter.
- **Wait:** if the file exists, retry with jittered backoff (50 ms → 1 s), up to a caller-specified timeout
  (default 10 s), then return `WriteLockBusy { holder }`.
- **Hold:** every write — saves, check-outs, check-ins, heartbeats, workflow transitions, audit rows — occurs inside
  exactly one SQLite transaction (`BEGIN IMMEDIATE … COMMIT`) while holding the write lock. SQLite's own locking
  remains active as a second layer. Operations lasting longer than 5 s (imports, migrations, backups) increment the
  heartbeat counter at least every 5 s.
- **Release:** delete the lock file after `COMMIT` (or rollback).
- **Stale detection:** a waiting client reads the lock file; if its `(session ID, heartbeat counter)` is unchanged
  for **30 s measured on the waiter's own monotonic clock**, the lock is stale. (Wall-clock timestamps from other
  machines are never compared, avoiding clock-skew errors.)
- **Stale break:** rename the lock file to `company.sdsdb.writelock.stale-<uuid>` (atomic; only one breaker wins),
  record an audit entry, delete the renamed file, then retry acquisition. SQLite's hot-journal rollback restores
  consistency if the dead holder left a journal.

### 7.3 Reads

Reads use short SQLite read transactions without the app lock. Because SQLite's shared/exclusive locks cannot be
trusted on SMB, and in rollback-journal mode a writer modifies pages in place, a reader could otherwise see a mix of
old and new pages **without any error**. Every read transaction is therefore **validated**:

1. Before `BEGIN`: if `company.sdsdb.writelock` exists, wait (same backoff as §7.2); otherwise read the file change
   counter (header offset 24) directly from the file, bypassing SQLite's cache.
2. Run the read transaction.
3. After it ends: if the write-lock file now exists or the change counter differs from step 1, discard the result
   and retry.

`SQLITE_BUSY`, `SQLITE_CORRUPT` or I/O errors during a read are also retried with backoff (up to 5 attempts in total)
before surfacing `Io`/`Corrupt`. Clients poll the change counter every 3 s while idle; a change triggers a change
notification (§9.3).

### 7.4 Check-out (user-facing record lock)

- Editable units: a **product** (including its composition, texts, classification and draft SDS revisions) and a
  **company substance**.
- Opening for edit inserts `checkout(entity_type, entity_id, user, machine, session_id, acquired_at,
  heartbeat_counter)`. Others see "Being edited by <display name> on <machine> since <time>" and get read-only
  views.
- The holder increments the heartbeat every 60 s. A check-out whose heartbeat has not changed for **10 minutes**
  (observer's monotonic clock) may be taken over by any user; an Admin may break any check-out at any time. Both are
  audited, and the previous holder's session is told on its next write attempt (`CheckedOut` error with the new
  holder).
- Edits are held in memory and in a **local recovery file** (per user profile, not on the share, updated every 30 s).
  **Save** and **Check-in** write the edits to the live records. **Autosave** (every 2 minutes) writes the pending
  edits as a JSON blob to a `checkout_draft` row attached to the check-out — not to the live records — so the work
  survives a local machine failure. Other users always see the **last explicitly saved** state. Taking over a stale
  check-out offers the new holder the orphaned draft to apply or discard. On startup, leftover local recovery files
  are offered for restore.

### 7.5 Sessions and network loss

- `session`: session ID, user, machine, app version, opened-at, heartbeat counter (every 60 s). Used to show "who is
  connected" and to gate migrations. Sessions whose heartbeat is unchanged for 5 minutes are considered gone.
- If the share becomes unreachable, the session switches to **read-only** mode (`ReadOnly { NetworkLost }`), keeps
  unsaved edits in the recovery file, and retries connection every 10 s.
- **Cloud-sync folders** (OneDrive, Dropbox, Google Drive, iCloud Drive, Box) are detected by path and known
  markers; opening or creating a DB there shows a strong warning that sync tools can corrupt live database files.
  Supported locations: local disks and SMB2/3 network shares.

### 7.6 Migrations and backups

- Migrations run only while holding the write lock with no other live sessions (or when an Admin forces it), and
  always after an automatic backup.
- **Backups** use SQLite's online backup API while holding the write lock, into
  `backups/company-YYYYMMDD-HHMMSS.sdsdb`: on the first open each calendar day, before migrations, and on demand
  ("Back up now"). Retention: 14 most recent daily + 8 weekly. Each backup is followed by `PRAGMA quick_check` on
  the copy; failure raises an alert.
- **Restore** (Admin only): requires no other live sessions; the current file is itself backed up first.

## 8. Core — profiles, classification, validation, workflow

### 8.1 Jurisdiction profiles

Profiles are versioned TOML files in `profiles/` (e.g. `US-HCS-2024.toml`), compiled into the reference file. A
profile declares:

- **Identity:** code, name, regulation citations, effective-from / effective-until dates, GHS revision basis.
- **Hazard classes and categories in scope**, including jurisdiction-specific ones (US: HNOC, combustible dust,
  simple asphyxiants, no aquatic; CA: biohazardous infectious materials, PHNOC/HHNOC; EU: ED/PBT/PMT and EUH
  statements).
- **Rule parameters:** cut-off values and generic concentration limits per class/category, ATE conversion table,
  summation thresholds, aquatic multipliers. **Every numeric parameter carries a `cite` field** referencing the
  regulation paragraph it was verified against.
- **Ingredient classification source order** (e.g. EU: `EU-harmonised` then `EU-self`; US: `GHS-US-self`, with
  `EU-harmonised` offered as an unverified suggestion).
- **Label rules:** pictogram precedence, signal word precedence, statement-supersession rules, P-statement
  selection matrix, statement-count guidance.
- **SDS template:** ordered sections and subsections (number, heading key), mandatory flags, and bindings to data
  fields and `field_key`s.
- **Languages:** required language sets per market (all-of / any-of).
- **Trade-secret ranges**, **revision deadline** (US 3 months, CA 90 days, EU without delay, AU 5-year review),
  **Section 15 regulatory lists** to screen.

**In this sub-project, `US-HCS-2024` is fully specified** (all classes, parameters verified and cited, full
template). The CA, EU, GB and AU profile files are created with their templates and structure, but their numeric
parameters and phrase data are completed and verified in Sub-project 2. The profile schema must be able to express
all five families; a schema-validation test loads skeleton profiles for all of them.

### 8.2 Classification engine (`sds-classify`)

Pure, deterministic, no I/O. Formulas in code; thresholds from the profile. **Single-substance products**
(`is_substance`) skip the mixture rules: the substance's own resolved classification passes through, followed by
label derivation (step 5). For mixtures, input: composition (concentrations as exact or ranges — how a range
enters the calculation, e.g. upper bound, is a profile parameter with `cite: needed` until verified against each
regulation's guidance),
ingredient classifications resolved via the profile's source order, product test data, and overrides. Output:
classifications, label elements, Section 2 notes, and a structured **derivation trace** (each step: rule ID, cited
parameter, inputs, arithmetic, result).

Steps:

1. **Physical hazards:** flammable-liquid category computed from flash point and initial boiling point; other
   physical classes set from user-entered test results/classifications.
2. **Health hazards:** acute toxicity per route by the ATE additivity formula (including the unknown-toxicity
   adjustment above the profile's threshold); skin corrosion/irritation and serious eye damage/irritation by
   summation (a pH ≤ 2 or ≥ 11.5 raises a required user decision); respiratory/skin sensitization, germ cell
   mutagenicity, carcinogenicity, reproductive toxicity (incl. lactation), STOT SE/RE by cut-offs; aspiration
   hazard by concentration + kinematic viscosity.
3. **Environmental hazards** (profiles that include them): aquatic acute/chronic by the summation method with
   M-factors.
4. **Profile-specific extras:** e.g. EU ED/PBT/PMT classes at the profile's threshold, EUH208 ("contains <sensitizer>")
   when a sensitizer is present above the profile's fraction of its limit; specific concentration limits and
   M-factors take precedence over generic limits.
5. **Label derivation:** pictograms with precedence rules, signal word, statement de-duplication/supersession,
   proposed P-statements from the selection matrix (user trims/selects; count guidance shown as a warning).
6. **Section 2 notes:** e.g. "x % of the mixture consists of ingredient(s) of unknown acute toxicity".

Rules the engine does **not** automate (recorded as overrides instead): bridging principles, weight-of-evidence
judgements, physical hazards requiring test interpretation, HNOC/PHNOC/HHNOC.

Measured whole-mixture data (e.g. a tested LD50) replaces the calculated result for that endpoint, recorded as a
`mixture_test_data` override.

### 8.3 SDS workflow

States of an `sds_revision`: **Draft → In Review → Approved → Published → Superseded**, plus **Rejected** (returns
to Draft with reviewer comments).

| Transition | Who | Conditions |
|---|---|---|
| Draft → In Review | Author | Validation has no blocking errors |
| In Review → Approved | Approver other than the submitting author | Validation has no blocking errors; classification is current (input hash matches) |
| In Review → Rejected → Draft | Approver | Comment required |
| Approved → Published | Author, Approver or Admin | Renders final PDFs, sets revision date (default today), writes the immutable snapshot; previous published revision → Superseded |
| Draft → Published | Author | Only when `approval_required = false` (records the author as approver) |

A new revision is created by copying the current published revision's inputs into a new Draft (version + 1).
Revision triggers (§6.2) list products whose published SDS is out of date with the deadline from the profile.

**Permissions** are enforced in the core (Admin: settings, users, lock breaking, restore; Approver: approve/reject;
Author: edit, submit, publish-approved; Viewer: browse products and substances, preview, export and print
**published** SDSs only — no check-outs, no drafts; attempts return `ReadOnly { NoPermission }`). Because the DB is a file readable by anyone with share access, roles are
an accountability mechanism, not security; the spec and UI say so.

### 8.4 Validation

`validate(product, profile)` returns issues with severity **blocking** or **warning**, each with a location (section,
field, language) and message key. Rules come from the profile plus generic checks. Examples:

- Blocking: mandatory field empty; required language translation missing; trade-secret range not permitted by
  profile; required supplier/emergency contact missing; EU hazardous mixture without UFI; classification stale
  relative to inputs; unresolved required user decision (e.g. pH flag); composition impossible (sum of exact values
  and range minimums > 100 %).
- Warning: composition incomplete (exact values do not total 100 % ± 0.5, or range maximums total < 100 %).
- Warning: more P-statements than the profile's guidance; an ingredient classification taken from an unverified
  suggestion source; exposure limits absent in Section 8 for an ingredient with a hazard classification.

### 8.5 UFI

The core provides UFI generation and validation (from VAT number + formulation number) per ECHA's published UFI
algorithm. The algorithm is implemented from ECHA's UFI developer documentation and tested against ECHA's published
examples.

## 9. Core — rendering, PDF, and the FFI API

### 9.1 Rendering pipeline (`sds-render`)

1. **Assemble:** product + classification + profile template + language → fully resolved **document tree**
   (sections → subsections → blocks: paragraph, key-value table, composition table, pictogram row, statement list,
   transport table). Canada: one combined bilingual document or two separate documents per company setting.
2. **Validate** (§8.4); previews render regardless and show issues; final rendering requires no blocking errors.
3. **Render:** the document tree is serialized to JSON and passed to fixed, bundled Typst templates as data (via
   Typst's data inputs) — **user text is never spliced into Typst source**. Fonts: bundled Noto Sans / Noto Sans
   Mono (Latin, Greek, Cyrillic). Every page: product name, SDS number, version, revision date, "Page X of Y"; EU
   profiles also print the first-compilation date and revision/version line on page 1 and "End of safety data sheet"
   on the last page. Optional company logo.
4. **Output:** **PDF/A-2b**; the PDF's metadata date is set to the revision date so identical inputs produce
   byte-identical output (within a Typst version). Drafts carry a diagonal **DRAFT** watermark.

Published PDFs are stored in the revision snapshot; re-export of any published revision returns the stored bytes.
Export file names: `{ProductCode}_{ProfileCode}_{lang}_v{version}.pdf` (`lang` = `en-fr` for a combined Canadian
document).

### 9.2 Printing and preview (in the apps, specified here for the API contract)

The core returns PDF bytes; the apps preview (macOS: PDFKit; Windows: `Windows.Data.Pdf`) and print (macOS: PDFKit
print operation; Windows: WinUI 3 `PrintManager`, fallback WebView2 PDF viewer per spike D). Batch export writes all
current published PDFs to a chosen folder.

### 9.3 FFI API (`sds-ffi`)

Coarse-grained, synchronous (apps call from background threads), object-oriented:

- `Session::open(db_path, reference_path, os_user, machine) -> Session`; `Session::create_database(...)`.
- Products, substances, library search, SDS documents/revisions: `list`, `get`, `checkout`, `save`, `checkin`,
  `create`, `delete` (soft delete; only unpublished items).
- `classify(product_id, profile_code) -> ClassificationResult`; `set_override(...)`; `select_p_statements(...)`.
- `validate(product_id, profile_code) -> Vec<Issue>`.
- `render_preview(revision_id, language) -> Vec<u8>`; `export_published(revision_id) -> Vec<ExportedPdf>`.
- Workflow: `submit`, `approve`, `reject`, `publish`, `new_revision`.
- Admin: users/roles, settings, supplier entities, `backup_now`, `list_backups`, `restore`, `break_checkout`.
- Status: `sessions()`, `checkouts()`, `revision_triggers()`, `read_only_state()`.
- Reference: `search_transport(un_number)`, `phrases(codes, language, profile)`, `profiles()`.
- **Change notification:** a foreign callback interface `ChangeListener` (entity type + IDs changed); polling
  `changes_since(token)` as fallback if spike B shows callbacks are unreliable in C#.
- **Errors** (typed enum): `CheckedOut { user, machine, since }`, `WriteLockBusy { holder }`,
  `ReadOnly { reason: NewerSchema | OlderReferenceData | NetworkLost | NoPermission }`,
  `ValidationFailed { issues }`, `NotFound`, `Conflict`, `Corrupt`, `Io { message }`. Apps map each to a localized,
  human-readable message.

### 9.4 CLI (`sdsctl`)

`sdsctl init`, `open`/`info`, `import` (JSON fixtures), `classify <product> --profile <code> [--trace]`,
`validate`, `render <product> --profile --lang --out`, `publish`, `backup`, `check-integrity`, and a
`stress-lock` command used by locking tests and spike A.

## 10. Core — error handling, logging, testing

**Principles:** never write without the write lock; never lose edits (recovery file); always name who holds a lock;
never show raw error strings to users; fail read-only rather than risk corruption.

**Logging:** local rotating log file per machine (not on the share), 5 × 5 MB, no document contents beyond IDs.

**Testing:**

- Unit tests in every crate; `sds-classify` has no I/O and is tested exhaustively.
- **Golden classification cases** from worked examples in official guidance (OSHA mixture classification guidance,
  ECHA Guidance on the Application of the CLP Criteria), each asserting the result and the derivation trace.
- **Property tests:** increasing the concentration of a hazardous ingredient never lowers any classification;
  adding a non-hazardous ingredient never raises one; results are independent of ingredient order.
- **Threshold verification:** every profile parameter's `cite` is checked against the primary source before its
  rule is marked complete (checklist in the implementation plan).
- **Locking tests:** multi-process tests on local disk (`sdsctl stress-lock`), killed-holder recovery, stale-break
  races; SMB validated in spike A and re-run before each release.
- **Rendering tests:** extracted text + page count assertions, page-image snapshot comparison, veraPDF PDF/A
  validation in CI.
- **CI:** GitHub Actions on macOS arm64, Windows x64 and Windows arm64 run all tests and an end-to-end `sdsctl`
  scenario (init → import → classify → validate → publish → export).

## 11. Risks, open questions, and verification debts

| Item | Status / handling |
|---|---|
| SQLite on SMB with mixed macOS/Windows clients | Spike A decides; fallback designed |
| `uniffi-bindgen-cs` lags UniFFI | Pin UniFFI 0.31.x; C ABI fallback |
| WinUI 3 printing | Spike D; WebView2 fallback |
| CMR and STOT cut-offs, EU subsection titles, Canada Section 12–15 rule, AU requirements | Marked *(from memory)* / *[unverified]* in research — verify against OSHA App A, CLP Annex I, REACH Annex II, HPR Schedule 1, SWA Code of Practice before encoding |
| CAS Registry Number licensing | Unresolved. The app lets users enter CAS RNs; **no bulk CAS data is bundled** until CAS terms (or counsel) confirm otherwise. Handled in Spec 2 |
| ECHA data reuse terms (Annex VI Excel, C&L Inventory) | Unverified; Spec 2 builds from Official Journal texts; C&L data only fetch-on-demand if at all |
| ACGIH TLVs | Copyrighted — never bundled; user-entered |
| GHS phrase text | Sourced only from legislation (OSHA App C, CLP Annex III, HPR), never the UN publication |
| Moving regulatory dates (US mixtures 2027-11-19, EU ATP 23 2027-02-01, CLP label rules 2028/2030) | Profiles are versioned with effective dates; reference data updates ship with app releases |
| Roles are not security | Stated in UI and docs |

## 12. Out of scope for Sub-project 1

GUI apps (Specs 3–4), importers and full non-US profile data (Spec 2), PubChem lookup (Spec 2), installers and
update checks (Spec 5), GHS container label printing, EU SDS distribution/recipient log (REACH Art. 31(9)),
poison-centre (PCN) dossiers, UI localization beyond English, server-based multi-user mode, import/export of
product data between databases.
