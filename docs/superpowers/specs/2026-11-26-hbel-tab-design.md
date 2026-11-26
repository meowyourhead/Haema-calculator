# HBEL tab — design

Date: 26 Nov 2026
Source material: `HBEL_Comment.xlsx` (Sheet1 = comments and criteria, Sheet2 = HbA2 / HbF reference ranges)

## 1. Intent

Bench scientists reporting haemoglobin HPLC (HBEL) results must choose the correct
LabWare comment from about 25 standard comments. Today that means scanning a
spreadsheet. The new **HBEL** tab lets them describe the results they have
(HbA2, HbF, Hb, red cell indices and film, the C and S windows, HbH inclusion bodies)
and immediately see the comment that fits, with its remarks, example cases and
links, ready to copy into LabWare.

The process must be **intuitive, logical and not complicated**: one screen, one
row per result, the suggested comment always visible.

## 2. Goals, non-goals, anti-goals

Goals
- A fifth title-bar tab **HBEL**, after **Dilution**.
- Every comment from column B, except code 2, reformatted (slashes → full stops,
  grammar tidied) with the words otherwise unchanged (exact texts in §6).
- The two un-numbered comments (sheet rows 29 and 31) become **code 25** and **code 26**.
- All the sheet's supporting content shown alongside its comment: criteria (verbatim
  sheet text), remarks, example sample IDs, links, and the cell-note citation.
- The "Library of variants" (Bio-Rad) link available on the tab.
- Works in all three palettes, light and dark, like the other tabs.

Non-goals
- No correction of the clinical content. Comment wording and criteria stay as
  written ("keep everything as written for now, we will fact check later").
  The fact-check findings are written up separately in `docs/hbel-fact-check.md`.
- No "highlighted / not yet added to LabWare" status anywhere (ignored on purpose).
- No numeric entry for Hb or indices (categories only — decided).
- No sending anything to LabWare; the user copies and pastes.
- No persistence of entered results between runs (same as the other tabs).

Anti-goals (would be a failure even if it "works")
- The tab confidently shows one comment when two fit equally well.
- Comment text that differs from what the sheet says beyond the agreed formatting.
- A cluttered, form-heavy screen that is slower than reading the spreadsheet.

## 3. Constraints

- Native Rust / `eframe` + `egui` 0.31 app, single self-contained `.exe`, offline.
  Follow `RELEASING.md` (no binaries committed, date-versioned releases).
- Reuse the existing look: `card`, `chip`, `field`, `button`, `toolbar`, `Theme`
  (`src/ui.rs`, `src/panels.rs`). Tab order / Enter navigation via the existing
  `field()` registration.
- No new crates are required; the data file can be parsed by hand.

## 4. Approaches considered (for storing comments and criteria)

1. **Readable data file compiled into the app (chosen).** One plain-text file in the
   repo (e.g. `assets/hbel_comments.txt`) holds each code's comment, criteria, remarks,
   example cases and links; embedded with `include_str!`. Keeps a single `.exe`,
   one approved version of the text, and makes the later fact-check a small,
   reviewable edit followed by a release.
2. Comments as Rust constants — simplest, but every wording fix is a code edit and
   text is mixed with logic.
3. External file next to the `.exe` — editable on bench PCs without a release, but
   different PCs could show different comment versions and it breaks the
   "copy the .exe anywhere" promise.

Input approach (also decided): **category chips for every result, plus optional
number boxes for HbA2 % and HbF %** that select the chip automatically using the
Sheet2 reference ranges.

## 5. How the tab works

### 5.1 Layout

Wide window: two columns. Narrow window (content width under ~900 pt): the
"Suggested comment" card stacks below the "Results" card.

