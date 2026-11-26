//! HBEL comment library: data model and parser for `assets/hbel_comments.txt`.
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Rows and their options
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Row {
    Hba2,
    Peak,
    Hbf,
    Hb,
    Indices,
    Film,
    CWin,
    SWin,
    Hbh,
    Hba1c,
}

pub const ROWS: [Row; 10] = [Row::Hba2, Row::Peak, Row::Hbf, Row::Hb, Row::Indices, Row::Film, Row::CWin, Row::SWin, Row::Hbh, Row::Hba1c];

pub struct Opt {
    pub key: &'static str,
    pub label: &'static str,
    pub range: Option<Range>,
}

const fn r(lo: f64, lo_incl: bool, hi: f64, hi_incl: bool) -> Option<Range> {
    Some(Range { lo, lo_incl, hi, hi_incl })
}
const fn o(key: &'static str, label: &'static str) -> Opt {
    Opt { key, label, range: None }
}

static HBA2_OPTS: [Opt; 5] = [
    Opt { key: "low", label: "Low", range: r(0.0, true, 2.2, false) },
    Opt { key: "normal", label: "Normal", range: r(2.2, true, 3.5, false) },
    Opt { key: "borderline", label: "Borderline", range: r(3.5, true, 4.0, false) },
    Opt { key: "raised", label: "Raised", range: r(4.0, true, 10.0, true) },
    Opt { key: "over10", label: ">10", range: r(10.0, false, 100.0, true) },
];
static HBF_OPTS: [Opt; 4] = [
    Opt { key: "normal", label: "Normal", range: r(0.0, true, 1.3, true) },
    Opt { key: "raised", label: "Raised <5", range: r(1.3, false, 5.0, false) },
    Opt { key: "mid", label: "5–50", range: r(5.0, true, 50.0, true) },
    Opt { key: "over50", label: ">50", range: r(50.0, false, 100.0, true) },
];
static PEAK_OPTS: [Opt; 3] = [o("normal", "Normal"), o("split", "Split"), o("degraded", "Degradation / co-eluting")];
static HB_OPTS: [Opt; 3] = [o("normal", "Normal"), o("low", "Low"), o("markedly-low", "Markedly low (<7)")];
static IND_OPTS: [Opt; 3] = [o("normal", "Normal"), o("micro-hypo", "Micro/Hypo"), o("marked", "Markedly decreased")];
static FILM_OPTS: [Opt; 4] = [o("target", "Target cells"), o("sphero", "Spherocytes"), o("sickle", "Sickle cells"), o("poikilo", "Marked poikilocytosis")];
static WIN_OPTS: [Opt; 2] = [o("neg", "(−)"), o("present", "Present")];
static HBH_OPTS: [Opt; 2] = [o("not-detected", "Not detected"), o("detected", "Detected")];
static A1C_OPTS: [Opt; 2] = [o("normal", "Normal"), o("high", "High")];

impl Row {
    pub fn index(self) -> usize {
        ROWS.iter().position(|r| *r == self).unwrap()
    }
    pub fn key(self) -> &'static str {
        match self {
            Row::Hba2 => "hba2",
            Row::Peak => "peak",
            Row::Hbf => "hbf",
            Row::Hb => "hb",
            Row::Indices => "indices",
            Row::Film => "film",
            Row::CWin => "c",
            Row::SWin => "s",
            Row::Hbh => "hbh",
            Row::Hba1c => "hba1c",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Row::Hba2 => "HbA2 %",
            Row::Peak => "HbA2 peak",
            Row::Hbf => "HbF %",
            Row::Hb => "Hb",
            Row::Indices => "Indices",
            Row::Film => "Film",
            Row::CWin => "C window",
            Row::SWin => "S window",
            Row::Hbh => "HbH inclusions",
            Row::Hba1c => "HbA1c",
        }
    }
    pub fn options(self) -> &'static [Opt] {
        match self {
            Row::Hba2 => &HBA2_OPTS,
            Row::Peak => &PEAK_OPTS,
            Row::Hbf => &HBF_OPTS,
            Row::Hb => &HB_OPTS,
            Row::Indices => &IND_OPTS,
            Row::Film => &FILM_OPTS,
            Row::CWin | Row::SWin => &WIN_OPTS,
            Row::Hbh => &HBH_OPTS,
            Row::Hba1c => &A1C_OPTS,
        }
    }
    pub fn is_numeric(self) -> bool {
        matches!(self, Row::Hba2 | Row::Hbf)
    }
    pub fn is_multi(self) -> bool {
        self == Row::Film
    }
    fn from_key(k: &str) -> Option<Row> {
        ROWS.iter().copied().find(|r| r.key() == k)
    }
}

