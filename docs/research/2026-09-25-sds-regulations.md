# SDS Regulatory Requirements — Research Notes (as of 2026-09-25)

> Compiled by an AI research agent for the SDS Manager project. **Not legal advice.**
> Several primary sites (eCFR, EUR-Lex, ECHA, UNECE GHS pages) blocked automated fetching; official mirrors
> (osha.gov, legislation.gov.uk, canada.ca, hse.gov.uk) were used instead. Claims marked **[unverified]** rest on
> search snippets or secondary sources, and claims marked *(from memory)* were not re-read from a primary source.
> Everything marked this way must be verified against the primary text before it is encoded in software.

## 1. US — OSHA HCS 2024 (29 CFR 1910.1200)

- Final rule published 20 May 2024, effective 19 Jul 2024; aligns mainly with GHS Rev 7 plus some Rev 8 items
  (non-animal test methods for skin corrosion/irritation). https://www.osha.gov/sites/default/files/publications/OSHA4437.pdf
  Correction notice 9 Oct 2024: https://www.osha.gov/laws-regs/federalregister/2024-10-09-0
- **Compliance dates** (extended 4 months, FR 15 Jan 2026): manufacturers/importers/distributors — substances
  **19 May 2026**, mixtures **19 Nov 2027**; employers — substances 20 Nov 2026, mixtures 19 May 2028. Until those
  dates, HCS 2012, HCS 2024, or both may be followed.
  https://www.osha.gov/hazcom/rulemaking/extension ·
  https://www.federalregister.gov/documents/2026/01/15/2026-00653/hazard-communication-standard