```
[ Clear ]                                                        (toolbar, right-aligned)
┌ Results ─────────────────────────────────┐ ┌ Suggested comment ─────────── [Copy] ┐
│ HbA2 %   [ 3.7 ]  Low · Normal · (Borderline) · Raised · >10                      │
│ HbA2 peak         Normal · Split · Degradation / co-eluting                      │
│ HbF %    [ 0.8 ]  (Normal) · Raised <5 · 5–50 · >50                              │
│ Hb                Normal · Low · Markedly low (<7)                               │
│ Indices           Normal · Micro/Hypo · Marked                                    │
│ Film              Target cells · Spherocytes · Sickle cells · Marked poikilo.   │
│ C window          (−) · Present                                                  │
│ S window          (−) · Present                                                  │
│ HbH inclusions    Not detected · Detected                                        │
│ HbA1c             Normal · High                                                  │
└──────────────────────────────────────────┘ │ Code 16                                 │
                                             │ HbA present. HbA2 is borderline …      │
                                             │ Criteria · Remarks · Example cases ·   │
                                             │ Links                                    │
                                             ├ Also possible ───────────────────────────┤
                                             │ Code 20 — if HbF: Slightly elevated     │
                                             └──────────────────────────────────────────┘
┌ All HBEL comments (collapsed by default) ─────────────────────────────────────────┐
│ 1  HbA present. No abnormal haemoglobin detected.                       [Copy] ▸ │
│ …                                                                                 │
└───────────────────────────────────────────────────────────────────────────────────┘
Reference ranges: HbA2 2.2–3.4 %, HbF 0–1.3 %   ·   Library of variants (Bio-Rad) ↗
```

### 5.2 Input rows

Each row is a single-select chip group (click a chip to select; click it again to
clear). **An empty row means "not entered" and never excludes a comment.**
The Film row is multi-select.

| Row | Chips (machine meaning) | Number box |
|---|---|---|
| HbA2 % | Low (<2.2) · Normal (2.2–<3.5) · Borderline (3.5–<4.0) · Raised (4.0–10) · >10 | yes |
| HbA2 peak | Normal · Split · Degradation / co-eluting variant | — |
| HbF % | Normal (0–1.3) · Raised <5 (>1.3–<5) · 5–50 · >50 | yes |
| Hb | Normal · Low · Markedly low (<7 g/dL) | — |
| Indices | Normal · Micro/Hypo · Markedly decreased | — |
| Film (multi) | Target cells · Spherocytes · Sickle cells · Marked poikilocytosis | — |
| C window | (−) · Present | — |
| S window | (−) · Present | — |
| HbH inclusions | Not detected · Detected | — |
| HbA1c | Normal · High | — |

Number boxes: typing a value selects the matching chip (highlighted) and matching uses
the **exact number** (needed to separate e.g. HbE 25–35 % from >90 %). Clicking a chip
clears the number and matching uses the chip's range. An invalid number shows the
field in the error style and is ignored. Ranges are half-open so values like 3.45 fall
in exactly one chip.