// ---------------------------------------------------------------------------
// Ranges
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Range {
    pub lo: f64,
    pub lo_incl: bool,
    pub hi: f64,
    pub hi_incl: bool,
}

impl Range {
    pub fn contains(&self, x: f64) -> bool {
        let above = if self.lo_incl { x >= self.lo } else { x > self.lo };
        let below = if self.hi_incl { x <= self.hi } else { x < self.hi };
        above && below
    }

    /// True when the two ranges share more than a single point.
    pub fn overlaps(&self, o: &Range) -> bool {
        self.lo.max(o.lo) < self.hi.min(o.hi)
    }

    /// `A..B` (inclusive), `A..<B`, `>A` (to 100) or `<B` (from 0).
    pub fn parse(s: &str) -> Option<Range> {
        let s = s.trim();
        let num = |t: &str| t.trim().parse::<f64>().ok().filter(|v| v.is_finite());
        if let Some(rest) = s.strip_prefix('>') {
            return Some(Range { lo: num(rest)?, lo_incl: false, hi: 100.0, hi_incl: true });
        }
        if let Some(rest) = s.strip_prefix('<') {
            return Some(Range { lo: 0.0, lo_incl: true, hi: num(rest)?, hi_incl: false });
        }
        let (a, b) = s.split_once("..")?;
        let (b, hi_incl) = match b.strip_prefix('<') {
            Some(b) => (b, false),
            None => (b, true),
        };
        let (lo, hi) = (num(a)?, num(b)?);
        if lo > hi {
            return None;
        }
        Some(Range { lo, lo_incl: true, hi, hi_incl })
    }

    pub fn display(&self) -> String {
        let f = |v: f64| crate::logic::fmt_js(v);
        if !self.lo_incl && self.hi >= 100.0 {
            format!(">{} %", f(self.lo))
        } else if self.lo <= 0.0 && !self.hi_incl {
            format!("<{} %", f(self.hi))
        } else if self.lo == self.hi {
            format!("{} %", f(self.lo))
        } else if self.hi_incl {
            format!("{}–{} %", f(self.lo), f(self.hi))
        } else {
            format!("{}–<{} %", f(self.lo), f(self.hi))
        }
    }
}

// ---------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Debug)]
pub enum Rule {
    Range(Range),
    /// Bitmask of option indices (any of).
    AnyOf(u8),
}

#[derive(Clone, Debug)]
pub struct Criterion {
    pub row: Row,
    pub rule: Rule,
    pub assumed: bool,
    pub key: bool,
}

