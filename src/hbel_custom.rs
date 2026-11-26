//! New HBEL comments added by users, shared through the "6. HBEL" folder next to the exe.
//! Each comment is one small text file in the same format as `assets/hbel_comments.txt`.
use crate::hbel_data::{parse, CritSet, Criterion, Entry, Library, Range, Row, Rule, ROWS};
use crate::hbel_match::{Inputs, NumState, Value};
use std::path::{Path, PathBuf};

/// Name of the shared folder, next to the exe.
pub const FOLDER: &str = "6. HBEL";

/// `<folder containing the exe>/6. HBEL`.
pub fn folder_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join(FOLDER))
}

/// The data-file syntax for a range (inverse of `Range::parse`).
pub fn range_syntax(r: &Range) -> String {
    let f = |v: f64| crate::logic::fmt_js(v);
    match (r.lo_incl, r.hi_incl) {
        (false, true) if r.hi >= 100.0 => format!(">{}", f(r.lo)),
        (true, false) if r.lo <= 0.0 => format!("<{}", f(r.hi)),
        (true, true) => format!("{}..{}", f(r.lo), f(r.hi)),
        (true, false) => format!("{}..<{}", f(r.lo), f(r.hi)),
        (false, true) => format!(">{}..{}", f(r.lo), f(r.hi)),
        (false, false) => format!(">{}..<{}", f(r.lo), f(r.hi)),
    }
}

/// Sheet column for a row (rows without one go into the remarks).
fn sheet_column(row: Row) -> Option<&'static str> {
    match row {
        Row::Hba2 => Some("A2"),
        Row::Hbf => Some("F"),
        Row::Hb => Some("Hgb"),
        Row::Indices => Some("RBC indices"),
        Row::CWin => Some("C window"),
        Row::SWin => Some("S window"),
        Row::Hbh => Some("HbH inclusion body"),
        Row::Peak | Row::Film | Row::Hba1c => None,
    }
}

fn labels(row: Row, mask: u8) -> String {
    row.options().iter().enumerate().filter(|(i, _)| mask & (1 << i) != 0).map(|(_, o)| o.label).collect::<Vec<_>>().join(", ")
}

/// Turn the results currently entered into a new comment with code `code`.
/// A typed HbA2 / HbF number becomes the range of the chip it falls in.
pub fn entry_from_inputs(code: u8, comment: &str, remark: &str, example: &str, inp: &Inputs, stamp: &str) -> Result<Entry, String> {
    let mut comment = comment.split_whitespace().collect::<Vec<_>>().join(" ");
    if comment.is_empty() {
        return Err("Type the comment text first.".into());
    }
    if !comment.ends_with('.') {
        comment.push('.');
    }
    let mut set = CritSet::default();
    let mut extra_remarks = Vec::new();
    for row in ROWS {
        let Some(v) = inp.value(row) else { continue };
        let (rule, text) = match v {
            Value::Opts(m) => (Rule::AnyOf(m), labels(row, m)),
            Value::Span(r) => {
                let label = row.options().iter().find(|o| o.range == Some(r)).map_or("", |o| o.label);
                (Rule::Range(r), label.to_string())
            }
            Value::Num(x) => {
                let opt = row.options().iter().find(|o| o.range.is_some_and(|r| r.contains(x)));
                let Some(opt) = opt else { continue };
                let NumState::Valid(_) = inp.number(row) else { continue };
                (Rule::Range(opt.range.unwrap()), format!("{} % ({})", crate::logic::fmt_js(x), opt.label))
            }
        };
        match sheet_column(row) {
            Some(col) => set.sheet.push((col.to_string(), text)),
            None => extra_remarks.push(format!("{}: {}", crate::hbel_match::row_name(row), text)),
        }
        set.crits.push(Criterion { row, rule, assumed: false, key: false });
    }
    if set.crits.is_empty() {
        return Err("Enter the case's results on the left first; they become the new comment's criteria.".into());
    }
    let mut remarks = Vec::new();
    let remark = remark.trim();
    if !remark.is_empty() {
        remarks.push(remark.to_string());
    }
    remarks.extend(extra_remarks);
    remarks.push(stamp.to_string());
    let examples = example.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect();
    Ok(Entry { code, comment, sets: vec![set], remarks, examples, ..Default::default() })
}