Fe (column I) is **not** an input row (it barely distinguishes comments and was not in
the user's list); it is still shown in every comment's criteria.

### 5.3 Matching

Each comment has a criterion (or none) per row. For every row the user has filled:
- comment has no criterion for that row → neutral;
- the user's value/range overlaps the comment's criterion → **match**;
- otherwise → **conflict**.

For the Film row, a comment's film criterion matches if any selected film chip is in
it; film never causes a conflict (it only supports).

- **Fits:** comments with zero conflicts and at least one match, ranked by number of
  matches (high first), then code number.
- **Suggested comment:** the top fit. If two or more fits share the top match count,
  the card says "**N comments fit equally well**" and shows each in full, rather than
  picking one.
- **Also possible:** up to 3 comments with exactly one conflict (or lower-ranked fits),
  each with the single difference that would make it the suggestion, e.g.
  "Code 15 — if HbA1c: High".
- Nothing entered → the card shows a short prompt ("Enter results on the left").
  Entries but no fit → "No comment fits exactly" plus the near misses.
- Code 12 has two criteria sets (homozygous, heterozygous); it is evaluated with both
  and the better one is used. The card says which set matched.

### 5.4 Criteria for each code

Criteria are taken from the sheet. Where the sheet leaves a cell blank but the comment
itself implies a value, the implied value is used and shown as *(assumed)* in the
criteria list. Assumptions (builder's/designer's call, flagged for review):

- **A1 – peak shape.** All comments not about variants/peaks assume a *Normal* HbA2
  peak. Exceptions: 22 = Split; 8, 18, 26 = Degradation / co-eluting; 9, 10, 11, 13,
  14 = Normal (sheet: "to report if no degradation follows A2").
- **A2 – windows.** Blank C/S window cells are treated as (−), except code 18
  (variant of unknown position: no window criterion).
- **A3 – code 15.** The remark "A1c elevated" is used as the criterion HbA1c = High.
- **A4 – code 18** has no criteria in the sheet; taken from its own wording:
  HbF raised (>1.3) and peak = Degradation / co-eluting.
- **A5 – "slightly elevated" HbF** (codes 20, 21, 23) and "Elevated (<5%)" (17) both
  mean >1.3–<5 %. "Elevated" (15, 18) means >1.3 %.
- **A6 – "Absent to low" HbF** (13, 14) means 0–1.3 %. "Absent/Low" HbA2 (7, 12)
  means <2.2 %. "Low to Normal" HbA2 (3, 4) means <3.5 %.
- **A7 – Hb.** "Low to Normal" = Normal or Low. "< 7 g/dl" and "Markedly decreased"
  = Markedly low. Code 12 heterozygous "Normal / slightly reduced" = Normal or Low.
- **A8 – Indices.** "Micro/Hypo" and "Low" = Micro/Hypo; "Markedly decreased" (24)
  = Markedly decreased; code 7 "Micro/Hypo/marked poikilocytosis" = Micro/Hypo or
  Markedly decreased, plus film = Marked poikilocytosis; code 9 "Microcytes/target cells/spherocytes" is read as
  an either/or list: no indices criterion, film = target cells or spherocytes; code 10 film = sickle cells.
- **A9 – HbH "NA"** (code 1) means no criterion.
- Fe is display-only; "HbF: Report" (9–11) means no HbF criterion.

| Code | HbA2 | Peak | HbF | Hb | Indices / film | C | S | HbH | Other |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 2.2–<3.5 | N* | 0–1.3 | Normal | Normal | − | − | — | |
| 3 | <3.5 | N* | 0–1.3 | Normal/Low | Micro/Hypo | − | − | Not det. | |
| 4 | <3.5 | N* | 0–1.3 | Normal/Low | Micro/Hypo | − | − | Detected | |
| 5 | 4–8 | N* | 0.1–5 | Normal/Low | Micro/Hypo | − | − | Not det. | |
| 6 | 4–8 | N* | 5–50 | Low | Micro/Hypo | − | − | Not det. | |
| 7 | <2.2 | N* | 92–95 | Markedly low | Micro/Hypo or Marked; film poikilo. | − | − | Not det. | |
| 8 | >10 | Degr. | — | — | — | − | − | — | |
| 9 | — | N | — | — | film target/sphero. | + | − | — | |
| 10 | — | N | — | — | film sickle | − | + | — | |
| 11 | — | N | — | — | — | + | + | — | |
| 12 hom | <2.2 | N* | 100 | — | Normal | −* | −* | — | |
| 12 het | <2.2 | N* | 1–30 | Normal/Low | Normal | −* | −* | — | |
| 13 | 25–35 | N | 0–1.3 | — | — | −* | −* | — | |
| 14 | >90 | N | 0–1.3 | — | — | −* | −* | — | |
| 15 | 2.2–<3.5 | N* | >1.3 | Normal | Normal | −* | −* | — | HbA1c High* |
| 16 | 3.5–<4.0 | N* | 0–1.3 | Normal | Normal | −* | −* | — | |
| 17 | 2.2–<3.5 | N* | >1.3–<5 | Normal | Normal | −* | −* | — | |
| 18 | —* | Degr.* | >1.3* | — | — | — | — | — | |
| 19 | <2.2 | N* | 0–1.3 | Normal | Normal | −* | −* | — | |
| 20 | 3.5–<4.0 | N* | >1.3–<5 | Normal | Normal | −* | −* | — | |
| 21 | 2.2–<3.5 | N* | >1.3–<5 | Low | Micro/Hypo | −* | −* | Not det. | |
| 22 | — | Split | 0–1.3 | Normal | Normal | −* | −* | Not det. | |
| 23 | 2.2–<3.5 | N* | >1.3–<5 | — | — | −* | −* | — | |
| 24 | 30–70 | N* | 10–40 | Markedly low | Markedly decreased | −* | −* | Not det. | |
| 25 | <2.2 | N* | 0–1.3 | Low | Normal | −* | −* | — | |
| 26 | >10 | Degr. | — | — | — | −* | −* | Detected | |

`*` = assumed (A1–A9), shown as *(assumed)* in the app. `—` = no criterion.

Sanity behaviour this produces (to be covered by tests): 26 beats 8 when HbH is
Detected; 8 wins when HbH is Not detected. 20 beats 16 once HbF is raised. With
HbA2 and HbF slightly raised: Hb and indices normal → 17 (15 ties only if HbA1c is
left blank — shown as "fit equally well"; HbA1c High → 15); Hb and indices low → 21;
Hb/indices not entered → 17, 21 and 23 are all shown, with the hints that split them.

### 5.5 Suggested-comment card content

- Code number and the comment text (selectable), **Copy** button → clipboard
  (`ctx.copy_text`), with a brief "Copied" confirmation.
- **Criteria**: the sheet's own wording for each criterion, verbatim (e.g.
  "Slightly elevated (3.5 to 3.9)"), assumed ones marked *(assumed)*, and Fe.
