//! Pure calculation logic, ported 1:1 from the JavaScript in the original HTML.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FieldVal {
    Blank,
    Valid(f64),
    Invalid,
}

impl FieldVal {
    pub fn is_valid(self) -> bool {
        matches!(self, Self::Valid(_))
    }

    pub fn value(self) -> Option<f64> {
        match self {
            Self::Valid(v) => Some(v),
            _ => None,
        }
    }
}

/// Equivalent of `getNumber`: blank → null, finite and > 0 → number, otherwise NaN.
pub fn parse_positive(raw: &str) -> FieldVal {
    match parse_free(raw) {
        FieldVal::Valid(v) if v > 0.0 => FieldVal::Valid(v),
        FieldVal::Blank => FieldVal::Blank,
        _ => FieldVal::Invalid,
    }
}

/// Equivalent of `numOrNull`: blank → null, finite → number, otherwise NaN.
pub fn parse_free(raw: &str) -> FieldVal {
    let s = raw.trim();
    if s.is_empty() {
        return FieldVal::Blank;
    }
    match s.parse::<f64>() {
        Ok(v) if v.is_finite() => FieldVal::Valid(v),
        _ => FieldVal::Invalid,
    }
}

#[derive(Clone, Copy)]
pub struct ParamDef {
    pub id: &'static str,
    pub name: &'static str,
    pub unit: &'static str,
    pub limit: f64,
    pub placeholders: [&'static str; 3],
}

pub const FBC_PARAMS: [ParamDef; 4] = [
    ParamDef {
        id: "wbc",
        name: "WBC",
        unit: "×10⁹/L",
        limit: 10.0,
        placeholders: ["e.g. 7.2", "e.g. 7.5", "e.g. 7.4"],
    },
    ParamDef {
        id: "rbc",
        name: "RBC",
        unit: "×10¹²/L",
        limit: 4.0,
        placeholders: ["e.g. 4.20", "e.g. 4.15", "e.g. 4.18"],
    },
    ParamDef {
        id: "hgb",
        name: "HGB",
        unit: "g/dL",
        limit: 4.0,
        placeholders: ["e.g. 13.5", "e.g. 13.7", "e.g. 13.6"],
    },
    ParamDef {
        id: "plt",
        name: "PLT",
        unit: "×10⁹/L",
        limit: 25.0,
        placeholders: ["e.g. 250", "e.g. 262", "e.g. 256"],
    },
];

pub const PTAPTT_PARAMS: [ParamDef; 2] = [
    ParamDef {
        id: "pt",
        name: "PT",
        unit: "seconds",
        limit: 5.0,
        placeholders: ["e.g. 12.4", "e.g. 12.6", "e.g. 12.5"],
    },
    ParamDef {
        id: "aptt",
        name: "APTT",
        unit: "seconds",
        limit: 5.0,
        placeholders: ["e.g. 31.2", "e.g. 31.8", "e.g. 31.5"],
    },
];

/// Relative percent difference used by the duplicate-run checker.
pub fn rpd(a: f64, b: f64) -> f64 {
    ((b - a) / a).abs() * 100.0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Badge {
    Pending,
    Acceptable,
    Flagged,
    CheckValue,
}

impl Badge {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::Acceptable => "Acceptable",
            Self::Flagged => "Flagged",
            Self::CheckValue => "Check value",
        }
    }
}

#[derive(Clone, Debug)]
pub struct PairOutcome {
    pub percent: String,
    pub badge: Badge,
}