/// The entry in data-file format.
pub fn to_text(e: &Entry) -> String {
    let one = |s: &str| s.replace(['\r', '\n'], " ");
    let mut out = format!("# HBEL comment added in the app. Delete this file to remove the comment.\n[{}]\ncomment: {}\n", e.code, one(&e.comment));
    for r in &e.remarks {
        out += &format!("remark: {}\n", one(r));
    }
    for x in &e.examples {
        out += &format!("example: {}\n", one(x));
    }
    for s in &e.sets {
        for (k, v) in &s.sheet {
            out += &format!("sheet {}: {}\n", k, one(v));
        }
        for c in &s.crits {
            let rule = match &c.rule {
                Rule::Range(r) => range_syntax(r),
                Rule::AnyOf(m) => c.row.options().iter().enumerate().filter(|(i, _)| m & (1 << i) != 0).map(|(_, o)| o.key).collect::<Vec<_>>().join(", "),
            };
            out += &format!("{}: {}\n", c.row.key(), rule);
        }
    }
    out
}

/// Write `e` into `dir` as `HBEL-<code>.txt` without ever overwriting: if that code's
/// file already exists (another PC saved first), the next free code is used.
pub fn save(dir: &Path, e: &Entry) -> Result<(u8, PathBuf), String> {
    let mut e = e.clone();
    for _ in 0..200 {
        let path = dir.join(format!("HBEL-{}.txt", e.code));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut f) => {
                use std::io::Write;
                f.write_all(to_text(&e).as_bytes()).map_err(|err| format!("Could not write {}: {}", path.display(), err))?;
                return Ok((e.code, path));
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                e.code = e.code.checked_add(1).ok_or("No free comment codes left (max 255).")?;
            }
            Err(err) => return Err(format!("Could not save to {}: {}", dir.display(), err)),
        }
    }
    Err("No free comment code found.".into())
}

/// Example sample IDs for the built-in comments. They identify real lab
/// specimens, so they live only in the shared folder, never in the source.
/// One line per code: `4: 26XX1234, 26XX5678` (`#` starts a note).
pub const EXAMPLES_FILE: &str = "examples.txt";

/// Read `EXAMPLES_FILE` from `dir`: (code, sample IDs) pairs and any bad lines.
pub fn load_examples(dir: &Path) -> (Vec<(u8, Vec<String>)>, Vec<String>) {
    match std::fs::read_to_string(dir.join(EXAMPLES_FILE)) {
        Ok(text) => parse_examples(&text),
        Err(_) => (Vec::new(), Vec::new()),
    }
}

pub fn parse_examples(text: &str) -> (Vec<(u8, Vec<String>)>, Vec<String>) {
    let mut out = Vec::new();
    let mut errors = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parsed = line.split_once(':').and_then(|(c, ids)| {
            let code = c.trim().parse::<u8>().ok()?;
            let ids: Vec<String> = ids.split([',', ';']).map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect();
            (!ids.is_empty()).then_some((code, ids))
        });
        match parsed {
            Some(p) => out.push(p),
            None => errors.push(format!("{} line {}: use `code: sample ID, sample ID`", EXAMPLES_FILE, n + 1)),
        }
    }
    (out, errors)
}

/// Add the examples to their comments (unknown codes are reported).
pub fn attach_examples(lib: &mut Library, examples: &[(u8, Vec<String>)]) -> Vec<String> {
    let mut errors = Vec::new();
    for (code, ids) in examples {
        match lib.entries.iter_mut().find(|e| e.code == *code) {
            Some(e) => {
                for id in ids {
                    if !e.examples.contains(id) {
                        e.examples.push(id.clone());
                    }
                }
            }
            None => errors.push(format!("{}: there is no comment with code {}", EXAMPLES_FILE, code)),
        }
    }
    errors
}