- **Remarks**, **Example cases** (sample IDs), **Links / references** — only the
  sections that have content.
- Matched rows are ticked; conflicting rows (in near misses) highlighted.

### 5.6 "All HBEL comments" card

Collapsed by default. Lists codes 1, 3–26 in order with the comment text and a Copy
button; expanding a row shows the same details as the suggested card. For reference
and training.

### 5.7 Footer

Reference ranges (Sheet2): HbA2 2.2–3.4 %, HbF 0–1.3 %. Link "Library of variants
(Bio-Rad)". Links open in the system's default browser (`ctx.open_url`); the app
itself still makes no network requests. README notes this.

## 6. Comment texts (final)

Formatting rules applied: a `/` used as a sentence separator becomes a full stop and
space; each sentence starts with a capital and ends with a full stop; doubled and
trailing spaces removed; the curly apostrophe becomes a straight one (safer in
LabWare). Slashes that are part of the wording are kept: "and/or" (3, 21) and
"Hb E / Beta Thalassemia" (24). One grammar fix: "Suggest to repeat" → "Suggest
repeating" (15). Spelling (haemoglobin / hemoglobin, Thalassemia) is left as written.

| Code | Comment |
|---|---|
| 1 | HbA present. No abnormal haemoglobin detected. |
| 3 | HbA present. The low red cell indices may suggest Thalassemia Trait and/or Iron Deficiency. Please correlate with family history and other laboratory findings. |
| 4 | HbA present. HbH inclusion body detected. Suggestive of Alpha Thalassemia Trait. Please correlate with family history and other laboratory findings. |
| 5 | HbA present. Suggestive of Beta Thalassemia Minor. Please correlate with family history and other laboratory findings. |
| 6 | HbA present. Suggestive of Beta Thalassemia Intermedia. Please correlate with family history and other laboratory findings. |
| 7 | HbA present. Suggestive of Beta Thalassemia Major. Please correlate with family history and other laboratory findings. |
| 8 | HbA present. Possible variant interference. Please correlate with family history and other laboratory findings. |
| 9 | HbA present. Suspected Hb C trait. Further investigation is required. |
| 10 | HbA present. Suspected Hb S trait. Further investigation is required. |
| 11 | HbA present. Suspected Hb SC trait. Further investigation is required. |
| 12 | HbA present. Suspected Hereditary persistence of fetal hemoglobin (HPFH). Further investigation is required. |
| 13 | HbA present. Suspected Hb E Heterozygous. Further investigation required. |
| 14 | HbA present. Suspected Hb E Homozygous. Further investigation required. |
| 15 | HbA present. Elevated HbF may be due to interference from the high HbA1c levels in the patient's blood. High HbA1c levels can interfere with the accuracy of HbF measurement. Please correlate with family history and other laboratory findings. Suggest repeating after 3 months. |
| 16 | HbA present. HbA2 is borderline elevated but no abnormal haemoglobin detected. |
| 17 | HbA present. The slightly elevated HbF is probably due to physiological causes. |
| 18 | HbF elevated. Unable to determine haemoglobin A2 levels due to possible interference with haemoglobin variant(s). Please correlate with family history and other laboratory findings. |
| 19 | HbA present. HbA2 is slightly below reference range but no abnormal haemoglobin detected. |
| 20 | HbA present. The slightly elevated HbF is probably due to physiological causes. HbA2 is borderline elevated but no abnormal haemoglobin detected. Please correlate with family history and other laboratory findings. |
| 21 | HbA present. HbF slightly elevated but no abnormal haemoglobin detected. Suspected Hereditary Persistence of Fetal Haemoglobin (HPFH). Recommend correlating these results with family history and/or additional laboratory tests. |
| 22 | HbA present. Split HbA2 peak detected suggesting a benign Delta Chain Variant. Exact HbA2 value not reported as split peaks cannot be accurately calibrated. Further investigation required. |
| 23 | HbA present. HbF is borderline elevated but no abnormal haemoglobin detected. |
| 24 | HbA present. Suggestive of Hb E / Beta Thalassemia. Please correlate with family history and other laboratory findings. |
| 25 | HbA present. HbA2 is slightly reduced and HbF minimally elevated. Pattern does not support Beta Thalassemia trait. Please correlate with iron status, clinical findings and family history. |
| 26 | HbA present. Possible variant interference. Exact HbA2 value not reported due to the presence of co-eluting variant peak. HbH inclusion bodies detected. Please correlate with family history and other laboratory findings. |

