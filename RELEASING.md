# Releasing and repository rules

These instructions apply to every future commit, push and release of this project.
Keep them in sync with how the repository is actually published.

## What goes in the repository

Commit only source code, build files and the assets required to build the app:

- `src/`, `examples/`, `build.rs`, `Cargo.toml`, `Cargo.lock`
- `assets/` (fonts, icon, baked backdrops are embedded into the executable)
- `tools/`, `README.md`, `RELEASING.md`, and the small agent/skill files
- `assets/icon/fox.jpg` (source for the generated icon)

Do **not** commit:

- `target/` or anything from the Rust toolchain / cargo registry (generic,
  downloadable dependencies)
- built binaries (`*.exe`, `*.pdb`); the release EXE is attached to a GitHub
  release instead
- large downloaded design-reference captures (`*.mhtml`); they are not needed
  to build and can be re-downloaded from Game UI Database

`.gitignore` enforces the list above.

## Version format

The version is the release date written as **`9 Nov 2026`** (day, abbreviated
month, year). Git tags cannot contain spaces, so the tag uses hyphens:
`9-Nov-2026`. The GitHub release title stays `9 Nov 2026`.

Each release uses the date it is published. Do not reuse an old tag.

The same date is the package version in `Cargo.toml`, written `year.month.day`
without leading zeros (`9 Nov 2026` → `version = "2026.11.9"`). The app's update
checker compares it with the newest GitHub release tag, so a build whose
version is not bumped tells users its own release is an update.

The update checker reads `https://api.github.com/repos/meowyourhead/Haema-calculator/releases/latest`
without logging in, so it only works while the repository is public.

## Release checklist

0. Set `version` in `Cargo.toml` to the release date (see above).

1. Make sure the working tree is clean and the code builds:

   ```sh
   cargo test
   cargo build --release
   ```

2. Copy the freshly built binary and give the release asset a versioned name:

   ```powershell
   Copy-Item target\release\haematology_calculator.exe "Haematology-Calculator-<version>.exe"
   ```

   Example for 9 Nov 2026: `Haematology-Calculator-9-Nov-2026.exe`.

3. Verify the EXE is the one just built (optional but recommended):

   ```powershell
   Get-FileHash target\release\haematology_calculator.exe, "Haematology-Calculator-9-Nov-2026.exe"
   ```

4. Commit and push the source (never the EXE):

   ```sh
   git add .
   git commit -m "Release 9 Nov 2026"
   git push origin main
   ```

5. Create the GitHub release and attach the EXE:

   ```sh
   gh release create 9-Nov-2026 "Haematology-Calculator-9-Nov-2026.exe" `
     --title "9 Nov 2026" `
     --notes "Haematology Calculator release for 9 Nov 2026."
   ```

   For future dates use the same pattern, e.g. tag `3-Dec-2026`, title
   `3 Dec 2026`, asset `Haematology-Calculator-3-Dec-2026.exe`.

6. Remove the local copy of the versioned EXE (it is not tracked), or leave it
   outside the repository. Never `git add` it.

## Summary

Future work follows the same rule: push necessary source and build assets only,
ignore generic downloadable dependencies and build output, and publish the
compiled EXE as an asset on a date-versioned GitHub release.
