//! Calculation helpers. These mirror the JavaScript in the HTML version exactly
//! (Math.round, toFixed, toPrecision, Number()).

/// Result of reading a text field.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Num {
    Empty,
    Invalid,
    Val(f64),
}

impl Num {
    pub fn ok(self) -> Option<f64> {
        match self {
            Num::Val(v) => Some(v),
            _ => None,
        }
    }
}

fn js_number(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    // Rust accepts "inf"/"nan"; JS `Number` does not (and the fields never allow letters).
    if t.chars().any(|c| c.is_alphabetic() && c != 'e' && c != 'E') {
        return Some(f64::NAN);
    }
    Some(t.parse::<f64>().unwrap_or(f64::NAN))
}

/// `getNumber` (duplicate-run checkers): blank → Empty, must be finite and > 0.
pub fn parse_positive(s: &str) -> Num {
    match js_number(s) {
        None => Num::Empty,
        Some(v) if v.is_finite() && v > 0.0 => Num::Val(v),
        Some(_) => Num::Invalid,
    }
}

/// `numOrNull` (retic / dilution): blank → Empty, any finite number.
pub fn parse_number(s: &str) -> Num {
    match js_number(s) {
        None => Num::Empty,
        Some(v) if v.is_finite() => Num::Val(v),
        Some(_) => Num::Invalid,
    }
}

/// Relative percent difference between two runs.
pub fn rpd(a: f64, b: f64) -> f64 {
    ((b - a) / a).abs() * 100.0
}

/// JavaScript `Math.round` (halves go towards +∞).
pub fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

pub fn round2(x: f64) -> f64 {
    js_round((x + f64::EPSILON) * 100.0) / 100.0
}

pub fn round1(x: f64) -> f64 {
    js_round((x + f64::EPSILON) * 10.0) / 10.0
}

pub fn is_rounded(raw: f64, rounded: f64) -> bool {
    (raw - rounded).abs() > 1e-9
}

/// JavaScript `Number.prototype.toFixed` (exact decimal, ties away from zero).
pub fn to_fixed(x: f64, n: usize) -> String {
    if !x.is_finite() {
        return x.to_string();
    }
    let neg = x < 0.0;
    let ax = x.abs();
    let wide = format!("{:.*}", n + 30, ax);
    let dot = wide.find('.').unwrap();
    let tail = &wide[dot + 1 + n..];
    let is_tie = tail.starts_with('5') && tail[1..].bytes().all(|b| b == b'0');
    let mut s = if is_tie {
        // Round the exact tie up (Rust's formatter would round half to even).
        let mut digits: Vec<u8> = wide[..dot + 1 + n].bytes().collect();
        let mut i = digits.len();
        loop {
            if i == 0 {
                digits.insert(0, b'1');
                break;
            }
            i -= 1;
            match digits[i] {
                b'.' => continue,
                b'9' => digits[i] = b'0',
                d => {
                    digits[i] = d + 1;
                    break;
                }
            }
        }
        let mut s = String::from_utf8(digits).unwrap();
        if n == 0 {
            s.pop(); // trailing '.'
        }
        s
    } else {
        format!("{:.*}", n, ax)
    };
    let is_zero = s.bytes().all(|b| b == b'0' || b == b'.');
    if neg && !is_zero {
        s.insert(0, '-');
    }
    s
}

/// `String(+x.toPrecision(6))`
pub fn fmt_raw(x: f64) -> String {
    if x == 0.0 || !x.is_finite() {
        return "0".into();
    }
    let e = format!("{:.5e}", x);
    let v: f64 = e.parse().unwrap_or(x);
    fmt_js(v)
}

/// `String(number)` for ordinary magnitudes.
pub fn fmt_js(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    format!("{}", v)
}

// ---------- duplicate-run checker -------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Status {
    Pending,
    Ok,
    Bad,
}

pub struct PairResult {
    pub percent: String,
    pub badge: &'static str,
    pub status: Status,
}

pub fn pair_result(a: Num, b: Num, limit: f64) -> PairResult {
    if a == Num::Empty || b == Num::Empty {
        return PairResult { percent: "—".into(), badge: "Pending", status: Status::Pending };
    }
    match (a, b) {
        (Num::Val(a), Num::Val(b)) => {
            let d = rpd(a, b);
            let ok = d <= limit + f64::EPSILON;
            PairResult {
                percent: format!("{}%", to_fixed(d, 2)),
                badge: if ok { "Acceptable" } else { "Flagged" },
                status: if ok { Status::Ok } else { Status::Bad },
            }
        }
        _ => PairResult { percent: "Invalid".into(), badge: "Check value", status: Status::Bad },
    }
}

// ---------- reticulocyte ------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Gender {
    Male,
    Female,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Corrected {
    NotComputable,
    Nil,
    Value(f64),
}

pub fn computed_corrected(retic: Num, hct: Num, gender: Option<Gender>) -> Corrected {
    match (retic, hct, gender) {
        (Num::Val(r), Num::Val(h), Some(g)) => {
            let thr = if g == Gender::Male { 41.0 } else { 36.0 };
            if js_round(h) < thr {
                Corrected::Value(r * (h / 45.0))
            } else {
                Corrected::Nil
            }
        }
        _ => Corrected::NotComputable,
    }
}

pub fn average_attempts(attempts: &[String]) -> Option<f64> {
    let vals: Vec<f64> = attempts.iter().filter_map(|s| parse_number(s).ok()).collect();
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
    fn to_fixed_matches_js() {
        assert_eq!(to_fixed(0.125, 2), "0.13");
        assert_eq!(to_fixed(2.5, 0), "3");
        assert_eq!(to_fixed(1.005, 2), "1.00");
        assert_eq!(to_fixed(0.93333333, 2), "0.93");
        assert_eq!(to_fixed(9.995, 2), "9.99"); // 9.995 is just below the tie in binary
        assert_eq!(to_fixed(0.0, 2), "0.00");
        assert_eq!(to_fixed(99.5, 0), "100");
    }

    #[test]
    fn raw_format() {
        assert_eq!(fmt_raw(0.9333333333), "0.933333");
        assert_eq!(fmt_raw(38.0), "38");
        assert_eq!(fmt_raw(0.9), "0.9");
    }

    #[test]
    fn rpd_and_status() {
        let r = pair_result(Num::Val(7.2), Num::Val(7.5), 10.0);
        assert_eq!(r.percent, "4.17%");
        assert_eq!(r.status, Status::Ok);
        let r = pair_result(Num::Val(250.0), Num::Val(330.0), 25.0);
        assert_eq!(r.status, Status::Bad);
    }

    #[test]
    fn corrected_rules() {
        let c = computed_corrected(Num::Val(0.9), Num::Val(38.0), Some(Gender::Male));
        assert_eq!(c, Corrected::Value(0.9 * (38.0 / 45.0)));
        let c = computed_corrected(Num::Val(0.9), Num::Val(41.2), Some(Gender::Male));
        assert_eq!(c, Corrected::Nil); // round(41.2)=41 is not < 41
        let c = computed_corrected(Num::Val(0.9), Num::Val(38.0), None);
        assert_eq!(c, Corrected::NotComputable);
    }
}