/// Read every `.txt` file in `dir` (except `EXAMPLES_FILE`). Files that fail to parse are reported, not fatal.
pub fn load(dir: &Path) -> (Vec<Entry>, Vec<String>) {
    let mut entries = Vec::new();
    let mut errors = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return (entries, errors) };
    let mut files: Vec<PathBuf> = rd
        .filter_map(|d| d.ok().map(|d| d.path()))
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("txt")))
        .filter(|p| !p.file_name().is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(EXAMPLES_FILE)))
        .collect();
    files.sort();
    for p in files {
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        match std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|s| parse(&s)) {
            Ok(lib) => entries.extend(lib.entries.into_iter().map(|mut e| {
                e.source = Some(name.clone());
                e
            })),
            Err(err) => errors.push(format!("{}: {}", name, err)),
        }
    }
    (entries, errors)
}

/// Built-in comments plus the added ones (an added comment whose code is already used is left out).
pub fn combined(base: &Library, custom: &[Entry]) -> Library {
    let mut lib = base.clone();
    for e in custom {
        if !lib.entries.iter().any(|x| x.code == e.code) {
            lib.entries.push(e.clone());
        }
    }
    lib.entries.sort_by_key(|e| e.code);
    lib
}

/// Messages for added comments left out by `combined` because their code is taken.
pub fn combined_errors(base: &Library, custom: &[Entry]) -> Vec<String> {
    let mut seen: Vec<u8> = base.entries.iter().map(|e| e.code).collect();
    let mut out = Vec::new();
    for e in custom {
        if seen.contains(&e.code) {
            out.push(format!("{}: code {} is already used, so this comment was skipped", e.source.as_deref().unwrap_or("?"), e.code));
        } else {
            seen.push(e.code);
        }
    }
    out
}

pub fn next_code(lib: &Library) -> u8 {
    lib.entries.iter().map(|e| e.code).max().unwrap_or(0).saturating_add(1)
}

/// "26 Nov 2026" for a count of days since 1 Jan 1970 (civil-from-days, proleptic Gregorian).
pub fn civil_date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!("{} {} {}", d, M[(m - 1) as usize], y)
}

