//! HBEL matching: turns the user's entries into a ranked comment suggestion.
use crate::hbel_data::{Criterion, Entry, Library, Range, Row, Rule, ROWS};
use crate::logic::{parse_number, Num};

/// What the user has entered on the HBEL tab. `sel` is indexed by `Row::index()`.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Inputs {
    pub hba2: String,
    pub hbf: String,
    pub sel: [Option<usize>; 10],
    pub film: u8,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NumState {
    Empty,
    Valid(f64),
    Invalid,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Value {
    Num(f64),
    Span(Range),
    Opts(u8),
}

impl Inputs {
    fn text(&self, row: Row) -> Option<&str> {
        match row {
            Row::Hba2 => Some(&self.hba2),
            Row::Hbf => Some(&self.hbf),
            _ => None,
        }
    }

    /// State of a number box. Valid only for finite values within 0–100 %.
    pub fn number(&self, row: Row) -> NumState {
        match self.text(row).map(parse_number) {
            None | Some(Num::Empty) => NumState::Empty,
            Some(Num::Val(v)) if (0.0..=100.0).contains(&v) => NumState::Valid(v),
            Some(_) => NumState::Invalid,
        }
    }

    pub fn value(&self, row: Row) -> Option<Value> {
        if row.is_multi() {
            return (self.film != 0).then_some(Value::Opts(self.film));
        }
        if row.is_numeric() {
            match self.number(row) {
                NumState::Valid(x) => return Some(Value::Num(x)),
                NumState::Invalid => return None,
                NumState::Empty => {}
            }
            return self.sel[row.index()].and_then(|i| row.options()[i].range).map(Value::Span);
        }
        self.sel[row.index()].map(|i| Value::Opts(1 << i))
    }

    pub fn chip_selected(&self, row: Row, i: usize) -> bool {
        if row.is_multi() {
            return self.film & (1 << i) != 0;
        }
        if row.is_numeric() {
            match self.number(row) {
                NumState::Valid(x) => return row.options()[i].range.is_some_and(|r| r.contains(x)),
                NumState::Invalid => return false,
                NumState::Empty => {}
            }
        }
        self.sel[row.index()] == Some(i)
    }

    /// Toggle a chip. On HbA2 / HbF this also clears the typed number.
    pub fn click_chip(&mut self, row: Row, i: usize) {
        if row.is_multi() {
            self.film ^= 1 << i;
            return;
        }
        let was = self.chip_selected(row, i);
        match row {
            Row::Hba2 => self.hba2.clear(),
            Row::Hbf => self.hbf.clear(),
            _ => {}
        }
        self.sel[row.index()] = if was { None } else { Some(i) };
    }

    /// Call after the user edits a number box: typing replaces any chip choice.
    pub fn number_edited(&mut self, row: Row) {
        if row.is_numeric() {
            self.sel[row.index()] = None;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.hba2.trim().is_empty() && self.hbf.trim().is_empty() && self.film == 0 && self.sel.iter().all(Option::is_none)
    }

    pub fn clear(&mut self) {
        *self = Inputs::default();
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Outcome {
    Match,
    Conflict,
    Neutral,
    KeyUnmet,
}

#[derive(Clone, Debug)]
pub struct Eval {
    pub code: u8,
    /// Index of the criteria set used (code 12 has two).
    pub set: usize,
    pub matches: usize,
    pub conflicts: Vec<Row>,
    pub unmet: Vec<Row>,
    /// Rows with a sheet (non-assumed) criterion the user has not entered.
    pub blank: Vec<Row>,
    pub rows: Vec<(Row, Outcome)>,
}

impl Eval {
    pub fn fits(&self) -> bool {
        self.conflicts.is_empty() && self.unmet.is_empty() && self.matches >= 1
    }
    fn near_miss(&self) -> bool {
        !self.fits() && self.conflicts.len() + self.unmet.len() == 1 && self.matches >= 1
    }
    pub fn outcome(&self, row: Row) -> Option<Outcome> {
        self.rows.iter().find(|(r, _)| *r == row).map(|(_, o)| *o)
    }
}

fn outcome(c: &Criterion, v: Option<Value>) -> Outcome {
    let Some(v) = v else {
        return if c.key { Outcome::KeyUnmet } else { Outcome::Neutral };
    };
    let hit = match (&c.rule, v) {
        (Rule::Range(r), Value::Num(x)) => r.contains(x),
        // a single-point criterion (HbF 100 %) matches any chip that contains it
        (Rule::Range(r), Value::Span(s)) => r.overlaps(&s) || (r.lo == r.hi && s.contains(r.lo)),
        (Rule::AnyOf(m), Value::Opts(o)) => m & o != 0,
        _ => false,
    };
    if hit {
        Outcome::Match
    } else if c.row.is_multi() {
        Outcome::Neutral
    } else {
        Outcome::Conflict
    }
}

pub fn evaluate(e: &Entry, inp: &Inputs) -> Eval {
    let mut best: Option<Eval> = None;
    for (si, set) in e.sets.iter().enumerate() {
        let mut ev = Eval { code: e.code, set: si, matches: 0, conflicts: vec![], unmet: vec![], blank: vec![], rows: vec![] };
        for c in &set.crits {
            let v = inp.value(c.row);
            let o = outcome(c, v);
            match o {
                Outcome::Match => ev.matches += 1,
                Outcome::Conflict => ev.conflicts.push(c.row),
                Outcome::KeyUnmet => ev.unmet.push(c.row),
                Outcome::Neutral => {}
            }
            if v.is_none() && !c.assumed {
                ev.blank.push(c.row);
            }
            ev.rows.push((c.row, o));
        }
        let score = |x: &Eval| (x.fits(), x.matches, std::cmp::Reverse(x.conflicts.len() + x.unmet.len()));
        if best.as_ref().map_or(true, |b| score(&ev) > score(b)) {
            best = Some(ev);
        }
    }
    best.expect("entry has at least one criteria set")
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    Empty,
    Fits,
    NoFit,
}

#[derive(Clone, Debug)]
pub struct Suggestion {
    pub state: State,
    pub top: Vec<Eval>,
    pub also: Vec<Eval>,
}

pub fn suggest(lib: &Library, inp: &Inputs) -> Suggestion {
    if inp.is_empty() {
        return Suggestion { state: State::Empty, top: vec![], also: vec![] };
    }
    let all: Vec<Eval> = lib.entries.iter().map(|e| evaluate(e, inp)).collect();
    let order = |a: &Eval, b: &Eval| b.matches.cmp(&a.matches).then(a.code.cmp(&b.code));
    let mut fits: Vec<Eval> = all.iter().filter(|e| e.fits()).cloned().collect();
    fits.sort_by(order);
    let best = fits.first().map_or(0, |e| e.matches);
    let (top, rest): (Vec<Eval>, Vec<Eval>) = fits.into_iter().partition(|e| e.matches == best);
    let mut near: Vec<Eval> = all.iter().filter(|e| e.near_miss()).cloned().collect();
    near.sort_by(order);
    let also = rest.into_iter().chain(near).take(3).collect();
    let state = if top.is_empty() { State::NoFit } else { State::Fits };
    Suggestion { state, top, also }
}

/// Short row name for sentences ("HbA2", "Hb", "HbH inclusions").
pub fn row_name(row: Row) -> &'static str {
    row.label().trim_end_matches(" %")
}

pub fn describe(c: &Criterion) -> String {
    let rule = match &c.rule {
        Rule::Range(r) => r.display(),
        Rule::AnyOf(m) => c
            .row
            .options()
            .iter()
            .enumerate()
            .filter(|(i, _)| m & (1 << i) != 0)
            .map(|(_, o)| o.label)
            .collect::<Vec<_>>()
            .join(" or "),
    };
    format!("{}: {}", row_name(c.row), rule)
}

pub fn hint(ev: &Eval, e: &Entry) -> Option<String> {
    let set = &e.sets[ev.set];
    let crit = |row: Row| set.crits.iter().find(|c| c.row == row);
    if ev.near_miss() {
        let row = ev.conflicts.first().or(ev.unmet.first())?;
        return crit(*row).map(|c| format!("needs {}", describe(c)));
    }
    if ev.fits() && !ev.blank.is_empty() {
        let parts: Vec<String> = ROWS.iter().filter(|r| ev.blank.contains(r)).filter_map(|r| crit(*r)).map(describe).collect();
        return Some(format!("also expects {}", parts.join(", ")));
    }
    None
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::hbel_data::{library, Row};

    /// "hba2=2.8 hb=normal film=target,sphero"
    fn inp(s: &str) -> Inputs {
        let mut i = Inputs::default();
        for kv in s.split_whitespace() {
            let (k, v) = kv.split_once('=').unwrap();
            let row = crate::hbel_data::ROWS.iter().copied().find(|r| r.key() == k).unwrap();
            match row {
                Row::Hba2 => i.hba2 = v.to_string(),
                Row::Hbf => i.hbf = v.to_string(),
                _ => {
                    for t in v.split(',') {
                        let idx = row.options().iter().position(|o| o.key == t).unwrap_or_else(|| panic!("{k}={t}"));
                        i.click_chip(row, idx);
                    }
                }
            }
        }
        i
    }

    fn top(s: &str) -> Vec<u8> {
        suggest(library(), &inp(s)).top.iter().map(|e| e.code).collect()
    }

    #[test]
    fn canonical_case_per_code() {
        let cases: [(u8, &str); 26] = [
            (1, "hba2=2.8 hbf=0.5 hb=normal indices=normal c=neg s=neg peak=normal"),
            (3, "hba2=2.8 hbf=0.5 hb=low indices=micro-hypo c=neg s=neg hbh=not-detected"),
            (4, "hba2=2.8 hbf=0.5 hb=low indices=micro-hypo c=neg s=neg hbh=detected"),
            (5, "hba2=5.2 hbf=2 hb=low indices=micro-hypo c=neg s=neg hbh=not-detected"),
            (6, "hba2=5.5 hbf=20 hb=low indices=micro-hypo"),
            (7, "hba2=1.5 hbf=94 hb=markedly-low indices=marked"),
            (8, "hba2=15 peak=degraded hbh=not-detected"),
            (9, "c=present s=neg peak=normal"),
            (10, "s=present c=neg film=sickle peak=normal"),
            (11, "c=present s=present peak=normal"),
            (12, "hba2=0.5 hbf=100 indices=normal"),
            (12, "hba2=1.8 hbf=20 hb=normal indices=normal"),
            (13, "hba2=28 hbf=0.5 peak=normal"),
            (14, "hba2=92 hbf=0.8 peak=normal"),
            (15, "hba2=2.8 hbf=3 hb=normal indices=normal hba1c=high"),
            (16, "hba2=3.7 hbf=0.5 hb=normal indices=normal"),
            (17, "hba2=2.8 hbf=3 hb=normal indices=normal"),
            (18, "hbf=8 peak=degraded"),
            (19, "hba2=1.8 hbf=0.5 hb=normal indices=normal"),
            (20, "hba2=3.7 hbf=3 hb=normal indices=normal"),
            (21, "hba2=2.8 hbf=3 hb=low indices=micro-hypo"),
            (22, "peak=split hbf=0.5 hb=normal indices=normal hbh=not-detected"),
            (23, "hba2=2.8 hbf=3 hb=low indices=normal"),
            (24, "hba2=45 hbf=25 hb=markedly-low indices=marked"),
            (25, "hba2=1.8 hbf=0.5 hb=low indices=normal"),
            (26, "hba2=15 peak=degraded hbh=detected"),
        ];
        for (code, s) in cases {
            assert_eq!(top(s), vec![code], "case {s}");
        }
        let lib = library();
        assert_eq!(suggest(lib, &inp("hba2=0.5 hbf=100 indices=normal")).top[0].set, 0);
        assert_eq!(suggest(lib, &inp("hba2=1.8 hbf=20 hb=normal indices=normal")).top[0].set, 1);
    }

    #[test]
    fn sanity_behaviours() {
        let lib = library();
        let s = suggest(lib, &inp("hba2=2.8 hbf=3"));
        assert_eq!(s.top.iter().map(|e| e.code).collect::<Vec<_>>(), vec![17, 21, 23]);
        let entry = |c: u8| lib.entries.iter().find(|e| e.code == c).unwrap();
        let h21 = hint(&s.top[1], entry(21)).unwrap();
        assert!(h21.contains("Hb: Low"), "{h21}");
        assert_eq!(hint(&s.top[2], entry(23)), None);
        assert_eq!(top("hba2=5 hbf=5.0 hb=low indices=micro-hypo"), vec![5, 6]);
        assert_eq!(top("hba2=15 peak=degraded hbh=not-detected"), vec![8]);
        assert_eq!(top("hba2=3.7 hbf=0.5"), vec![16]);
        assert_eq!(top("hba2=3.7 hbf=3"), vec![20]);
    }

    #[test]
    fn empty_and_nofit() {
        assert_eq!(suggest(library(), &Inputs::default()).state, State::Empty);
        let s = suggest(library(), &inp("hba2=9"));
        assert_eq!(s.state, State::NoFit);
        assert!(s.top.is_empty());
    }

    #[test]
    fn chip_boundaries() {
        let sel = |row: Row, v: &str| -> usize {
            let mut i = Inputs::default();
            match row {
                Row::Hba2 => i.hba2 = v.into(),
                _ => i.hbf = v.into(),
            }
            let n: Vec<usize> = (0..row.options().len()).filter(|&k| i.chip_selected(row, k)).collect();
            assert_eq!(n.len(), 1, "{v}: {n:?}");
            n[0]
        };
        let key = |row: Row, v: &str| row.options()[sel(row, v)].key;
        for (v, k) in [("2.19", "low"), ("2.2", "normal"), ("3.4", "normal"), ("3.45", "normal"), ("3.5", "borderline"), ("3.99", "borderline"), ("4.0", "raised"), ("10", "raised"), ("10.1", "over10")] {
            assert_eq!(key(Row::Hba2, v), k, "HbA2 {v}");
        }
        for (v, k) in [("1.3", "normal"), ("1.31", "raised"), ("4.99", "raised"), ("5", "mid"), ("50", "mid"), ("50.1", "over50")] {
            assert_eq!(key(Row::Hbf, v), k, "HbF {v}");
        }
    }

    #[test]
    fn invalid_numbers_ignored() {
        for v in ["3,5", "3.5%", "-1", "150"] {
            let i = Inputs { hba2: v.into(), ..Default::default() };
            assert_eq!(i.number(Row::Hba2), NumState::Invalid, "{v}");
            assert!(i.value(Row::Hba2).is_none(), "{v}");
            assert!((0..5).all(|k| !i.chip_selected(Row::Hba2, k)), "{v}");
        }
        assert_eq!(Inputs::default().number(Row::Hba2), NumState::Empty);
    }

    #[test]
    fn chip_click_clears_number_and_toggles() {
        let mut i = Inputs { hba2: "3.7".into(), ..Default::default() };
        i.click_chip(Row::Hba2, 1);
        assert_eq!(i.hba2, "");
        assert_eq!(i.sel[0], Some(1));
        i.click_chip(Row::Hba2, 1);
        assert_eq!(i.sel[0], None);
        i.click_chip(Row::Film, 0);
        i.click_chip(Row::Film, 2);
        assert_eq!(i.film, 0b101);
        i.click_chip(Row::Film, 0);
        assert_eq!(i.film, 0b100);
        assert!(!i.is_empty());
        i.clear();
        assert!(i.is_empty());
    }

    #[test]
    fn hint_texts() {
        let lib = library();
        let s = suggest(lib, &inp("hba2=5.5 hbf=20 hb=normal indices=micro-hypo"));
        let six = s.also.iter().find(|e| e.code == 6).expect("code 6 in also");
        let entry = lib.entries.iter().find(|e| e.code == 6).unwrap();
        assert_eq!(hint(six, entry).as_deref(), Some("needs Hb: Low"));
        assert!(s.also.len() <= 3);
    }

    #[test]
    fn code12_homozygous_reachable_by_chips() {
        let mut i = Inputs::default();
        i.click_chip(Row::Hba2, 0); // Low
        i.click_chip(Row::Hbf, 3); // >50
        i.click_chip(Row::Indices, 0); // Normal
        let s = suggest(library(), &i);
        assert_eq!(s.top.iter().map(|e| (e.code, e.set)).collect::<Vec<_>>(), vec![(12, 0)]);
    }

    #[test]
    fn deleting_number_does_not_restore_old_chip() {
        let mut i = Inputs::default();
        i.click_chip(Row::Hba2, 1); // Normal
        i.hba2 = "3.7".into();
        i.number_edited(Row::Hba2);
        i.hba2.clear();
        i.number_edited(Row::Hba2);
        assert!((0..5).all(|k| !i.chip_selected(Row::Hba2, k)));
        assert!(i.value(Row::Hba2).is_none());
    }
}