## 7. Supporting content per code (from the sheet)

| Code | Remarks | Example cases | Links / references |
|---|---|---|---|
| 4 | — | [sample ID], [sample ID] | — |
| 8 | Degradation peak co-eluting with HbA2 peak | — | — |
| 9–11, 13, 14 | HbA2: "To report if no degradation follows A2"; HbF: "Report" | — | — |
| 12 | Homozygous: HbF 100 %, HbA2 absent/low, RBC normal. Heterozygous: HbF ~1 to ~30 %, Hb normal / slightly reduced, normal morphology, MCV and MCHC usually normal, MCV may be slightly increased | [sample ID], [sample ID] | news-medical.net "What is Hereditary Persistence of Fetal Hemoglobin"; PMC7983139 (both URLs from the sheet) |
| 13 | — | [sample ID], [sample ID] | — |
| 15 | A1c elevated | — | — |
| 18 | Refer to D-10 guide | — | — |
| 21 | — | [sample ID], [sample ID] | Patel S, Dehury S, Purohit P, Meher S, Das K (2015). Inheritance of Hereditary Persistence of Fetal Haemoglobin (HPFH) in a Family of Western Odisha, India. J Clin Diagn Res 9(9): OD09–OD10. https://doi.org/10.7860/JCDR/2015/12878.6548 (from cell note by a colleague) |
| 22 | — | [sample ID] | — |
| 24 | — | [sample ID] | — |
| 26 | Degradation peak co-eluting with HbA2 peak | [sample ID] | — |
| (tab) | — | — | Library of variants: https://hemoglobins.bio-rad.com/Pages/ChromatogramPage.aspx?plaid=1&metid=1 |

Exact URLs: news-medical.net/health/What-is-Hereditary-Persistence-of-Fetal-Hemoglobin.aspx
and ncbi.nlm.nih.gov/pmc/articles/PMC7983139/ (https).

Excluded on purpose: code 2, the "Highlighted not added to LabWare comment yet" note,
the yellow highlighting, the stray "```" in F15, and the empty "Column1" header.

## 8. Data file

`assets/hbel_comments.txt`, embedded with `include_str!`. One section per code (code
12 has two criteria sets). Each criterion stores both the **sheet's verbatim text**
(for display) and the **machine meaning** (range or chip set), plus an *assumed*
flag. Exact syntax is the builder's call; it must be plain text that a non-programmer
can edit, with comments allowed, and documented at the top of the file.

The file is parsed at start-up; tests guarantee it parses and is complete, so a bad
edit fails `cargo test` rather than the app.

## 9. Testing

- Data file parses; codes are exactly 1 and 3–26; every code has a comment.
- Comment text rules: no `/` except "and/or" and "Hb E / Beta"; no double spaces;
  every comment ends with a full stop; texts equal §6.
- Matching: one canonical result set per code that suggests that code; the sanity
  behaviours in §5.4; ties reported as ties; empty input → no suggestion; number
  boxes select the right chip at boundaries (2.19, 2.2, 3.4, 3.45, 3.5, 3.99, 4.0,
  10, 10.1; HbF 1.3, 1.31, 4.99, 5, 50, 50.1).
- Manual/screenshot check of the tab in all three palettes, light and dark, wide and
  narrow, using the existing `screenshot` feature.

## 10. Other deliverables

- `docs/hbel-fact-check.md`: the clinical fact-check findings, for the later review.
- README feature list gains the HBEL tab.

## 11. Left to the builder

Module layout (e.g. `src/hbel.rs` for data + matching, panel code in `panels.rs` or a
new file); data-file syntax; exact spacing, chip sizes and colours within the existing
theme; narrow-layout breakpoint; whether the "All comments" card remembers its
expanded state during the session.
