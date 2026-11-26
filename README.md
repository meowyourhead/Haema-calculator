# Haematology Calculator (native Windows)

A standalone, fully offline rebuild of `Haematology Calculator.html`, written in Rust with
`eframe`/`egui`. There is no browser engine, web view, installer, or network access involved.

## Run

Double-click **`Haematology Calculator.exe`**.

The executable is self-contained: fonts and all assets are compiled into it, so it can be
copied anywhere and run without the source folder.

## Releases

Releases are date-versioned (for example **9 Nov 2026**). Download the latest
`Haematology-Calculator-<version>.exe` from the repository's **Releases** page.
See `RELEASING.md` for the commit, push and release rules used by this project.

## Features

- Frameless window with a centered title bar: **FBC / Retic / PT / APTT / Dilution / HBEL** tabs plus
  custom minimize / maximize / close controls. Drag the empty part of the strip to move the window,
  double-click it to maximize, and drag any edge or corner to resize.
- The sidebar is now just the theme / palette controls (the page tabs live in the title bar).
- Fox application icon embedded in the `.exe`, so it also appears in the taskbar, Alt+Tab and any
  shortcut to the program.
- **Tab** / **Enter** move to the next input field and **Shift+Tab** / **Shift+Enter** to the
  previous one, wrapping at the ends. Off-screen fields are scrolled into view automatically.
- **FBC** and **PT / APTT** duplicate-run checkers with two or three runs.
- Percentage difference badges, per-card status and an overall status.
- **Reticulocyte** calculator with corrected and absolute retic counts, gender thresholds,
  rounding notes and a step-by-step calculations panel.
- **Counting helper** with 3–12 attempts and a "Use as retic %" action.
- **Dilution** calculator for 1:2 diluted samples (calculated neat = ×2).
- **HBEL** comment helper: pick results (or type HbA2 / HbF %) to get the matching LabWare
  comment with its criteria, remarks, example cases and links; **View source sheet** shows the
  original spreadsheet content with search and filters. Comments live in
  `assets/hbel_comments.txt` (edit, then rebuild). Links open in the default browser.
- **Add new comment** (HBEL): saves a comment for a new kind of case, using the results entered
  as its criteria, into a shared **`6. HBEL`** folder next to the `.exe` (one `HBEL-<code>.txt`
  file per comment, codes from 27). The app asks before creating the folder. Every PC that runs
  the app from that location sees the added comments; **Reload** in the source sheet re-reads
  them, and deleting a file removes its comment.
- Three palettes — **Default**, **Genshin**, **Pokémon** — each with light and dark variants.
  Genshin uses a soft, pre-blurred world backdrop (baked from the saved Game UI Database
  reference), gold hairlines and ivory/gold pill controls. Pokémon follows the Let's Go,
  Pikachu/Eevee reference: a sunlit blurred meadow, rounded cream paper cards with milky rims,
  teal outlines, and Pokémon-red action buttons.
- Theme and palette are remembered in `%APPDATA%\HaematologyCalculator\settings.txt`.

## Build from source

```sh
cargo build --release
```

The resulting binary is `target\release\haematology_calculator.exe`. The embedded fonts are
already included in the repository under `assets/fonts`; `tools/extract_fonts.py` documents how
they were extracted from the original HTML. The fox icon is generated from `fox.jpg` into
`assets/icon/fox.ico` (exe resource, via `build.rs` and the Windows resource compiler) and
`assets/icon/fox-128.rgba` (window / taskbar icon).

Run the calculation-logic tests with:

```sh
cargo test
```

## Genshin design reference

The Genshin theme is grounded in `Game UI Database - Genshin Impact (Console).mhtml`.
Extract its screenshots and contact sheets for inspection with:

```sh
python tools/genshin_reference.py
```

Regenerate the blurred page backdrop (192×128 raw RGBA, embedded in the binary) with:

```sh
python tools/make_genshin_backdrop.py
```

Future Genshin theme work should use the `genshin-ui` subagent in
`.opencode/agents/genshin-ui.md`, which documents the palette, materials, blur
treatment, ornament vocabulary and the screenshot-verification workflow.

## Pokémon design reference

The Pokémon theme is grounded in the three Let's Go, Pikachu/Eevee pages under `pokemon ui/`.
Extract the screenshots and contact sheets with:

```sh
python tools/pokemon_reference.py
```

Regenerate the blurred meadow backdrop (192×108 raw RGBA, embedded in the binary) with:

```sh
python tools/make_pokemon_backdrop.py
```

The reference language is: cream paper panels with thick milky rims and soft shadows;
rounded Poppins-style type; teal outlines for selections and focus; Pokémon-red CTAs;
pale yellow highlights; and a sunlit, out-of-focus meadow behind the UI.