/// "Added 26 Nov 2026 on LAB-PC1" (UTC date; PC name from COMPUTERNAME / HOSTNAME).
pub fn stamp() -> String {
    let days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64 / 86_400).unwrap_or(0);
    let pc = std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_default();
    if pc.is_empty() {
        format!("Added {}", civil_date(days))
    } else {
        format!("Added {} on {}", civil_date(days), pc)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::hbel_data::{library, parse, Range, Row, Rule};
    use crate::hbel_match::{suggest, Inputs};

    #[test]
    fn examples_file() {
        let (ex, bad) = parse_examples("\u{feff}# real IDs\n\n4: 26XX0001; 26XX0002\n99: 26XX0003\nnonsense\n12:\n");
        assert_eq!(ex, vec![(4, vec!["26XX0001".to_string(), "26XX0002".to_string()]), (99, vec!["26XX0003".to_string()])]);
        assert_eq!(bad.len(), 2, "{:?}", bad);
        let mut lib = library().clone();
        let errs = attach_examples(&mut lib, &ex);
        assert_eq!(errs, vec!["examples.txt: there is no comment with code 99".to_string()]);
        attach_examples(&mut lib, &ex); // re-reading does not duplicate
        assert_eq!(lib.entries.iter().find(|e| e.code == 4).unwrap().examples.len(), 2);
    }

    #[test]
    fn examples_file_is_not_a_comment_file() {
        let dir = std::env::temp_dir().join(format!("hbel-ex-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(EXAMPLES_FILE), "4: 26XX0001\n").unwrap();
        let (entries, errors) = load(&dir);
        assert!(entries.is_empty() && errors.is_empty(), "{:?}", errors);
        assert_eq!(load_examples(&dir).0.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("hbel-test-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn sample_inputs() -> Inputs {
        let mut i = Inputs { hba2: "3.7".into(), ..Default::default() };
        i.click_chip(Row::Hb, 1); // Low
        i.click_chip(Row::Film, 0); // Target cells
        i
    }

    #[test]
    fn range_syntax_roundtrip() {
        for s in ["3.5..<4", ">1.3..<5", ">10", "<2.2", "0..1.3", "100..100", ">1.3"] {
            let r = Range::parse(s).unwrap();
            assert_eq!(Range::parse(&range_syntax(&r)), Some(r), "{s}");
        }
    }

    #[test]
    fn entry_from_inputs_builds_criteria() {
        let e = entry_from_inputs(27, "  New pattern seen  ", "Check with senior", "26XX0001", &sample_inputs(), "Added 26 Nov 2026 on LAB-PC1").unwrap();
        assert_eq!(e.code, 27);
        assert_eq!(e.comment, "New pattern seen.");
        let s = &e.sets[0];
        let rule = |r: Row| s.crits.iter().find(|c| c.row == r).map(|c| c.rule.clone());
        assert_eq!(rule(Row::Hba2), Some(Rule::Range(Range::parse("3.5..<4").unwrap())));
        assert_eq!(rule(Row::Hb), Some(Rule::AnyOf(0b010)));
        assert_eq!(rule(Row::Film), Some(Rule::AnyOf(0b0001)));
        assert_eq!(rule(Row::Hbf), None);
        assert!(s.sheet.contains(&("A2".to_string(), "3.7 % (Borderline)".to_string())));
        assert!(s.sheet.contains(&("Hgb".to_string(), "Low".to_string())));
        assert_eq!(e.remarks, vec!["Check with senior".to_string(), "Film: Target cells".to_string(), "Added 26 Nov 2026 on LAB-PC1".to_string()]);
        assert_eq!(e.examples, vec!["26XX0001".to_string()]);
    }

    #[test]
    fn entry_from_inputs_requires_comment_and_results() {
        assert!(entry_from_inputs(27, "   ", "", "", &sample_inputs(), "x").is_err());
        assert!(entry_from_inputs(27, "Text.", "", "", &Inputs::default(), "x").is_err());
    }

    #[test]
    fn to_text_roundtrips_through_parser() {
        let e = entry_from_inputs(27, "New pattern seen.", "", "26XX0001", &sample_inputs(), "Added today").unwrap();
        let lib = parse(&to_text(&e)).unwrap();
        let back = &lib.entries[0];
        assert_eq!(back.code, 27);
        assert_eq!(back.comment, e.comment);
        assert_eq!(back.sets[0].sheet, e.sets[0].sheet);
        assert_eq!(back.sets[0].crits.len(), e.sets[0].crits.len());
        assert_eq!(back.examples, e.examples);
    }

    #[test]
    fn save_never_overwrites_and_load_reads_back() {
        let d = tmp("save");
        let e = entry_from_inputs(27, "First.", "", "", &sample_inputs(), "x").unwrap();
        let (code1, p1) = save(&d, &e).unwrap();
        let (code2, p2) = save(&d, &e).unwrap();
        assert_eq!(code1, 27);
        assert_eq!(code2, 28, "second save must take the next free code");
        assert_ne!(p1, p2);
        std::fs::write(d.join("broken.txt"), "[30]\nbogus: 1\n").unwrap();
        std::fs::write(d.join("notes.docx"), "ignored").unwrap();
        let (entries, errors) = load(&d);
        assert_eq!(entries.iter().map(|e| e.code).collect::<Vec<_>>(), vec![27, 28]);
        assert!(entries.iter().all(|e| e.source.is_some()));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("broken.txt"), "{}", errors[0]);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn load_skips_codes_already_taken() {
        let d = tmp("dup");
        std::fs::write(d.join("a.txt"), "[16]\ncomment: Clash.\nhb: low\n").unwrap();
        let (entries, errors) = load(&d);
        let lib = combined(library(), &entries);
        assert_eq!(lib.entries.iter().filter(|e| e.code == 16).count(), 1);
        let errs = combined_errors(library(), &entries);
        assert_eq!(errs.len() + errors.len(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn next_code_after_highest() {
        let e = entry_from_inputs(27, "X.", "", "", &sample_inputs(), "x").unwrap();
        let lib = combined(library(), &[e]);
        assert_eq!(next_code(&lib), 28);
        assert_eq!(next_code(library()), 27);
    }

    #[test]
    fn custom_comment_is_matched() {
        let e = entry_from_inputs(27, "New pattern seen.", "", "", &sample_inputs(), "x").unwrap();
        let lib = combined(library(), &[e]);
        let s = suggest(&lib, &sample_inputs());
        assert_eq!(s.top.iter().map(|e| e.code).collect::<Vec<_>>(), vec![27]);
    }

    #[test]
    fn date_label() {
        assert_eq!(civil_date(0), "1 Jan 1970");
        assert_eq!(civil_date(20_783), "26 Nov 2026");
    }
}