#[derive(Clone, Debug, Default)]
pub struct CritSet {
    pub name: Option<String>,
    pub sheet: Vec<(String, String)>,
    pub crits: Vec<Criterion>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Link {
    pub label: String,
    pub url: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    Normal,
    Hbf,
    Thal,
    Variant,
}

pub const GROUPS: [Group; 4] = [Group::Normal, Group::Hbf, Group::Thal, Group::Variant];

impl Group {
    pub fn label(self) -> &'static str {
        match self {
            Group::Normal => "Normal & borderline",
            Group::Hbf => "HbF",
            Group::Thal => "Thalassaemia",
            Group::Variant => "Variants",
        }
    }
    fn from_key(k: &str) -> Option<Group> {
        match k {
            "normal" => Some(Group::Normal),
            "hbf" => Some(Group::Hbf),
            "thal" => Some(Group::Thal),
            "variant" => Some(Group::Variant),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Entry {
    pub code: u8,
    pub comment: String,
    pub groups: Vec<Group>,
    pub sets: Vec<CritSet>,
    pub remarks: Vec<String>,
    pub examples: Vec<String>,
    pub links: Vec<Link>,
    pub references: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Library {
    pub entries: Vec<Entry>,
    pub resources: Vec<Link>,
}

enum Section {
    None,
    Entry,
    Resources,
}

fn parse_link(v: &str) -> Option<Link> {
    let (label, url) = v.split_once('|')?;
    let (label, url) = (label.trim(), url.trim());
    if label.is_empty() || url.is_empty() {
        return None;
    }
    Some(Link { label: label.to_string(), url: url.to_string() })
}

fn finish(lib: &mut Library, cur: Option<Entry>, start: usize) -> Result<(), String> {
    if let Some(e) = cur {
        if e.comment.is_empty() {
            return Err(format!("line {}: code {} has no comment", start, e.code));
        }
        lib.entries.push(e);
    }
    Ok(())
}

pub fn parse(src: &str) -> Result<Library, String> {
    let mut lib = Library::default();
    let mut cur: Option<Entry> = None;
    let mut cur_start = 0;
    let mut section = Section::None;
    for (i, raw) in src.lines().enumerate() {
        let n = i + 1;
        let line = raw.trim_end_matches('\r').trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let err = |m: &str| format!("line {}: {}", n, m);
        if let Some(head) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            finish(&mut lib, cur.take(), cur_start)?;
            if head.trim() == "resources" {
                section = Section::Resources;
                continue;
            }
            let code: u8 = head.trim().parse().map_err(|_| err("bad section header"))?;
            if lib.entries.iter().any(|e| e.code == code) {
                return Err(err(&format!("duplicate code {}", code)));
            }
            cur = Some(Entry { code, sets: vec![CritSet::default()], ..Default::default() });
            cur_start = n;
            section = Section::Entry;
            continue;
        }
        let (k, v) = line.split_once(':').ok_or_else(|| err("expected `key: value`"))?;
        let (k, v) = (k.trim(), v.trim());
        match section {
            Section::None => return Err(err("line outside an entry")),
            Section::Resources => {
                if k != "link" {
                    return Err(err("only `link:` lines are allowed in [resources]"));
                }
                lib.resources.push(parse_link(v).ok_or_else(|| err("bad link (use `Label | url`)"))?);
                continue;
            }
            Section::Entry => {}
        }
        let e = cur.as_mut().unwrap();
        if let Some(col) = k.strip_prefix("sheet ") {
            e.sets.last_mut().unwrap().sheet.push((col.trim().to_string(), v.to_string()));
            continue;
        }
        match k {
            "comment" => {
                if !e.comment.is_empty() {
                    return Err(err("second `comment:`"));
                }
                e.comment = v.to_string();
            }
            "groups" => {
                for g in v.split(',').map(str::trim).filter(|g| !g.is_empty()) {
                    e.groups.push(Group::from_key(g).ok_or_else(|| err(&format!("unknown group `{}`", g)))?);
                }
            }
            "remark" => e.remarks.push(v.to_string()),
            "example" => e.examples.push(v.to_string()),
            "reference" => e.references.push(v.to_string()),
            "link" => e.links.push(parse_link(v).ok_or_else(|| err("bad link (use `Label | url`)"))?),
            "set" => {
                let first = e.sets.len() == 1 && e.sets[0].name.is_none() && e.sets[0].sheet.is_empty() && e.sets[0].crits.is_empty();
                let s = CritSet { name: Some(v.to_string()), ..Default::default() };
                if first {
                    e.sets[0] = s;
                } else {
                    e.sets.push(s);
                }
            }
            _ => {
                let row = Row::from_key(k).ok_or_else(|| err(&format!("unknown key `{}`", k)))?;
                // trailing flags: (assumed), (key), (assumed, key)
                let (body, flags) = match v.rfind('(') {
                    Some(p) if v.ends_with(')') => (v[..p].trim(), &v[p + 1..v.len() - 1]),
                    _ => (v, ""),
                };
                let mut assumed = false;
                let mut key = false;
                for f in flags.split(',').map(str::trim).filter(|f| !f.is_empty()) {
                    match f {
                        "assumed" => assumed = true,
                        "key" => key = true,
                        _ => return Err(err(&format!("unknown flag `{}`", f))),
                    }
                }
                let rule = if row.is_numeric() {
                    Rule::Range(Range::parse(body).ok_or_else(|| err(&format!("bad range `{}`", body)))?)
                } else {
                    let mut mask = 0u8;
                    for t in body.split(',').map(str::trim) {
                        let i = row.options().iter().position(|o| o.key == t).ok_or_else(|| err(&format!("unknown option `{}` for {}", t, k)))?;
                        mask |= 1 << i;
                    }
                    Rule::AnyOf(mask)
                };
                let set = e.sets.last_mut().unwrap();
                if set.crits.iter().any(|c| c.row == row) {
                    return Err(err(&format!("second `{}` criterion in one set", k)));
                }
                set.crits.push(Criterion { row, rule, assumed, key });
            }
        }
    }
    finish(&mut lib, cur, cur_start)?;
    Ok(lib)
}

static LIBRARY: OnceLock<Library> = OnceLock::new();

/// The embedded HBEL library (parsed once).
pub fn library() -> &'static Library {
    LIBRARY.get_or_init(|| match parse(include_str!("../assets/hbel_comments.txt")) {
        Ok(l) => l,
        Err(e) => panic!("assets/hbel_comments.txt: {}", e),
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    const MINI: &str = "# test\n[16]\ncomment: HbA present. HbA2 is borderline.\nsheet A2: Slightly elevated (3.5 to 3.9)\nsheet F: Normal\nhba2: 3.5..<4\npeak: normal (assumed)\n";

    #[test]
    fn range_parse_forms() {
        let r = Range::parse("3.5..<4").unwrap();
        assert_eq!(r, Range { lo: 3.5, lo_incl: true, hi: 4.0, hi_incl: false });
        let r = Range::parse(">10").unwrap();
        assert_eq!(r, Range { lo: 10.0, lo_incl: false, hi: 100.0, hi_incl: true });
        let r = Range::parse("<2.2").unwrap();
        assert_eq!(r, Range { lo: 0.0, lo_incl: true, hi: 2.2, hi_incl: false });
        let r = Range::parse("0.1..5").unwrap();
        assert!(r.contains(5.0) && r.contains(0.1) && !r.contains(5.01));
        assert!(Range::parse("abc").is_none());
    }

    #[test]
    fn range_overlap_needs_positive_length() {
        let p = |s| Range::parse(s).unwrap();
        assert!(!p("5..50").overlaps(&p("0.1..5")));
        assert!(p("0..1.3").overlaps(&p("1..30")));
        assert!(!p("0..<2.2").overlaps(&p("2.2..<3.5")));
    }

    #[test]
    fn parses_minimal_entry() {
        let lib = parse(MINI).unwrap();
        assert_eq!(lib.entries.len(), 1);
        let e = &lib.entries[0];
        assert_eq!(e.code, 16);
        assert_eq!(e.comment, "HbA present. HbA2 is borderline.");
        assert_eq!(e.sets.len(), 1);
        let s = &e.sets[0];
        assert!(s.name.is_none());
        assert_eq!(s.sheet, vec![("A2".to_string(), "Slightly elevated (3.5 to 3.9)".to_string()), ("F".to_string(), "Normal".to_string())]);
        let a2 = s.crits.iter().find(|c| c.row == Row::Hba2).unwrap();
        assert_eq!(a2.rule, Rule::Range(Range::parse("3.5..<4").unwrap()));
        let pk = s.crits.iter().find(|c| c.row == Row::Peak).unwrap();
        assert!(pk.assumed && !pk.key);
        assert_eq!(pk.rule, Rule::AnyOf(0b001));
    }

    #[test]
    fn parses_named_sets_and_links() {
        let src = "[12]\ncomment: X.\nlink: A | https://x\nset: Homozygous\nhbf: 100..100\nset: Heterozygous\nhbf: 1..30\nhba1c: high (assumed, key)\n";
        let lib = parse(src).unwrap();
        let e = &lib.entries[0];
        assert_eq!(e.sets.len(), 2);
        assert_eq!(e.sets[0].name.as_deref(), Some("Homozygous"));
        assert_eq!(e.sets[1].name.as_deref(), Some("Heterozygous"));
        assert_eq!(e.links, vec![Link { label: "A".into(), url: "https://x".into() }]);
        let k = e.sets[1].crits.iter().find(|c| c.row == Row::Hba1c).unwrap();
        assert!(k.assumed && k.key);
    }

    #[test]
    fn crlf_parses_same() {
        let a = parse(MINI).unwrap();
        let b = parse(&MINI.replace('\n', "\r\n")).unwrap();
        assert_eq!(a.entries.len(), b.entries.len());
        for (x, y) in a.entries.iter().zip(&b.entries) {
            assert_eq!(x.code, y.code);
            assert_eq!(x.comment, y.comment);
            assert_eq!(x.sets[0].crits.len(), y.sets[0].crits.len());
            assert_eq!(x.sets[0].sheet, y.sets[0].sheet);
        }
    }

    #[test]
    fn errors_name_line() {
        let err = parse("[16]\ncomment: X.\nbogus: 1\n").unwrap_err();
        assert!(err.contains("line 3"), "{err}");
        assert!(parse("[16]\ncomment: X.\n[16]\ncomment: Y.\n").is_err());
        assert!(parse("[16]\nhb: normal\n").is_err(), "missing comment");
        assert!(parse("[16]\ncomment: X.\nhb: sometimes\n").is_err(), "unknown option");
        assert!(parse("hb: normal\n").is_err(), "outside entry");
    }
}