pub fn pair_outcome(a: FieldVal, b: FieldVal, limit: f64) -> PairOutcome {
    match (a, b) {
        (FieldVal::Blank, _) | (_, FieldVal::Blank) => PairOutcome {
            percent: "—".to_owned(),
            badge: Badge::Pending,
        },
        (FieldVal::Invalid, _) | (_, FieldVal::Invalid) => PairOutcome {
            percent: "Invalid".to_owned(),
            badge: Badge::CheckValue,
        },
        (FieldVal::Valid(a), FieldVal::Valid(b)) => {
            let diff = rpd(a, b);
            let ok = diff <= limit + f64::EPSILON;
            PairOutcome {
                percent: format!("{diff:.2}%"),
                badge: if ok {
                    Badge::Acceptable
                } else {
                    Badge::Flagged
                },
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overall {
    Pending,
    Ok,
    Bad,
}

impl Overall {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "Awaiting results",
            Self::Ok => "All acceptable",
            Self::Bad => "Review flagged result",
        }
    }
}

pub fn overall(rows: &[PairOutcome]) -> Overall {
    if rows
        .iter()
        .any(|r| matches!(r.badge, Badge::Flagged | Badge::CheckValue))
    {
        Overall::Bad
    } else if !rows.is_empty() && rows.iter().all(|r| r.badge == Badge::Acceptable) {
        Overall::Ok
    } else {
        Overall::Pending
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gender {
    Male,
    Female,
}

impl Gender {
    pub fn label(self) -> &'static str {
        match self {
            Self::Male => "Male",
            Self::Female => "Female",
        }
    }

    pub fn threshold(self) -> f64 {
        match self {
            Self::Male => 41.0,
            Self::Female => 36.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Corrected {
    NotComputable,
    Nil,
    Value(f64),
}

#[derive(Clone, Debug, Default)]
pub struct ReticResult {
    pub corrected: Option<Corrected>,
    pub corrected_rounded: Option<f64>,
    pub corrected_was_rounded: bool,
    pub corrected_text: String,
    pub corrected_note: Option<String>,

    pub absolute_text: String,
    pub absolute_note: Option<String>,
    pub absolute_value: Option<f64>,

    pub rbc_rounded: Option<f64>,
    pub rbc_note: Option<String>,

    pub retic_rounded: Option<f64>,
    pub retic_was_rounded: bool,
}

// JavaScript's Math.round is floor(x + 0.5); Rust's f64::round is half-away-from-zero.
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

// The original code adds Number.EPSILON before rounding to avoid binary-float surprises.
pub fn round2(x: f64) -> f64 {
    js_round((x + f64::EPSILON) * 100.0) / 100.0
}

pub fn round1(x: f64) -> f64 {
    js_round((x + f64::EPSILON) * 10.0) / 10.0
}

pub fn is_rounded2(raw: f64, rounded: f64) -> bool {
    (raw - rounded).abs() > 1e-9
}

/// Equivalent of `String(+x.toPrecision(6))`.
pub fn fmt_raw(x: f64) -> String {
    if !x.is_finite() {
        return if x.is_nan() {
            "NaN".to_owned()
        } else if x > 0.0 {
            "Infinity".to_owned()
        } else {
            "-Infinity".to_owned()
        };
    }
    let rounded: f64 = format!("{x:.5e}").parse().unwrap_or(x);
    if rounded == 0.0 {
        "0".to_owned()
    } else {
        format!("{rounded}")
    }
}

pub fn fmt1(x: f64) -> String {
    format!("{x:.1}")
}

pub fn fmt2(x: f64) -> String {
    format!("{x:.2}")
}

pub fn compute_corrected(
    retic: FieldVal,
    hct: FieldVal,
    gender: Option<Gender>,
) -> Corrected {
    let (Some(retic), Some(hct), Some(gender)) = (retic.value(), hct.value(), gender) else {
        return Corrected::NotComputable;
    };
    if js_round(hct) < gender.threshold() {
        Corrected::Value(retic * (hct / 45.0))
    } else {
        Corrected::Nil
    }
}

pub fn compute_retic(
    retic: FieldVal,
    hct: FieldVal,
    rbc: FieldVal,
    gender: Option<Gender>,
) -> ReticResult {
    let mut out = ReticResult::default();

    let corrected = compute_corrected(retic, hct, gender);
    out.corrected = Some(corrected);

    let rbc_ok = rbc.is_valid();
    let r = rbc.value().unwrap_or(0.0);
    let rbc_rounded = rbc_ok.then(|| round1(r));
    out.rbc_rounded = rbc_rounded;
    if let Some(rr) = rbc_rounded {
        if is_rounded2(r, rr) {
            out.rbc_note = Some(format!("1 dp: {} → {}", fmt_raw(r), fmt1(rr)));
        }
    }

    match corrected {
        Corrected::NotComputable => {
            out.corrected_text = "—".to_owned();
            out.corrected_rounded = None;
        }
        Corrected::Nil => {
            out.corrected_text = "NIL".to_owned();
            out.corrected_rounded = None;
        }
        Corrected::Value(v) => {
            let cr = round2(v);
            out.corrected_rounded = Some(cr);
            out.corrected_was_rounded = is_rounded2(v, cr);
            out.corrected_text = fmt2(cr);
            if out.corrected_was_rounded {
                out.corrected_note = Some(format!(
                    "Rounded to 2 dp: {} → {}.",
                    fmt_raw(v),
                    fmt2(cr)
                ));
            }
        }
    }

    // Absolute retic needs both retic % and RBC.
    let (Some(retic_v), Some(rbc_r)) = (retic.value(), rbc_rounded) else {
        out.absolute_text = "—".to_owned();
        return out;
    };

    let (input, source_rounded_note) = match corrected {
        Corrected::Value(_) => {
            let cr = out.corrected_rounded.unwrap_or_else(|| round2(retic_v));
            (cr, None)
        }
        _ => {
            let rr = round2(retic_v);
            out.retic_rounded = Some(rr);
            out.retic_was_rounded = is_rounded2(retic_v, rr);
            let note = if out.retic_was_rounded {
                Some(format!(
                    "Retic % at 2 dp: {} → {}.",
                    fmt_raw(retic_v),
                    fmt2(rr)
                ))
            } else {
                None
            };
            (rr, note)
        }
    };

    let value = input * rbc_r * 10.0;
    out.absolute_value = Some(value);
    out.absolute_text = fmt1(value);
    if let Corrected::Value(_) = corrected {
        if out.corrected_was_rounded {
            out.absolute_note = Some(format!(
                "Using corrected retic % at 2 dp ({})",
                fmt2(input)
            ));
        }
    } else {
        out.absolute_note = source_rounded_note;
    }

    out
}

/// Counting-helper average: average of the entered attempts / 10.
pub fn average_attempts(attempts: &[String]) -> Option<f64> {
    let vals: Vec<f64> = attempts
        .iter()
        .filter_map(|s| parse_free(s).value())
        .collect();
    if vals.is_empty() {
        None
    } else {
        Some(vals.iter().sum::<f64>() / vals.len() as f64 / 10.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_difference() {
        let r = pair_outcome(FieldVal::Valid(7.2), FieldVal::Valid(7.5), 10.0);
        assert_eq!(r.percent, "4.17%");
        assert_eq!(r.badge, Badge::Acceptable);
        let r = pair_outcome(FieldVal::Valid(7.2), FieldVal::Valid(8.1), 10.0);
        assert_eq!(r.badge, Badge::Flagged);
    }

    #[test]
    fn overall_invalid_input_needs_review() {
        let rows = vec![
            pair_outcome(FieldVal::Valid(7.2), FieldVal::Valid(7.5), 10.0),
            pair_outcome(FieldVal::Valid(7.5), FieldVal::Invalid, 10.0),
        ];
        assert_eq!(overall(&rows), Overall::Bad);
    }

    #[test]
    fn corrected_retic() {
        let r = compute_retic(
            FieldVal::Valid(0.9),
            FieldVal::Valid(38.0),
            FieldVal::Valid(4.1),
            Some(Gender::Male),
        );
        assert_eq!(r.corrected_text, "0.76");
        assert_eq!(r.absolute_text, "31.2");
    }

    #[test]
    fn no_correction_uses_observed() {
        let r = compute_retic(
            FieldVal::Valid(1.25),
            FieldVal::Valid(45.0),
            FieldVal::Valid(4.15),
            Some(Gender::Male),
        );
        assert_eq!(r.corrected_text, "NIL");
        // 4.15 rounds to 4.2 at 1 dp (the original JS rounds the same way)
        assert_eq!(r.absolute_text, "52.5");
    }

    #[test]
    fn missing_gender_still_absolute() {
        let r = compute_retic(
            FieldVal::Valid(1.0),
            FieldVal::Valid(40.0),
            FieldVal::Valid(4.0),
            None,
        );
        assert_eq!(r.corrected_text, "—");
        assert_eq!(r.absolute_text, "40.0");
    }

    #[test]
    fn rounding_notes() {
        let r = compute_retic(
            FieldVal::Valid(0.955),
            FieldVal::Valid(38.0),
            FieldVal::Valid(4.14),
            Some(Gender::Male),
        );
        assert!(r.corrected_note.is_some());
        assert!(r.rbc_note.is_some());
    }
}