- **SDS rules** (https://www.osha.gov/laws-regs/regulations/standardnumber/1910/1910.1200):
  - (g)(2): 16 headings in fixed order.
  - English required; other languages may be added.
  - (g)(5): new significant hazard info must be added within **3 months**.
  - (g)(8): electronic access OK if there are "no barriers to immediate employee access".
- **Appendix D, Table D.1** — minimum content (https://www.osha.gov/laws-regs/regulations/standardnumber/1910/1910.1200AppD):
  1. Product identifier; other means of identification; recommended use and restrictions; US manufacturer/importer
     name, address, phone; emergency phone.
  2. Classification; signal word, hazard statements, symbols, precautionary statements; hazards identified under
     (d)(1)(ii) (new 2024); hazards not otherwise classified; "% of mixture of unknown acute toxicity" statement when ≥1%.
  3. Substance: chemical name, synonyms, CAS No., impurities contributing to classification. Mixture: exact % or a
     prescribed trade-secret range.
  4. First-aid. 5. Fire-fighting. 6. Accidental release. 7. Handling and storage.
  8. OSHA PEL, ACGIH TLV, other limits; engineering controls; PPE.
  9. 18 properties, ending with new **(r) particle characteristics**.
  10. Reactivity; stability; hazardous reactions; conditions to avoid; incompatibles; hazardous decomposition.
  11. Toxicology incl. NTP/IARC/OSHA carcinogen listings and how data gaps were filled.
  12–15. Content not enforced by OSHA but headings required. Section 14 items: UN number, UN proper shipping name,
      class, packing group, environmental hazards, bulk transport per IMO, user precautions.
  16. "The date of preparation of the SDS or the last change to it".
- **Trade secrets** ((i)(1)(iv)): concentration may be withheld only by prescribed ranges, narrowest possible:
  0.1–1, 0.5–1.5, 1–5, 3–7, 5–10, 7–13, 10–30, 15–40, 30–60, 45–70, 60–80, 65–85, 80–100 %.
- **Labels** (App C, (f)): ≤100 ml containers — product identifier, pictograms, signal word, manufacturer name and
  phone, statement that full info is on the outer package; ≤3 ml — product identifier only if a full label would
  interfere with normal use. Shipped containers need not be relabelled (updated label may accompany shipment).
  OSHA **dropped** the proposed "date released for shipment" element.
- **Classification changes**: App A (health) — updated skin/eye criteria, non-animal methods, "corrosive to the
  respiratory tract". App B (physical) — flammable gases 1A (incl. pyrophoric, chemically unstable A/B), 1B, 2;
  aerosols 1–3 plus **chemicals under pressure**; new class **desensitized explosives**. Physical list: explosives,
  flammable gases, aerosols/chemicals under pressure, oxidizing gases, gases under pressure, flammable liquids 1–4,
  flammable solids, self-reactives A–G, pyrophoric liquids/solids, self-heating, water-reactive, oxidizing
  liquids/solids, organic peroxides, corrosive to metals, desensitized explosives. US extras: combustible dust,
  simple asphyxiants, HNOC. **Aquatic hazard classes (and GHS09) are not adopted.**
  https://www.osha.gov/laws-regs/regulations/standardnumber/1910/1910.1200AppB ·
  https://www.osha.gov/sites/default/files/publications/OSHA3844.pdf

## 2. UN GHS

- **Current: Rev 11** (ST/SG/AC.10/30/Rev.11), published 12 Sep 2025; e-version free for consultation.
  https://unece.org/transport/dangerous-goods/ghs-rev11-2025
  Changes: "Hazardous to the ozone layer" → "Hazardous to the atmospheric system" with global-warming criterion and
  **H421** (per PubChem Rev 11 table); new P322/P323, new P502 matrix, P-statement rationalisation; aerosol and
  chemicals-under-pressure scoping; skin sensitisation non-animal/mixture methods; simple asphyxiant guidance.
  Secondary: https://en.reach24h.com/news/insights/chemical/un-ghs-revision-11
- Rev 10 (2023): desensitized explosives, non-animal methods, P rationalisation.
  https://www.cirs-group.com/en/chemicals/un-ghs-the-10th-revised-edition-has-been-published
- **No major jurisdiction has adopted Rev 10/11 yet**: US Rev 7 (+ some Rev 8); Canada Rev 7 (+ some Rev 8);
  Australia Rev 7; EU via its own ATPs.
- Building blocks: 16-section SDS (Annex 4); H2xx physical / H3xx health / H4xx environmental; P1xx general,
  P2xx prevention, P3xx response, P4xx storage, P5xx disposal; pictograms GHS01–GHS09; signal words Danger/Warning.
  PubChem publishes a Rev 11 H/P/pictogram table as TSV: https://pubchem.ncbi.nlm.nih.gov/ghs/
- **Licensing**: the GHS "purple book" is "© United Nations. All rights reserved."
  https://unece.org/sites/default/files/2023-07/GHS%20Rev10e.pdf — **do not copy phrase text from the UN book.**
  Source it from reusable legislation instead:
  - 29 CFR 1910.1200 App C — US federal work, public domain.
  - CLP Annexes III (statements, all EU official languages) and V (pictograms) — EUR-Lex reuse permitted,
    commercial or not, under Decision 2011/833/EU. https://eur-lex.europa.eu/content/legal-notice/legal-notice.html
  - Canada: federal law reproducible under the Reproduction of Federal Law Order *(from memory; confirm)*.
  - **Canadian French wording differs from EU French** — use Health Canada HPR French text.
  - Pictogram files: OSHA https://www.osha.gov/hazcom/pictograms (OSHA mandates 8 of 9 — check whether GHS09 is
    included, otherwise take it from CLP Annex V). WHMIS adds a biohazardous infectious materials pictogram.

## 3. EU — REACH Annex II (Reg. 2020/878) and CLP

- 2020/878 applies since 1 Jan 2021; old-format SDSs invalid after 31 Dec 2022.
  https://www.legislation.gov.uk/eur/2020/878/annex
- **Part A formatting**: compilation date on page 1; revisions marked "Revision: (date)" with version number, and
  changes flagged in Section 16; every page shows "page X of Y" or "continued on next page" / "end of safety data
  sheet"; simple, clear language — phrases like "may be dangerous" or "harmless" not allowed.
- **Subsections** (structure confirmed; titles *from memory*):
  1.1 Product identifier (incl. **UFI**, and "**nanoform**" where relevant); 1.2 Relevant identified uses / uses
  advised against; 1.3 Supplier details; 1.4 Emergency telephone · 2.1 Classification; 2.2 Label elements; 2.3 Other
  hazards (PBT/vPvB, endocrine disruptor ≥0.1%) · 3.1 Substances / 3.2 Mixtures (SCLs, M-factors, ATEs now required)
  · 4.1–4.3 · 5.1–5.3 · 6.1–6.4 · 7.1–7.3 · 8.1 Control parameters; 8.2 Exposure controls · 9.1 Basic properties;
  9.2 Other information (9.2.1 physical hazard classes; 9.2.2 other safety characteristics) · 10.1–10.6 · 11.1 CLP
  hazard classes; **11.2 Other hazards (11.2.1 endocrine disrupting properties)** · 12.1–12.5; **12.6 Endocrine
  disrupting properties**; 12.7 Other adverse effects · 13.1 Waste treatment · 14.1 "UN number or ID number" … 14.7
  "Maritime transport in bulk according to IMO instruments" · 15.1 Regulations specific to the substance/mixture;
  15.2 Chemical safety assessment · 16 Other information. Secondary: https://ekotox.eu/sds-new-requirements-2023/
- **All 16 sections mandatory in the EU**, including 12–15.
- SDS supplied free of charge, paper or electronic, by first delivery; after revision re-sent to recipients from
  the previous 12 months (REACH Art. 31(8)–(9), *from memory*).
- **New CLP hazard classes** (Delegated Reg. 2023/707): ED HH, ED ENV, PBT/vPvB, PMT/vPvM; EUH380/381, EUH430/431,
  EUH440/441, EUH450/451. Substances: new from 1 May 2025, existing by 1 Nov 2026. Mixtures: new from **1 May 2026**,
  existing by **1 May 2028**. https://eur-lex.europa.eu/eli/reg_del/2023/707/oj/eng
- **CLP revision** (Reg. 2024/2865): in force 10 Dec 2024; main provisions 1 Jul 2026 / 1 Jan 2027 — label minimum
  font sizes, 120% line spacing, black on white, fold-out labels, voluntary digital labelling, advertising and
  distance sales. **Reg. 2025/2439 ("stop-the-clock")** postponed label formatting, relabelling, advertising and
  distance sales to **1 Jan 2028**.
  https://en.reach24h.com/news/industry-news/chemical/eu-delays-clp-implementation-dates-to-2028
  **Omnibus VI**: provisional agreement 16–17 Jun 2026 — further postponement to **1 Jan 2030**, font sizes mainly for
  consumer products, small-package and digital-label derogations; formal adoption pending.
  https://www.produktkanzlei.com/en/2026/06/24/provisional-agreement-on-the-chemicals-omnibus-omnibus-vi/ ·
  https://single-market-economy.ec.europa.eu/news/commission-welcomes-political-agreement-simplify-rules-chemicals-sector-2026-06-16_en
- **Annex VI harmonised classifications**: ATP 22 applies from 1 May 2026; ATP 23 (Reg. 2025/1222) from
  **1 Feb 2027**. https://www.chemius.net/atp/23rd-atp-to-clp-regulation-published-what-you-need-to-know/
- **Language**: official language(s) of each Member State where placed on the market; national rules vary.
  Finland: Finnish **and** Swedish. Luxembourg: French **or** German per national helpdesk
  (https://www.reach.lu/en/supply-chain/safety-data-sheets). Belgium: check by region. ECHA table (not fetched):
  https://echa.europa.eu/documents/10162/17217/languages_required_for_labels_and_sds_en.pdf
- A later Annex II amendment carrying the 2023/707 classes into the SDS is expected **[unverified]**.

## 4. Canada — WHMIS 2015 / Hazardous Products Regulations

- 2022 amendments in force 15 Dec 2022; aligned to GHS Rev 7 + parts of Rev 8: chemicals under pressure,
  non-flammable aerosols, flammable gas subcategories, new SDS elements, alternative combustible dust statement.
  CBI concentrations may use narrower ranges within the prescribed ranges. **Transition ended 14 Dec 2025.** Health
  Canada focuses on compliance promotion until 19 Jul 2027.
  https://www.canada.ca/en/health-canada/services/environmental-workplace-health/occupational-health-safety/workplace-hazardous-materials-information-system/amendments-hazardous-products-regulations.html
- SDS in **English and French** — one bilingual document or two versions supplied together. Section 16 shows the
  date of the latest revision. Update within **90 days** of significant new data.
  https://www.ccohs.ca/oshanswers/chemicals/whmis_ghs/sds.html
- Canadian supplier identity required. Canada-specific classes: biohazardous infectious materials, PHNOC/HHNOC.
  Explosives excluded (Explosives Act). Carcinogen Cat 2 at 0.1–1% needs label and SDS.
  Variances vs US: https://www.ccohs.ca/oshanswers/chemicals/whmis_ghs/variances.html
- **[unverified]** Section 12–15 headings required but content not — confirm against HPR Schedule 1.

## 5. UK and Australia

- **Great Britain**: GB CLP and UK REACH. Reg. 2020/878 was **not** carried over — GB SDSs follow the older
  2015/830-style Annex II. No UFI or poison-centre notification. Classifications from the **GB MCL list**, not EU
  Annex VI. From 21 May 2026 substance C&L notification to HSE abolished. UK law under Open Government Licence.
  https://www.hse.gov.uk/chemical-classification/legal/changes-gb-clp-regulation.htm ·
  https://alchemycompliance.co.uk/2021/02/25/new-gb-clp-regulation-and-gb-safety-data-sheets/
  Northern Ireland follows EU rules.
- **Australia**: WHS Regulations; GHS Rev 7 mandatory since 1 Jan 2023; Safe Work Australia model Code of Practice
  on preparing SDSs. https://www.safeworkaustralia.gov.au/safety-topic/hazards/chemicals/classifying-chemicals/transition-ghs7
  *(From memory)*: English, Australian supplier and phone, review every 5 years, ADG Code in Section 14.
- **One engine can cover all of them.** Differences are profile data: GHS revision and building blocks, subsection
  numbering and headings, mandatory vs optional sections, language set, local phrases (EUH, Canadian supplementary,
  WHMIS classes), supplier/emergency rules, trade-secret ranges, update deadlines, Section 15 lists. The one real
  engine difference: mixture cut-offs vary by jurisdiction.

## 6. Section 14 — transport data

Fields: UN number, proper shipping name, class and subsidiary risk, packing group, environmental hazard / marine
pollutant, special precautions, IMO bulk. Modes: DOT (49 CFR), ADR/RID, IMDG, IATA, ADG (Australia).

| Source | Status |
|---|---|
| **49 CFR 172.101 Hazardous Materials Table** | US public domain; Excel export via PHMSA oCFR (updated to Aug 2026); marine pollutants in App B. https://portal.phmsa.dot.gov/ocfrTool/hazmat-apx/appxb |
| UN Model Regulations Rev 24 (2025) Dangerous Goods List | Free to view, © UN. https://unece.org/transport/dangerous-goods/un-model-regulations-rev-24 |
| ADR 2025 | Free to view, © UN; ADR 2027 applies from 1 Jan 2027. https://unece.org/adr-2025-files |
| IMDG Code (IMO), IATA DGR | Paid, copyrighted — do not bundle |

Recommendation: pre-fill UN number, name, class and packing group from the US table; IMDG (EmS, marine pollutant)
and IATA fields user-entered.

## 7. Mixture classification — what can be automated

Formulas from GHS/CLP *(from memory; OSHA App A matches GHS)* — **all thresholds must be verified against OSHA
App A and CLP Annex I before coding.**

- **Acute toxicity**: 100/ATEmix = Σ(Cᵢ/ATEᵢ). If ingredients of unknown toxicity total >10%:
  (100 − ΣC_unknown)/ATEmix = Σ(Cᵢ/ATEᵢ). Category/range data → point estimate via conversion table (e.g. oral
  Cat 1 → 0.5 mg/kg). Ingredients ≥1% considered. Section 2 "unknown acute toxicity" statement computable.
- **Skin**: ΣSkin1 ≥5% → Cat 1; ΣSkin1 1–5% → Cat 2; ΣSkin2 ≥10% → Cat 2; (10 × Skin1) + Skin2 ≥10% → Cat 2.
  pH ≤2 / ≥11.5 rule plus exceptions needs user decision.
- **Eye**: Skin1 + Eye1 ≥3% → Eye 1; 1–3% → Eye 2; 10 × (Skin1 + Eye1) + Eye2 ≥10% → Eye 2. (OSHA App A confirms
  the 5/10 and 3/10 thresholds.)
- **Cut-off classes**:
  - Mutagens: Cat 1 ≥0.1%, Cat 2 ≥1% (OSHA confirmed).
  - Sensitizers (OSHA): resp 1/1A/1B ≥0.1%; skin 1A ≥0.1%, 1B ≥1%. CLP: 1% for skin/resp Cat 1 and 1B, 0.1% for
    1A, 0.2% for respiratory gases.
  - Carcinogens: 1A/1B ≥0.1%; Cat 2 ≥1% (CLP). OSHA: SDS for Cat 2 from 0.1%, label from 1%.
  - Reproductive: CLP Cat 1 ≥0.3%, Cat 2 ≥3%, lactation ≥0.3%. OSHA 0.1%.
  - STOT SE/RE: Cat 1 ingredient ≥10% → Cat 1; 1–10% → Cat 2; Cat 2 ingredient ≥10% → Cat 2.
  - Aspiration: ≥10% Cat 1 ingredients and kinematic viscosity ≤20.5 mm²/s at 40 °C.
  - EU ED/PBT/PMT: ≥0.1%.
  - **CMR and STOT cut-offs are from memory — confirm before coding.**
- **Aquatic** (CLP; Canada optional; not OSHA): Acute 1 if Σ(Acute1 × M) ≥25%; Chronic 1 if Σ(Chronic1 × M) ≥25%;
  Chronic 2 if 10 × M × Chronic1 + Chronic2 ≥25%; Chronic 3 if 100 × M × Chronic1 + 10 × Chronic2 + Chronic3 ≥25%;
  Chronic 4 if Σ Chronic 1–4 ≥25%. Plus additivity formula when test data exist.
- **EU Annex VI SCLs, M-factors and ATEs override generic limits**; "minimum classification" (\*) entries and Annex
  VI notes need handling.
- **Label elements**: pictogram precedence (skull suppresses exclamation mark; corrosion suppresses irritation),
  signal word (Danger beats Warning), H de-duplication, P selection and trimming (≤6 is CLP guidance).
- **Needs data or judgement**: physical hazards need test data (flash point + boiling point → flammable liquid 1–4;
  EU has no Cat 4); bridging principles (dilution, batching, interpolation, substantially similar mixtures,
  aerosols); weight of evidence; HNOC.

## 8. Ingredient / substance data sources

| Source | Can a free app bundle it? |
|---|---|
| CLP Annex VI Table 3 | Legal text reusable (EUR-Lex notice). ECHA's Excel extract reuse terms not fetched **[unverified]** — safer to build from Official Journal ATP texts |
| ECHA C&L Inventory / ECHA CHEM | Terms not fetched; snippet reported third-party rights and resale restrictions **[unverified]** — fetch-on-demand, not bundled; legal review |
| PubChem | NCBI places no restrictions, **but depositors may hold rights** — keep depositor source per record. https://www.ncbi.nlm.nih.gov/home/about/policies/ |
| OSHA PELs (Z tables), NIOSH RELs / Pocket Guide | US government works — bundle freely |
| Cal/OSHA PELs, Prop 65, EPA lists (SARA 313, CERCLA, TSCA) | Government lists; generally free (confirm each) |
| EU IOELVs/BOELVs | Directives on EUR-Lex — reusable |
| **ACGIH TLVs/BEIs** | **Do not bundle** — copyrighted (https://www.acgih.org/about/copyright-policy/). Required by OSHA App D in Section 8, so **user-entered** |
| **CAS Registry Numbers** | **High risk.** CAS Information Use Policy (June 2024) caps downloads at 5,000 and prohibits aggregation/redistribution "for any commercial use, whether paid or unpaid" (https://www.cas.org/legal/infopolicy). A snippet mentioned a licence-free allowance of up to 10,000 CAS RNs in no-charge products — **not found in current policy; unconfirmed, ask CAS.** CAS RNs appearing inside legislation (Annex VI, 49 CFR table, Z tables) — question for counsel |
| GB MCL (HSE) | UK government; likely Open Government Licence (confirm) |

## 9. SDS format rules

- 16 fixed headings in fixed order everywhere; EU adds mandatory subsection numbers and titles.
- Dates: US date of preparation or last change (Section 16); Canada date of latest revision (Section 16); EU
  compilation date on page 1, "Revision: date" and version, changes flagged.
- Page numbering mandatory in the EU; good practice elsewhere.
- Language: US English (others allowed); Canada EN + FR; EU national official language(s); GB and AU English.
- Electronic delivery: US OK if no barriers; EU paper or electronic, free; Canada electronic allowed *(from memory)*.
  Use PDF/A output.
- Revision triggers: US 3 months; Canada 90 days; EU without delay with re-distribution to last-12-month recipients.
