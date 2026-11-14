//! The tool panels: FBC (with dilution) / PT-APTT run sheets and Reticulocyte.
use crate::app::*;
use crate::logic::*;
use crate::theme::{hex, rgba, Palette, Role, Theme};
use crate::ui::*;
use eframe::egui::{self, pos2, vec2, Align, Align2, Color32, CornerRadius, Id, Layout, Pos2, Rect, Sense, Stroke, Ui, UiBuilder};

fn status_colors(th: &Theme, s: Status) -> Color32 {
    match s {
        Status::Ok => th.ok,
        Status::Bad => th.bad,
        Status::Pending => th.muted,
    }
}

// ---------------------------------------------------------------------------
// Duplicate-run checkers (FBC, PT / APTT)
// ---------------------------------------------------------------------------

fn run_pairs(third: bool) -> &'static [(usize, usize)] {
    if third {
        &[(0, 1), (0, 2), (1, 2)]
    } else {
        &[(0, 1)]
    }
}

/// One parameter's verdict and which runs to point at when it is flagged.
struct ParamEval {
    status: Status,
    flagged_runs: [bool; 3],
}

fn eval_param(p: &ParamDef, vals: &[String; 3], third: bool) -> ParamEval {
    let mut statuses = vec![];
    let mut failing: Vec<(usize, usize)> = vec![];
    for &(a, b) in run_pairs(third) {
        let s = pair_result(parse_positive(&vals[a]), parse_positive(&vals[b]), p.limit).status;
        if s == Status::Bad {
            failing.push((a, b));
        }
        statuses.push(s);
    }
    let status = if statuses.contains(&Status::Bad) {
        Status::Bad
    } else if statuses.iter().all(|s| *s == Status::Ok) {
        Status::Ok
    } else {
        Status::Pending
    };
    // Point at the run common to every failing comparison (the outlier); if
    // no single run explains it, mark every run involved.
    let mut flagged_runs = [false; 3];
    if !failing.is_empty() {
        let common: Vec<usize> = (0..3).filter(|r| failing.iter().all(|(a, b)| a == r || b == r)).collect();
        if common.len() == 1 {
            flagged_runs[common[0]] = true;
        } else {
            for (a, b) in &failing {
                flagged_runs[*a] = true;
                flagged_runs[*b] = true;
            }
        }
    }
    ParamEval { status, flagged_runs }
}

/// "HGB", "HGB and PLT", "WBC, HGB and PLT".
fn join_and(names: &[&str]) -> String {
    match names.len() {
        0 => String::new(),
        1 => names[0].to_string(),
        n => format!("{} and {}", names[..n - 1].join(", "), names[n - 1]),
    }
}

/// The checker's headline: what the technologist has to do next.
fn overall_message(ck: &Checker, evals: &[ParamEval]) -> (Status, String) {
    let flagged: Vec<&str> = ck.params.iter().zip(evals).filter(|(_, e)| e.status == Status::Bad).map(|(p, _)| p.name).collect();
    if !flagged.is_empty() {
        return (Status::Bad, format!("Review {}", join_and(&flagged)));
    }
    if evals.iter().all(|e| e.status == Status::Ok) {
        return (Status::Ok, "All acceptable".into());
    }
    let n = if ck.third { 3 } else { 2 };
    let any_entered = ck.vals.iter().any(|v| v[..n].iter().any(|s| !s.trim().is_empty()));
    if !any_entered {
        let runs = if ck.third { "Enter runs 1, 2 and 3" } else { "Enter run 1 and run 2" };
        return (Status::Pending, runs.into());
    }
    let waiting: Vec<&str> = ck.params.iter().zip(evals).filter(|(_, e)| e.status == Status::Pending).map(|(p, _)| p.name).collect();
    (Status::Pending, format!("Waiting for {}", join_and(&waiting)))
}

// --- Run sheet geometry -----------------------------------------------------
// One row per parameter: name, limit, the runs (narrow: values are at most
// five characters) and the comparisons, all on one line.

const ROW_H: f32 = 46.0;
const CELL_H: f32 = 32.0;
const RUN_W: f32 = 84.0;
/// Run boxes when space is tight: still fits "13.75" comfortably.
const RUN_W_COMPACT: f32 = 74.0;
const RUN_GAP: f32 = 10.0;
const COL_GAP: f32 = 22.0;
const HEAD_ROW: f32 = 26.0;
/// Extra height of a row whose comparisons wrap under the runs.
const STACK_H: f32 = 26.0;
/// Card chrome around the body: 1 px edge, 20 px padding, each side.
const CARD_CHROME: f32 = 42.0;

/// Left inset of the text inside a `CELL_H` field, so headings and computed
/// values line up with the typed digits.
fn cell_inset(th: &Theme) -> f32 {
    if th.palette == Palette::Genshin { (CELL_H * 0.45).min(18.0) } else { 11.0 }
}

/// A small column heading: sentence case, quiet.
fn col_head(ui: &Ui, th: &Theme, pos: Pos2, anchor: Align2, s: &str, color: Color32) -> Rect {
    if th.palette == Palette::Cyberpunk {
        text_ls(ui, th, pos, anchor, &s.to_uppercase(), Role::SansB, 11.0, color, 0.08)
    } else {
        text(ui, th, pos, anchor, s, Role::SansB, 12.0, color)
    }
}

/// Width `badge` will take for this label.
fn badge_width(ui: &Ui, th: &Theme, s: &str) -> f32 {
    if th.palette == Palette::Cyberpunk {
        galley(ui, th, &s.to_uppercase(), Role::SansB, 12.0, th.text, 0.05, f32::INFINITY).size().x + 26.0
    } else {
        text_width(ui, th, s, Role::SansB, 12.0) + 17.0
    }
}

/// Hairline between sheet rows, in the palette's own rule colour.
fn sheet_rule(th: &Theme) -> Color32 {
    match th.palette {
        Palette::Genshin => rgba(th.gold, 0.32),
        Palette::Cyberpunk => rgba(th.accent, 0.22),
        _ => th.line,
    }
}

struct Sheet {
    limit_x: f32,
    runs_x: f32,
    cmp_x: f32,
    pct_w: f32,
    cmp_w: f32,
    run_w: f32,
    /// Comparisons say "Acceptable" / "Check value" beside the symbol; the
    /// compact form keeps only the ✓ / ! symbol.
    words: bool,
    /// Inner width with everything on one line.
    inner: f32,
}

/// The run sheet in the roomiest form that fits `avail`: full words, then
/// symbols only, then narrower run boxes. If even that is too wide the
/// comparisons wrap under the runs (see `sheet_body`).
fn sheet_layout(ui: &Ui, th: &Theme, params: &[ParamDef], third: bool, avail: f32) -> Sheet {
    let name_w = params
        .iter()
        .map(|p| text_width(ui, th, p.name, Role::SansB, 15.0) + 6.0 + text_width(ui, th, p.unit, Role::Sans, 11.5))
        .fold(96.0, f32::max)
        .ceil();
    let limit_w = text_width(ui, th, "≤ 25%", Role::Mono, 13.0).ceil();
    let n = if third { 3.0 } else { 2.0 };
    let m = run_pairs(third).len() as f32;
    let pct_w = text_width(ui, th, "100.00%", Role::MonoB, 14.0).ceil();
    let limit_x = name_w + COL_GAP;
    let runs_x = limit_x + limit_w + COL_GAP;
    let build = |words: bool, run_w: f32| {
        let cmp_w = if words {
            pct_w + 10.0 + badge_width(ui, th, "Acceptable").max(badge_width(ui, th, "Check value")).ceil()
        } else {
            // the symbol, or a heading like "1 vs 2" if that is wider
            (pct_w + 20.0).max(text_width(ui, th, "1 vs 2", Role::SansB, 12.0).ceil())
        };
        let cmp_x = runs_x + n * run_w + (n - 1.0) * RUN_GAP + COL_GAP;
        let inner = cmp_x + m * cmp_w + (m - 1.0) * COL_GAP;
        Sheet { limit_x, runs_x, cmp_x, pct_w, cmp_w, run_w, words, inner }
    };
    let options = [(true, RUN_W), (false, RUN_W), (false, RUN_W_COMPACT)];
    let mut sh = build(true, RUN_W);
    for (words, run_w) in options {
        sh = build(words, run_w);
        if sh.inner <= avail + 0.5 {
            break;
        }
    }
    sh
}

/// One comparison: the difference, then a glyph + word verdict. Pending
/// comparisons show only a dash, so empty rows stay quiet.
/// `words: false` keeps only the glyph (narrow windows). Returns the right edge.
fn comparison(ui: &Ui, th: &Theme, x: f32, mid: f32, pct_w: f32, res: &PairResult, words: bool) -> f32 {
    if res.status == Status::Pending {
        text(ui, th, pos2(x + pct_w, mid), Align2::RIGHT_CENTER, "—", Role::MonoB, 14.0, th.line_strong);
        return x + pct_w;
    }
    let pc = if res.status == Status::Bad { status_colors(th, res.status) } else { th.text };
    text(ui, th, pos2(x + pct_w, mid), Align2::RIGHT_CENTER, &res.percent, Role::MonoB, 14.0, pc);
    if words {
        let bw = badge_width(ui, th, res.badge);
        badge(ui, th, pos2(x + pct_w + 10.0 + bw, mid), res.badge, res.status);
        x + pct_w + 10.0 + bw
    } else {
        status_glyph(ui.painter(), pos2(x + pct_w + 14.0, mid), 6.0, res.status, status_colors(th, res.status));
        x + pct_w + 20.0
    }
}

#[allow(clippy::too_many_arguments)]
fn sheet_body(ui: &mut Ui, th: &Theme, iw: f32, sh: &Sheet, key: &str, params: &[ParamDef], vals: &mut [[String; 3]], third: bool, evals: &[ParamEval]) {
    let o = ui.cursor().min;
    let n = if third { 3 } else { 2 };
    let pairs = run_pairs(third);
    // Too narrow for one line: the comparisons wrap under the runs, each
    // carrying its own "1 vs 2" caption since the column headings are gone.
    let stacked = sh.inner > iw + 0.5;
    let inset = cell_inset(th);
    let rule = Stroke::new(1.0_f32, sheet_rule(th));

    // column headings
    let hm = o.y + HEAD_ROW / 2.0 - 4.0;
    col_head(ui, th, pos2(o.x + sh.limit_x, hm), Align2::LEFT_CENTER, "Limit", th.muted);
    for k in 0..n {
        let x = o.x + sh.runs_x + k as f32 * (sh.run_w + RUN_GAP);
        col_head(ui, th, pos2(x + inset, hm), Align2::LEFT_CENTER, &format!("Run {}", k + 1), th.muted);
    }
    if !stacked {
        for (j, (a, b)) in pairs.iter().enumerate() {
            let x = o.x + sh.cmp_x + j as f32 * (sh.cmp_w + COL_GAP);
            let head = if sh.words { format!("Run {} vs {}", a + 1, b + 1) } else { format!("{} vs {}", a + 1, b + 1) };
            col_head(ui, th, pos2(x + sh.pct_w, hm), Align2::RIGHT_CENTER, &head, th.muted);
        }
    }
    let mut y = o.y + HEAD_ROW;
    ui.painter().line_segment([pos2(o.x, y - 0.5), pos2(o.x + iw, y - 0.5)], rule);

    for (i, p) in params.iter().enumerate() {
        let row_h = if stacked { ROW_H + STACK_H } else { ROW_H };
        let row = Rect::from_min_max(pos2(o.x - 12.0, y), pos2(o.x + iw + 12.0, y + row_h));
        let flagged = evals[i].status == Status::Bad;
        if flagged {
            // The one loud thing on the page: a flagged row is washed in the
            // flag colour with a solid edge, so it is found without reading.
            let pnt = ui.painter();
            let r = Rect::from_min_max(row.min + vec2(0.0, 1.0), row.max - vec2(0.0, 1.0));
            if th.palette == Palette::Cyberpunk {
                cyber_fill(pnt, r, 4.0, th.bad_bg);
            } else {
                fill(pnt, r, 6.0, th.bad_bg);
            }
            pnt.rect_filled(Rect::from_min_size(r.min, vec2(3.0, r.height())), 0.0, th.bad);
        }
        let mid = y + ROW_H / 2.0;

        // name + unit
        let name_c = match p.color {
            Some(k) => th.param[k].0,
            None => th.text,
        };
        let nr = text(ui, th, pos2(o.x, mid), Align2::LEFT_CENTER, p.name, Role::SansB, 15.0, name_c);
        text(ui, th, pos2(nr.right() + 6.0, mid + 1.0), Align2::LEFT_CENTER, p.unit, Role::Sans, 11.5, th.muted);
        // limit
        text(ui, th, pos2(o.x + sh.limit_x, mid), Align2::LEFT_CENTER, &format!("≤ {}%", fmt_js(p.limit)), Role::Mono, 13.0, th.muted);

        // runs
        for k in 0..n {
            let x = o.x + sh.runs_x + k as f32 * (sh.run_w + RUN_GAP);
            let state = if parse_positive(&vals[i][k]) == Num::Invalid {
                FieldState::Invalid
            } else if evals[i].flagged_runs[k] {
                FieldState::Flagged
            } else {
                FieldState::Normal
            };
            let r = Rect::from_min_size(pos2(x, mid - CELL_H / 2.0), vec2(sh.run_w, CELL_H));
            field_s(ui, th, r, Id::new((key, p.id, k)), &mut vals[i][k], "", state);
        }

        // comparisons
        let mut sx = o.x + sh.limit_x;
        for (j, (a, b)) in pairs.iter().enumerate() {
            let res = pair_result(parse_positive(&vals[i][*a]), parse_positive(&vals[i][*b]), p.limit);
            if stacked {
                let cm = y + ROW_H + STACK_H / 2.0 - 6.0;
                let cap = col_head(ui, th, pos2(sx, cm), Align2::LEFT_CENTER, &format!("{} vs {}", a + 1, b + 1), th.muted);
                // Left-aligned after its caption, so the gap never varies.
                let shown = if res.status == Status::Pending { "—" } else { res.percent.as_str() };
                let tw = text_width(ui, th, shown, Role::MonoB, 14.0);
                sx = comparison(ui, th, cap.right() + 6.0 + tw - sh.pct_w, cm, sh.pct_w, &res, false) + 22.0;
            } else {
                let x = o.x + sh.cmp_x + j as f32 * (sh.cmp_w + COL_GAP);
                comparison(ui, th, x, mid, sh.pct_w, &res, sh.words);
            }
        }

        y += row_h;
        if i + 1 < params.len() && !flagged && evals[i + 1].status != Status::Bad {
            ui.painter().line_segment([pos2(o.x, y - 0.5), pos2(o.x + iw, y - 0.5)], rule);
        }
    }
    ui.allocate_rect(Rect::from_min_max(o, pos2(o.x + iw, y)), Sense::hover());
}

/// Small buttons placed in a card's header, right-aligned.
fn header_buttons(ui: &mut Ui, card: Rect, f: impl FnOnce(&mut Ui)) {
    let r = Rect::from_min_max(pos2(card.left() + 20.0, card.top() + (HEAD_H - 32.0) / 2.0), pos2(card.right() - 20.0, card.top() + (HEAD_H + 32.0) / 2.0));
    let mut child = ui.new_child(UiBuilder::new().max_rect(r).layout(Layout::right_to_left(Align::Center)));
    child.spacing_mut().item_spacing.x = 8.0;
    f(&mut child);
}

impl App {
    /// FBC (with the dilution sheet underneath) or PT / APTT.
    pub fn checker_panel(&mut self, ui: &mut Ui, th: &Theme, origin: Pos2, w: f32, is_fbc: bool) -> f32 {
        let ck = if is_fbc { &mut self.fbc } else { &mut self.pt };
        let key = ck.key;
        let params = ck.params;
        let third = ck.third;

        // Both sheets share one width: the wider of the two at their natural
        // size, centred, so the page reads as one compact column.
        let sh = sheet_layout(ui, th, params, third, w - CARD_CHROME);
        let mut cw = sh.inner + CARD_CHROME;
        if is_fbc {
            cw = cw.max(dilution_natural(ui, th) + CARD_CHROME);
        }
        let cw = cw.min(w).floor();
        let x0 = origin.x + ((w - cw) / 2.0).floor();

        let evals: Vec<ParamEval> = params.iter().zip(&ck.vals).map(|(p, v)| eval_param(p, v, third)).collect();
        let (st, msg) = overall_message(ck, &evals);
        let opts = CardOpts {
            title: if is_fbc { "FBC" } else { "PT / APTT" },
            unit: "",
            bar: None,
            head_bg: None,
            title_color: None,
            chip: None,
            min_height: 0.0,
            alert: st == Status::Bad,
            status: Some((st, &msg)),
            collapsed: false,
        };
        let vals = &mut ck.vals;
        let rect = card(ui, th, pos2(x0, origin.y), cw, &opts, |ui, iw| sheet_body(ui, th, iw, &sh, key, params, vals, third, &evals));

        let (mut clear, mut toggle) = (false, false);
        header_buttons(ui, rect, |ui| {
            if button(ui, th, "Clear", Btn::MiniMuted, true).clicked() {
                clear = true;
            }
            let label = if third { "− Run 3" } else { "+ Run 3" };
            if button(ui, th, label, Btn::Mini, true).clicked() {
                toggle = true;
            }
        });
        if toggle {
            ck.third = !ck.third;
            if !ck.third {
                for v in ck.vals.iter_mut() {
                    v[2].clear();
                }
            }
        }
        if clear {
            for v in ck.vals.iter_mut() {
                for s in v.iter_mut() {
                    s.clear();
                }
            }
            ui.memory_mut(|m| m.request_focus(Id::new((key, params[0].id, 0usize))));
        }

        if is_fbc {
            self.dilution_card(ui, th, pos2(x0, rect.bottom() + 16.0), cw)
        } else {
            rect.bottom()
        }
    }
}

// ---------------------------------------------------------------------------
// Reticulocyte
// ---------------------------------------------------------------------------

/// Retic inputs: wide enough for five characters ("38.25"), no wider.
const R_IN_W: f32 = 76.0;
const GENDER_W: f32 = 140.0;
const R_IN_GAP: f32 = 12.0;
/// Caption band above a compact field.
const R_CAP_H: f32 = 18.0;
/// Counting-helper boxes: counts are at most four digits.
const ATTEMPT_W: f32 = 68.0;
const ATTEMPT_COLS: usize = 4;
const R_CARD_GAP: f32 = 16.0;

fn mini_btn_w(ui: &Ui, th: &Theme, s: &str) -> f32 {
    text_width(ui, th, s, Role::SansB, 12.8) * if th.palette == Palette::Cyberpunk { 1.15 } else { 1.0 } + 26.0
}

/// Card title plus header text, with room for Cyberpunk's tracked capitals.
fn head_w(ui: &Ui, th: &Theme, title: &str, unit: &str) -> f32 {
    let t = text_width(ui, th, title, Role::Display, 16.0) * if th.palette == Palette::Cyberpunk { 1.25 } else { 1.0 };
    let u = if unit.is_empty() { 0.0 } else { 8.0 + text_width(ui, th, unit, Role::Sans, 11.84) };
    t + u
}

/// Natural inner width of the counting helper.
fn helper_inner(ui: &Ui, th: &Theme) -> f32 {
    let n = ATTEMPT_COLS as f32;
    let grid = n * ATTEMPT_W + (n - 1.0) * R_IN_GAP;
    let avg = text_width(ui, th, "Average of 12", Role::Sans, 13.0) + 16.0 + text_width(ui, th, "100.00 %", Role::MonoB, 16.0) + 10.0 + mini_btn_w(ui, th, "Use as retic %");
    grid.max(head_w(ui, th, "Counting helper", "retics / 1000 RBCs")).max(avg).ceil()
}

/// Natural inner width of the Reticulocyte card.
fn main_inner(ui: &Ui, th: &Theme) -> f32 {
    // the last caption ("RBC ×10¹²/L") is wider than its box
    let cap = text_width(ui, th, "RBC ×10¹²/L", Role::SansB, 12.0) * if th.palette == Palette::Cyberpunk { 1.2 } else { 1.0 };
    let inputs = 2.0 * R_IN_W + GENDER_W + 3.0 * R_IN_GAP + R_IN_W.max(cell_inset(th) + cap + 4.0);
    let head = head_w(ui, th, "Reticulocyte", "") + 24.0 + mini_btn_w(ui, th, "Hide calculations") + 8.0 + mini_btn_w(ui, th, "Clear");
    inputs.max(head).ceil()
}

impl App {
    /// Inner width a run sheet has in the default window.
    fn sheet_room(&self) -> f32 {
        self.default_page_width().map(|w| w - CARD_CHROME).unwrap_or(f32::INFINITY)
    }

    /// Natural width of the current page, before the window margin.
    pub fn page_width(&self, ui: &Ui, th: &Theme) -> f32 {
        match self.tab {
            // Run 3 makes the sheet wider: it takes its compact form if that
            // keeps it within the default window, so the window only grows down.
            Tab::Fbc => (sheet_layout(ui, th, &FBC_PARAMS, self.fbc.third, self.sheet_room()).inner.max(dilution_natural(ui, th)) + CARD_CHROME).ceil(),
            Tab::PtAptt => (sheet_layout(ui, th, &PT_PARAMS, self.pt.third, self.sheet_room()).inner + CARD_CHROME).ceil(),
            Tab::Retic => self.retic_width(ui, th).ceil(),
        }
    }
}

pub struct ReticOut {
    pub corrected_text: String,
    pub corrected_rounded: bool,
    pub corrected_note: Option<String>,
    pub absolute_text: String,
    pub absolute_note: Option<String>,
    pub rbc_note: Option<String>,
    /// What is still missing (or why there is no value), shown under an empty result.
    pub corrected_hint: Option<String>,
    pub absolute_hint: Option<String>,
}

pub enum EqPart {
    Val(String),
    Op(String),
    Res(String),
}

#[derive(PartialEq, Clone, Copy)]
pub enum StepState {
    Pending,
    Done,
    Nil,
}

pub struct Step {
    pub num: usize,
    pub title: &'static str,
    pub state: StepState,
    pub formula: String,
    pub eq: Vec<EqPart>,
    pub tags: Vec<(String, bool)>,
    pub hint: Option<String>,
    pub result: Option<String>,
}

impl App {
    fn retic_nums(&self) -> (Num, Num, Num, Corrected) {
        let retic = parse_number(&self.r_retic);
        let hct = parse_number(&self.r_hct);
        let rbc = parse_number(&self.r_rbc);
        (retic, hct, rbc, computed_corrected(retic, hct, self.gender))
    }

    pub fn compute_retic(&self) -> ReticOut {
        let (retic, hct, rbc, corrected) = self.retic_nums();
        let reticv = retic.ok();
        let rbcv = rbc.ok();
        let needs = |items: &[(&'static str, bool)]| {
            let missing: Vec<&str> = items.iter().filter(|(_, have)| !have).map(|(n, _)| *n).collect();
            if missing.is_empty() { None } else { Some(format!("Needs {}", join_and(&missing))) }
        };
        let mut out = ReticOut {
            corrected_text: "—".into(),
            corrected_rounded: false,
            corrected_note: None,
            absolute_text: "—".into(),
            absolute_note: None,
            rbc_note: None,
            corrected_hint: needs(&[("retic %", reticv.is_some()), ("HCT", hct.ok().is_some()), ("gender", self.gender.is_some())]),
            absolute_hint: needs(&[("retic %", reticv.is_some()), ("RBC", rbcv.is_some())]),
        };
        if corrected == Corrected::Nil {
            out.corrected_hint = Some("HCT is not below the threshold, so no correction applies".into());
        }
        let rbc_r = rbcv.map(round1);
        if let (Some(r), Some(rr)) = (rbcv, rbc_r) {
            if is_rounded(r, rr) {
                out.rbc_note = Some(format!("1 dp: {} → {}", fmt_raw(r), to_fixed(rr, 1)));
            }
        }
        let mut corrected_r = None;
        let mut corrected_was = false;
        match corrected {
            Corrected::NotComputable => {}
            Corrected::Nil => out.corrected_text = "NIL".into(),
            Corrected::Value(c) => {
                let cr = round2(c);
                corrected_r = Some(cr);
                corrected_was = is_rounded(c, cr);
                out.corrected_text = to_fixed(cr, 2);
                out.corrected_rounded = corrected_was;
                if corrected_was {
                    out.corrected_note = Some(format!("Rounded to 2 dp: {} → {}.", fmt_raw(c), to_fixed(cr, 2)));
                }
            }
        }
        if let (Some(retic), Some(rbc_r)) = (reticv, rbc_r) {
            match corrected {
                Corrected::Value(_) => {
                    let cr = corrected_r.unwrap();
                    out.absolute_text = to_fixed(cr * rbc_r * 10.0, 1);
                    if corrected_was {
                        out.absolute_note = Some(format!("Using corrected retic % at 2 dp ({})", to_fixed(cr, 2)));
                    }
                }
                _ => {
                    let rr = round2(retic);
                    out.absolute_text = to_fixed(rr * rbc_r * 10.0, 1);
                    if is_rounded(retic, rr) {
                        out.absolute_note = Some(format!("Retic % at 2 dp: {} → {}.", fmt_raw(retic), to_fixed(rr, 2)));
                    }
                }
            }
        }
        out
    }

    pub fn build_steps(&self) -> Vec<Step> {
        let (retic, hct, rbc, corrected) = self.retic_nums();
        let mut steps = vec![];
        let op = |s: &str| EqPart::Op(s.to_string());
        let val = |s: String| EqPart::Val(s);

        // 1. counting helper
        let vals: Vec<f64> = self.attempts.iter().filter_map(|s| parse_number(s).ok()).collect();
        let f1 = "Average of attempts ÷ 10".to_string();
        if vals.is_empty() {
            steps.push(Step { num: 1, title: "Counting helper: average % retic", state: StepState::Pending, formula: f1, eq: vec![], tags: vec![], hint: Some("Enter counts in the counting helper to see the average.".into()), result: None });
        } else {
            let avg = vals.iter().sum::<f64>() / vals.len() as f64 / 10.0;
            let avg_r = round2(avg);
            let mut eq = vec![op("(")];
            for (i, v) in vals.iter().enumerate() {
                if i > 0 {
                    eq.push(op("+"));
                }
                eq.push(val(fmt_js(*v)));
            }
            eq.extend([op(")"), op("÷"), val(vals.len().to_string()), op("attempts ÷"), val("10".into()), op("="), EqPart::Res(fmt_raw(avg))]);
            let tags = if is_rounded(avg, avg_r) {
                vec![(format!("Rounded to 2 dp when sent with “Use as retic %”: {} → {}", fmt_raw(avg), to_fixed(avg_r, 2)), false)]
            } else {
                vec![]
            };
            steps.push(Step { num: 1, title: "Counting helper: average % retic", state: StepState::Done, formula: f1, eq, tags, hint: None, result: Some(format!("{} %", to_fixed(avg_r, 2))) });
        }

        // 2. correction check
        let f2 = "Correct only if round(HCT) < 41 (male) or 36 (female)".to_string();
        match (hct, self.gender) {
            (Num::Val(h), Some(g)) => {
                let thr = if g == Gender::Male { 41.0 } else { 36.0 };
                let hr = js_round(h);
                let applies = hr < thr;
                let gl = if g == Gender::Male { "male" } else { "female" };
                let eq = vec![op("round("), val(fmt_raw(h)), op(")"), op("="), val(fmt_js(hr)), op(if applies { "<" } else { "≥" }), val(fmt_js(thr)), op(&format!("({} threshold)", gl))];
                steps.push(Step {
                    num: 2,
                    title: "Correction check",
                    state: if applies { StepState::Done } else { StepState::Nil },
                    formula: f2,
                    eq,
                    tags: vec![],
                    hint: None,
                    result: Some(if applies { "Correction applies".into() } else { "No correction (NIL)".into() }),
                });
            }
            _ => steps.push(Step { num: 2, title: "Correction check", state: StepState::Pending, formula: f2, eq: vec![], tags: vec![], hint: Some("Enter HCT and select gender to check whether correction applies.".into()), result: None }),
        }

        // 3. corrected retic
        let f3 = "Observed retic % × (HCT ÷ 45)".to_string();
        match corrected {
            Corrected::NotComputable => steps.push(Step { num: 3, title: "Corrected retic %", state: StepState::Pending, formula: f3, eq: vec![], tags: vec![], hint: Some("Enter retic %, HCT and gender to calculate the corrected retic %.".into()), result: None }),
            Corrected::Nil => steps.push(Step { num: 3, title: "Corrected retic %", state: StepState::Nil, formula: f3, eq: vec![], tags: vec![], hint: Some("HCT is not below the threshold, so no correction is applied. The observed retic % is used in the next step.".into()), result: Some("NIL".into()) }),
            Corrected::Value(c) => {
                let cr = round2(c);
                let (r, h) = (retic.ok().unwrap(), hct.ok().unwrap());
                let eq = vec![val(fmt_raw(r)), op("× ("), val(fmt_raw(h)), op("÷"), val("45".into()), op(")"), op("="), EqPart::Res(fmt_raw(c))];
                let tags = if is_rounded(c, cr) { vec![(format!("Rounded to 2 dp: {} → {}. The 2 dp value is used in step 4.", fmt_raw(c), to_fixed(cr, 2)), false)] } else { vec![] };
                steps.push(Step { num: 3, title: "Corrected retic %", state: StepState::Done, formula: f3, eq, tags, hint: None, result: Some(format!("{} %", to_fixed(cr, 2))) });
            }
        }

        // 4. absolute retic
        let f4 = "% retic × RBC (×10¹²/L) × 10".to_string();
        match (retic.ok(), rbc.ok()) {
            (Some(r), Some(b)) => {
                let mut tags: Vec<(String, bool)> = vec![];
                let (inp, source);
                if let Corrected::Value(c) = corrected {
                    inp = round2(c);
                    source = "corrected retic % (2 dp)";
                } else {
                    inp = round2(r);
                    source = "observed retic % (2 dp)";
                    if is_rounded(r, inp) {
                        tags.push((format!("Observed retic % rounded to 2 dp: {} → {}", fmt_raw(r), to_fixed(inp, 2)), false));
                    }
                    if corrected == Corrected::NotComputable {
                        tags.push(("HCT / gender not entered, so no correction is applied".into(), true));
                    }
                }
                let rbc_r = round1(b);
                if is_rounded(b, rbc_r) {
                    tags.push((format!("RBC rounded to 1 dp: {} → {}", fmt_raw(b), to_fixed(rbc_r, 1)), false));
                }
                let raw = inp * rbc_r * 10.0;
                tags.push((format!("Using {}", source), true));
                tags.push(("Result rounded to 1 dp".into(), true));
                let eq = vec![val(to_fixed(inp, 2)), op("×"), val(to_fixed(rbc_r, 1)), op("×"), val("10".into()), op("="), EqPart::Res(fmt_raw(raw))];
                steps.push(Step { num: 4, title: "Absolute retic", state: StepState::Done, formula: f4, eq, tags, hint: None, result: Some(format!("{} ×10⁹/L", to_fixed(raw, 1))) });
            }
            _ => steps.push(Step { num: 4, title: "Absolute retic", state: StepState::Pending, formula: f4, eq: vec![], tags: vec![], hint: Some("Enter retic % and RBC count to calculate the absolute retic.".into()), result: None }),
        }
        steps
    }

    fn reset_retic(&mut self) {
        self.r_retic.clear();
        self.r_hct.clear();
        self.r_rbc.clear();
        self.gender = None;
        self.attempts = vec![String::new(); MIN_ATTEMPTS];
        self.retic_note = None;
    }

    /// Natural width of the Retic page: the two cards side by side at their
    /// compact sizes, or the reference table, whichever is wider.
    pub fn retic_width(&self, ui: &Ui, th: &Theme) -> f32 {
        let row = helper_inner(ui, th) + CARD_CHROME + R_CARD_GAP + main_inner(ui, th) + CARD_CHROME;
        // A folded reference table is only a header, so it does not widen the page.
        if self.ref_open { row.max(crate::reference::natural_width(ui, th) + CARD_CHROME) } else { row }
    }

    pub fn retic_panel(&mut self, ui: &mut Ui, th: &Theme, origin: Pos2, w: f32) -> f32 {
        let y = origin.y;
        let out = self.compute_retic();
        let gap = R_CARD_GAP;
        let helper_w = (helper_inner(ui, th) + CARD_CHROME).ceil();
        let two_col = w >= helper_w + gap + main_inner(ui, th) + CARD_CHROME;
        let h_id = Id::new("retic-grid-h");
        let min_h: f32 = if two_col { ui.ctx().data(|d| d.get_temp(h_id)).unwrap_or(0.0) } else { 0.0 };

        // The counting helper sits first (left): its average feeds retic %.
        // It keeps its compact width; the results card takes the rest.
        let (main_pos, main_w) = if two_col { (pos2(origin.x + helper_w + gap, y), w - helper_w - gap) } else { (pos2(origin.x, y), w) };

        let main_opts = CardOpts { title: "Reticulocyte", unit: "", bar: None, head_bg: None, title_color: None, chip: None, min_height: min_h, alert: false, status: None, collapsed: false };
        let main_rect = card(ui, th, main_pos, main_w, &main_opts, |ui, iw| self.retic_main_body(ui, th, iw, &out));
        let (mut clear, mut toggle) = (false, false);
        let calc_open = self.calc_open;
        header_buttons(ui, main_rect, |ui| {
            if button(ui, th, "Clear", Btn::MiniMuted, true).clicked() {
                clear = true;
            }
            let label = if calc_open { "Hide calculations" } else { "Show calculations" };
            if button(ui, th, label, Btn::Mini, true).clicked() {
                toggle = true;
            }
        });

        let helper_pos = if two_col { origin } else { pos2(origin.x, main_rect.bottom() + gap) };
        let helper_opts = CardOpts { title: "Counting helper", unit: "retics / 1000 RBCs", bar: None, head_bg: None, title_color: None, chip: None, min_height: min_h, alert: false, status: None, collapsed: false };
        let helper_rect = card(ui, th, helper_pos, if two_col { helper_w } else { w }, &helper_opts, |ui, iw| self.helper_body(ui, th, iw));
        let row_h = main_rect.height().max(helper_rect.height());
        if two_col {
            let prev: f32 = ui.ctx().data(|d| d.get_temp(h_id)).unwrap_or(0.0);
            if (prev - row_h).abs() > 0.5 {
                ui.ctx().data_mut(|d| d.insert_temp(h_id, row_h));
                ui.ctx().request_repaint();
            }
        }
        if clear {
            self.reset_retic();
            ui.memory_mut(|m| m.request_focus(Id::new(("r-attempt", 0usize))));
        }
        if toggle {
            self.calc_open = !self.calc_open;
        }
        let mut y = main_rect.bottom().max(helper_rect.bottom()) + 16.0;

        if self.calc_open {
            y = self.calc_card(ui, th, pos2(origin.x, y), w) + 16.0;
        }
        crate::reference::reference_card(ui, th, pos2(origin.x, y), w, &mut self.ref_open)
    }

    fn retic_main_body(&mut self, ui: &mut Ui, th: &Theme, iw: f32, out: &ReticOut) {
        let o = ui.cursor().min;
        let inset = cell_inset(th);
        // Four compact inputs in one line, captions above, left-aligned: the
        // values are a few digits, so the boxes are sized for that.
        let widths = [R_IN_W, R_IN_W, GENDER_W, R_IN_W];
        let mut xs = [o.x; 4];
        for k in 1..4 {
            xs[k] = xs[k - 1] + widths[k - 1] + R_IN_GAP;
        }
        let cap_y = o.y + 6.0;
        let fy = o.y + R_CAP_H;
        let caps = ["Retic %", "HCT %", "Gender", "RBC ×10¹²/L"];
        for (k, c) in caps.iter().enumerate() {
            col_head(ui, th, pos2(xs[k] + inset, cap_y), Align2::LEFT_CENTER, c, th.muted);
        }
        let cell = |k: usize| Rect::from_min_size(pos2(xs[k], fy), vec2(widths[k], CELL_H));
        let bad = |s: &str| parse_number(s) == Num::Invalid;
        let inv = bad(&self.r_retic);
        if field(ui, th, cell(0), Id::new("r-retic"), &mut self.r_retic, "", inv) {
            self.retic_note = None;
        }
        let inv = bad(&self.r_hct);
        field(ui, th, cell(1), Id::new("r-hct"), &mut self.r_hct, "", inv);
        self.gender_toggle(ui, th, cell(2));
        let inv = bad(&self.r_rbc);
        field(ui, th, cell(3), Id::new("r-rbc"), &mut self.r_rbc, "", inv);

        // Rounding notes hang under the field they describe, one per line.
        let mut y = fy + CELL_H;
        // The retic note runs right from its box; the RBC note (last column)
        // ends flush with its box and grows leftwards, so neither wraps.
        if let Some(n) = self.retic_note.as_deref() {
            y = note(ui, th, pos2(xs[0], y + 6.0), Align2::LEFT_TOP, n, iw).bottom();
        }
        if let Some(n) = out.rbc_note.as_deref() {
            y = note(ui, th, pos2(xs[3] + R_IN_W, y + 6.0), Align2::RIGHT_TOP, n, xs[3] + R_IN_W - o.x).bottom();
        }

        let mut y = y + 16.0;
        let tiles = [
            Tile { label: "Corrected retic", value: &out.corrected_text, unit: "%", rounded: out.corrected_rounded, note: out.corrected_note.as_deref(), hint: out.corrected_hint.as_deref() },
            Tile { label: "Absolute retic", value: &out.absolute_text, unit: "×10⁹/L", rounded: false, note: out.absolute_note.as_deref(), hint: out.absolute_hint.as_deref() },
        ];
        y += result_tiles(ui, th, pos2(o.x, y), iw, &tiles);

        // disclaimer
        y += 16.0;
        let msg = "Verify these calculated results against your laboratory's SOP before use. You are solely responsible for deriving and confirming your own results.";
        let tw = iw - 28.0 - 3.0;
        let th_h = paragraph_height(ui, th, tw, msg, Role::Serif, 12.8, 1.5);
        let r = Rect::from_min_size(pos2(o.x, y), vec2(iw, th_h + 24.0));
        let rad = if th.palette == Palette::Pokemon { 14 } else { th.radius.round() as u8 };
        ui.painter().rect_filled(r, CornerRadius { nw: 0, sw: 0, ne: rad, se: rad }, th.warn_bg);
        ui.painter().rect_filled(Rect::from_min_size(r.min, vec2(3.0, r.height())), 0.0, th.warn);
        paragraph(ui, th, pos2(r.left() + 3.0 + 14.0, r.top() + 12.0), tw, msg, Role::Serif, 12.8, th.warn, 1.5);
        y = r.bottom();
        ui.allocate_rect(Rect::from_min_max(o, pos2(o.x + iw, y)), Sense::hover());
    }

    fn gender_toggle(&mut self, ui: &mut Ui, th: &Theme, rect: Rect) {
        let p = ui.painter().clone();
        let r = match th.palette {
            Palette::Genshin => pill(),
            Palette::Pokemon => 14.0,
            Palette::Cyberpunk => 0.0,
            _ => th.radius,
        };
        let border = match th.palette {
            Palette::Pokemon => th.ring,
            Palette::Genshin => rgba(th.gold, 0.45),
            Palette::Cyberpunk => rgba(th.accent, 0.62),
            _ => th.line,
        };
        if th.palette == Palette::Cyberpunk {
            cyber_panel(&p, rect, 4.0, th.surface2, 1.0, border);
        } else {
            fill(&p, rect, r, th.surface2);
        }
        let halves = [
            (Gender::Male, "Male", Rect::from_min_max(rect.min, pos2(rect.center().x, rect.bottom()))),
            (Gender::Female, "Female", Rect::from_min_max(pos2(rect.center().x, rect.top()), rect.max)),
        ];
        for (i, (g, name, half)) in halves.iter().enumerate() {
            let resp = ui.interact(*half, Id::new(("gender", i)), Sense::click());
            let active = self.gender == Some(*g);
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                self.gender = Some(*g);
            }
            if active {
                p.with_clip_rect(*half).rect_filled(rect, cr(r), th.accent);
            } else if resp.hovered() && th.palette == Palette::Genshin {
                p.with_clip_rect(*half).rect_filled(rect, cr(r), crate::theme::mix(0.16, th.gold, th.surface2));
            }
            let fg = if active { th.on_accent } else { th.text };
            text(ui, th, half.center(), Align2::CENTER_CENTER, name, Role::SansB, 14.08, fg);
        }
        p.line_segment([pos2(rect.center().x, rect.top() + 1.0), pos2(rect.center().x, rect.bottom() - 1.0)], Stroke::new(1.0_f32, border));
        if th.palette == Palette::Cyberpunk {
            cyber_stroke(&p, rect, 4.0, 1.0, border);
        } else {
            stroke_in(&p, rect, r, 1.0, border);
        }
    }

    /// Keep exactly one spare empty box after the last count (at least
    /// `MIN_ATTEMPTS` boxes, at most `MAX_ATTEMPTS`), so there is nothing to
    /// add or remove by hand: typing in the last box opens the next one.
    fn tidy_attempts(&mut self, ui: &Ui) {
        let empty = |s: &String| s.trim().is_empty();
        if self.attempts.len() < MAX_ATTEMPTS && self.attempts.last().is_some_and(|s| !empty(s)) {
            self.attempts.push(String::new());
        }
        while self.attempts.len() > MIN_ATTEMPTS {
            let n = self.attempts.len();
            let last_focused = ui.memory(|m| m.has_focus(Id::new(("r-attempt", n - 1))));
            if empty(&self.attempts[n - 1]) && empty(&self.attempts[n - 2]) && !last_focused {
                self.attempts.pop();
            } else {
                break;
            }
        }
    }

    fn helper_body(&mut self, ui: &mut Ui, th: &Theme, iw: f32) {
        self.tidy_attempts(ui);
        let o = ui.cursor().min;
        let n = self.attempts.len();
        let inset = cell_inset(th);
        // A grid of small numbered boxes, four to a row: a count is at most
        // four digits, so each box is sized for that and no more.
        let cols = ATTEMPT_COLS.min(((iw + R_IN_GAP) / (ATTEMPT_W + R_IN_GAP)).floor().max(1.0) as usize);
        let row_h = R_CAP_H + CELL_H + 10.0;
        for i in 0..n {
            let x = o.x + (i % cols) as f32 * (ATTEMPT_W + R_IN_GAP);
            let ry = o.y + (i / cols) as f32 * row_h;
            text(ui, th, pos2(x + inset, ry + 6.0), Align2::LEFT_CENTER, &(i + 1).to_string(), Role::Mono, 11.5, th.muted);
            let invalid = parse_number(&self.attempts[i]) == Num::Invalid;
            let r = Rect::from_min_size(pos2(x, ry + R_CAP_H), vec2(ATTEMPT_W, CELL_H));
            field(ui, th, r, Id::new(("r-attempt", i)), &mut self.attempts[i], "", invalid);
        }
        let rows = n.div_ceil(cols);
        let mut y = o.y + rows as f32 * row_h - 10.0 + 16.0;

        // average, and the one action: send it to retic %
        ui.painter().line_segment([pos2(o.x, y + 0.5), pos2(o.x + iw, y + 0.5)], Stroke::new(1.0_f32, sheet_rule(th)));
        let counted = self.attempts.iter().filter(|s| parse_number(s).ok().is_some()).count();
        let avg = average_attempts(&self.attempts);
        let avg_text = avg.map(|a| format!("{} %", to_fixed(a, 2))).unwrap_or_else(|| "—".into());
        let row = Rect::from_min_size(pos2(o.x, y + 12.0), vec2(iw, 32.0));
        let cap = if counted > 0 { format!("Average of {}", counted) } else { "Average".to_string() };
        text(ui, th, pos2(o.x, row.center().y), Align2::LEFT_CENTER, &cap, Role::Sans, 13.0, th.muted);
        let mut child = ui.new_child(UiBuilder::new().max_rect(row).layout(Layout::right_to_left(Align::Center)));
        let b = button(&mut child, th, "Use as retic %", Btn::Mini, avg.is_some());
        text(ui, th, pos2(b.rect.left() - 10.0, row.center().y), Align2::RIGHT_CENTER, &avg_text, Role::MonoB, 16.0, if avg.is_some() { th.text } else { th.line_strong });
        if b.clicked() {
            if let Some(a) = avg {
                let ar = round2(a);
                self.r_retic = to_fixed(ar, 2);
                self.retic_note = if is_rounded(a, ar) { Some(format!("2 dp: {} → {}", fmt_raw(a), to_fixed(ar, 2))) } else { None };
            }
        }
        let hint = if n < MAX_ATTEMPTS { "A new box opens when you fill the last one." } else { "Up to 12 attempts." };
        y = paragraph(ui, th, pos2(o.x, row.bottom() + 12.0), iw, hint, Role::Sans, 12.0, th.muted, 1.35).bottom();
        ui.allocate_rect(Rect::from_min_max(o, pos2(o.x + iw, y)), Sense::hover());
    }

    fn calc_card(&mut self, ui: &mut Ui, th: &Theme, pos: Pos2, w: f32) -> f32 {
        let steps = self.build_steps();
        let opts = CardOpts { title: "Calculations", unit: "updates as you type", bar: None, head_bg: None, title_color: None, chip: None, min_height: 0.0, alert: false, status: None, collapsed: false };
        let rect = card(ui, th, pos, w, &opts, |ui, iw| {
            let o = ui.cursor().min;
            // Steps wrap into a 4×1, 2×2 or 1×4 grid by the width available;
            // never 3 + 1, which would orphan the final result.
            let gap = 16.0;
            let fit = (((iw + gap) / (236.0 + gap)).floor() as usize).clamp(1, 4);
            let cols = match fit {
                4 => 4,
                2 | 3 => 2,
                _ => 1,
            };
            let big = cols == 4;
            let cols = cols.min(steps.len().max(1));
            let cw = (iw - gap * (cols as f32 - 1.0)) / cols as f32;
            let mut y = o.y;
            let mut i = 0;
            while i < steps.len() {
                let end = (i + cols).min(steps.len());
                let mut row_h: f32 = 0.0;
                let mut stored: f32 = 0.0;
                for k in i..end {
                    let id = Id::new(("calc-step-h", k));
                    stored = stored.max(ui.ctx().data(|d| d.get_temp::<f32>(id)).unwrap_or(0.0));
                }
                for (c, k) in (i..end).enumerate() {
                    let x = o.x + c as f32 * (cw + gap);
                    let h = draw_step(ui, th, pos2(x, y), cw, &steps[k], stored);
                    row_h = row_h.max(h);
                    if big && k + 1 < steps.len() {
                        text(ui, th, pos2(x + cw + gap / 2.0, y + 18.0), Align2::CENTER_CENTER, "→", Role::SansB, 12.8, th.line_strong);
                    }
                }
                for k in i..end {
                    let id = Id::new(("calc-step-h", k));
                    let prev: f32 = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(0.0);
                    if (prev - row_h).abs() > 0.5 {
                        ui.ctx().data_mut(|d| d.insert_temp(id, row_h));
                        ui.ctx().request_repaint();
                    }
                }
                y += row_h + gap;
                i = end;
            }
            ui.allocate_rect(Rect::from_min_max(o, pos2(o.x + iw, y - gap)), Sense::hover());
        });
        rect.bottom()
    }
}

/// One calculated result: corrected or absolute retic.
struct Tile<'a> {
    label: &'a str,
    value: &'a str,
    unit: &'a str,
    rounded: bool,
    note: Option<&'a str>,
    hint: Option<&'a str>,
}

const TILE_PAD: f32 = 14.0;
const TILE_VALUE: f32 = 26.0;
const TILE_HINT: f32 = 12.5;

fn tile_extra_h(ui: &Ui, th: &Theme, t: &Tile, w: f32) -> f32 {
    let inner = w - 2.0 * TILE_PAD;
    if let Some(n) = t.note {
        8.0 + note_size(ui, th, n, inner).y
    } else if let Some(h) = t.hint {
        6.0 + paragraph_height(ui, th, inner, h, Role::Sans, TILE_HINT, 1.3)
    } else {
        0.0
    }
}

/// The two results as a matching pair of tiles (side by side when there is
/// room). Values are plain ink: they are results, not warnings. Returns height.
fn result_tiles(ui: &mut Ui, th: &Theme, pos: Pos2, iw: f32, tiles: &[Tile]) -> f32 {
    let gap = 12.0;
    let side = iw >= 360.0;
    let tw = if side { (iw - gap) / 2.0 } else { iw };
    let base_h = TILE_PAD + 16.0 + 6.0 + 32.0 + TILE_PAD - 4.0;
    let extras: Vec<f32> = tiles.iter().map(|t| tile_extra_h(ui, th, t, tw)).collect();
    let row_h = base_h + extras.iter().cloned().fold(0.0, f32::max);
    let p = ui.painter().clone();
    let mut bottom = pos.y;
    for (i, t) in tiles.iter().enumerate() {
        let (x, y, h) = if side { (pos.x + i as f32 * (tw + gap), pos.y, row_h) } else { (pos.x, bottom + if i > 0 { gap } else { 0.0 }, base_h + extras[i]) };
        let r = Rect::from_min_size(pos2(x, y), vec2(tw, h));
        bottom = r.bottom();
        match th.palette {
            Palette::Cyberpunk => cyber_panel(&p, r, 6.0, rgba(th.surface2, if th.dark { 0.80 } else { 1.0 }), 1.0, rgba(th.accent, 0.40)),
            Palette::Genshin => {
                let base = if th.dark { hex(0x384053) } else { crate::theme::mix(0.07, hex(0x4a5366), th.surface) };
                fill(&p, r, 5.0, base);
                stroke_in(&p, r, 5.0, 1.0, rgba(th.gold, 0.40));
            }
            Palette::Pokemon => {
                fill(&p, r, 14.0, th.field);
                stroke_in(&p, r, 14.0, 1.5, rgba(th.ring, 0.35));
            }
            Palette::Default => {
                fill(&p, r, th.radius, th.surface2);
                stroke_in(&p, r, th.radius, 1.0, th.line);
            }
        }
        let x0 = r.left() + TILE_PAD;
        let mut cy = r.top() + TILE_PAD - 2.0;
        if th.palette == Palette::Cyberpunk {
            text_ls(ui, th, pos2(x0, cy), Align2::LEFT_TOP, &t.label.to_uppercase(), Role::SansB, 12.0, th.muted, 0.08);
        } else {
            text(ui, th, pos2(x0, cy), Align2::LEFT_TOP, t.label, Role::SansB, 13.0, th.muted);
        }
        cy += 16.0 + 6.0 + 16.0;
        let empty = t.value == "—";
        let vr = text(ui, th, pos2(x0, cy), Align2::LEFT_CENTER, t.value, Role::MonoB, TILE_VALUE, if empty { th.muted } else { th.text });
        if !empty && t.value != "NIL" {
            let ur = text(ui, th, pos2(vr.right() + 6.0, cy + 4.0), Align2::LEFT_CENTER, t.unit, Role::Sans, 13.5, th.muted);
            if t.rounded {
                text(ui, th, pos2(ur.right() + 8.0, cy + 4.0), Align2::LEFT_CENTER, "2 dp", Role::SansB, 10.88, th.warn);
            }
        }
        cy += 16.0 + 8.0 - 4.0;
        if let Some(n) = t.note {
            note(ui, th, pos2(x0, cy + 2.0), Align2::LEFT_TOP, n, tw - 2.0 * TILE_PAD);
        } else if let Some(hn) = t.hint {
            paragraph(ui, th, pos2(x0, cy), tw - 2.0 * TILE_PAD, hn, Role::Sans, TILE_HINT, th.muted, 1.3);
        }
    }
    bottom - pos.y
}

/// Computed values in the worked calculation: tinted with the info hue so they
/// stand apart from inputs without borrowing the flag/accent colour.
fn result_chip_colors(th: &Theme) -> (Color32, Color32, Color32) {
    (th.text, rgba(th.ring, if th.dark { 0.18 } else { 0.12 }), rgba(th.ring, 0.55))
}

/// Draws one calculation step; returns its height.
fn draw_step(ui: &mut Ui, th: &Theme, pos: Pos2, w: f32, s: &Step, min_h: f32) -> f32 {
    let p = ui.painter().clone();
    let bg_i = p.add(egui::Shape::Noop);
    let iw = w - 2.0 - 20.0;
    let x = pos.x + 11.0;
    let mut y = pos.y + 1.0 + 8.0;
    let pending = s.state == StepState::Pending;

    // header row: number badge + title (+ result chip)
    let num_c = pos2(x + 11.0, y + 11.0);
    let (nb, nf) = match s.state {
        StepState::Pending => (th.line_strong, th.bg),
        StepState::Nil => (th.neutral, th.on_accent),
        StepState::Done => (th.accent, th.on_accent),
    };
    if th.palette == Palette::Pokemon {
        p.rect_filled(Rect::from_center_size(num_c, vec2(22.0, 22.0)), 7.0, nb);
    } else if th.palette == Palette::Cyberpunk {
        cyber_panel(&p, Rect::from_center_size(num_c, vec2(22.0, 22.0)), 5.0, nb, 1.0, rgba(th.accent, 0.55));
    } else {
        p.circle_filled(num_c, 11.0, nb);
    }
    text(ui, th, num_c, Align2::CENTER_CENTER, &s.num.to_string(), Role::SansB, 11.52, nf);
    let title_c = if pending { th.muted } else { th.text };
    let title_w = iw - 28.0;
    let step_title = if th.palette == Palette::Cyberpunk { s.title.to_uppercase() } else { s.title.to_string() };
    let tg = galley(ui, th, &step_title, Role::Display, 13.76, title_c, if th.palette == Palette::Cyberpunk { 0.04 } else { 0.0 }, title_w);
    let title_h = tg.size().y.max(22.0);
    let one_line = tg.rows.len() <= 1;
    let tw_used = tg.size().x;
    paint_galley(&p, th, Role::Display, pos2(x + 28.0, y + title_h / 2.0 - tg.size().y / 2.0), tg, title_c);
    let mut head_bottom = y + title_h;
    if let Some(res) = &s.result {
        let (fg, bgc, bd) = if s.state == StepState::Nil { (th.neutral, th.neutral_bg, th.line_strong) } else { result_chip_colors(th) };
        let radius = pill();
        let rw = text_width(ui, th, res, Role::MonoB, 14.08) + 22.0;
        let fits = one_line && x + 28.0 + tw_used + 6.0 + rw <= x + iw;
        let (rpos, anchor) = if fits { (pos2(x + iw, y + 11.0), Align2::RIGHT_CENTER) } else { (pos2(x, head_bottom + 4.0 + 12.0), Align2::LEFT_CENTER) };
        if !fits {
            head_bottom += 4.0 + 24.0;
        }
        chip(ui, th, rpos, anchor, res, Role::MonoB, 14.08, fg, bgc, Some((1.0, bd)), vec2(10.0, 1.0), radius);
    }
    y = head_bottom + 1.0;

    // formula
    let g = galley(ui, th, &s.formula, Role::SerifI, 11.52, th.muted, 0.0, iw);
    paint_galley(&p, th, Role::SerifI, pos2(x, y), g.clone(), th.muted);
    y += g.size().y;

    // equation chips
    if !s.eq.is_empty() {
        y += 5.0;
        let (mut cx, mut cy) = (x, y);
        let row_h = 21.0;
        for item in &s.eq {
            let (txt, kind) = match item {
                EqPart::Val(t) => (t.as_str(), 0),
                EqPart::Op(t) => (t.as_str(), 1),
                EqPart::Res(t) => (t.as_str(), 2),
            };
            let role = if kind == 1 { Role::Mono } else { Role::MonoB };
            let tw = text_width(ui, th, txt, role, 12.8);
            let iw_item = if kind == 1 { tw } else { tw + 16.0 + if kind == 0 { 2.0 } else { 0.0 } };
            if cx > x && cx + iw_item > x + iw {
                cx = x;
                cy += row_h + 3.0;
            }
            let mid = cy + row_h / 2.0;
            match kind {
                0 => {
                    chip(ui, th, pos2(cx, mid), Align2::LEFT_CENTER, txt, Role::MonoB, 12.8, th.text, th.surface2, Some((1.0, th.line)), vec2(8.0, 1.0), if th.palette == Palette::Pokemon { 7.0 } else { 6.0 });
                }
                1 => {
                    text(ui, th, pos2(cx, mid), Align2::LEFT_CENTER, txt, Role::Mono, 12.8, th.muted);
                }
                _ => {
                    let (fg, bgc, bd) = result_chip_colors(th);
                    chip(ui, th, pos2(cx, mid), Align2::LEFT_CENTER, txt, Role::MonoB, 12.8, fg, bgc, Some((1.0, bd)), vec2(8.0, 1.0), if th.palette == Palette::Pokemon { 7.0 } else { 6.0 });
                }
            }
            cx += iw_item + 5.0;
        }
        y = cy + row_h;
    }
    if let Some(h) = &s.hint {
        y += 4.0;
        let r = paragraph(ui, th, pos2(x, y), iw, h, Role::Serif, 12.16, th.muted, 1.22);
        y = r.bottom();
    }
    if !s.tags.is_empty() {
        y += 5.0;
        let (mut cx, mut cy) = (x, y);
        let mut row_h: f32 = 0.0;
        for (t, info) in &s.tags {
            let label = if *info { t.clone() } else { format!("≈\u{a0}{}", t) };
            let mut job = egui::text::LayoutJob::single_section(label, egui::text::TextFormat { font_id: th.font(Role::Sans, 12.8), color: if *info { th.muted } else { th.warn }, line_height: Some(17.3), ..Default::default() });
            job.wrap.max_width = iw - 18.0;
            let g = ui.fonts(|f| f.layout_job(job));
            let sz = g.size() + vec2(18.0, 6.0);
            if cx > x && cx + sz.x > x + iw {
                cx = x;
                cy += row_h + 4.0;
                row_h = 0.0;
            }
            let r = Rect::from_min_size(pos2(cx, cy), sz);
            let (fg, bgc, bd) = if *info { (th.muted, th.neutral_bg, th.line_strong) } else { (th.warn, th.warn_bg, th.warn) };
            let rad = if th.palette == Palette::Pokemon { 8.0 } else { 6.0 };
            fill(&p, r, rad, bgc);
            stroke_in(&p, r, rad, 1.0, bd);
            p.galley(r.min + vec2(9.0, 3.0), g, fg);
            row_h = row_h.max(sz.y);
            cx += sz.x + 4.0;
        }
        y = cy + row_h;
    }
    let h = (y + 8.0 + 1.0 - pos.y).max(min_h);
    let rect = Rect::from_min_size(pos, vec2(w, h));
    let rad = th.radius;
    if th.palette == Palette::Cyberpunk {
        p.set(bg_i, egui::Shape::convex_polygon(chamfer_points(rect, 5.0), if pending { th.surface2 } else { th.surface }, Stroke::NONE));
        cyber_stroke(&p, rect, 5.0, 1.0, if pending { th.line } else { rgba(th.accent, 0.34) });
        p.rect_filled(Rect::from_min_size(rect.min, vec2(2.0, rect.height().min(18.0))), 0.0, if pending { th.line_strong } else { th.accent });
    } else {
        p.set(bg_i, egui::Shape::rect_filled(rect, cr(rad), if pending { th.surface2 } else { th.surface }));
        stroke_in(&p, rect, rad, if th.palette == Palette::Pokemon { 1.5 } else { 1.0 }, if th.palette == Palette::Pokemon { th.line_strong } else { th.line });
    }
    h
}

// ---------------------------------------------------------------------------
// Dilution (drawn under the FBC sheet)
// ---------------------------------------------------------------------------

const DIL_W: f32 = 84.0;
const DIL_MIN_W: f32 = 62.0;
const DIL_GAP: f32 = 12.0;
const DIL_ROWS: [&str; 2] = ["1:2 diluted", "Neat (×2)"];

fn dil_label_w(ui: &Ui, th: &Theme) -> f32 {
    DIL_ROWS.iter().map(|s| text_width(ui, th, s, Role::Sans, 13.0)).fold(0.0, f32::max).ceil()
}

/// Inner width of the dilution sheet at full column width.
pub fn dilution_natural(ui: &Ui, th: &Theme) -> f32 {
    let n = DIL_PARAMS.len() as f32;
    dil_label_w(ui, th) + COL_GAP + n * DIL_W + (n - 1.0) * DIL_GAP
}

/// The FBC colour for a dilution parameter, so the two sheets read as one.
fn dil_color(th: &Theme, name: &str) -> Color32 {
    FBC_PARAMS.iter().find(|p| p.name == name).and_then(|p| p.color).map(|k| th.param[k].0).unwrap_or(th.text)
}

impl App {
    pub fn dilution_card(&mut self, ui: &mut Ui, th: &Theme, pos: Pos2, w: f32) -> f32 {
        let opts = CardOpts { title: "Dilution", unit: "", bar: None, head_bg: None, title_color: None, chip: None, min_height: 0.0, alert: false, status: None, collapsed: false };
        let rect = card(ui, th, pos, w, &opts, |ui, iw| {
            let o = ui.cursor().min;
            let lw = dil_label_w(ui, th);
            let n = DIL_PARAMS.len() as f32;
            let colw = ((iw - lw - COL_GAP - (n - 1.0) * DIL_GAP) / n).clamp(DIL_MIN_W, DIL_W);
            let inset = cell_inset(th);
            let rule = Stroke::new(1.0_f32, sheet_rule(th));
            let xc = |i: usize| o.x + lw + COL_GAP + i as f32 * (colw + DIL_GAP);

            // parameter headings, coloured like their FBC rows
            let hm = o.y + HEAD_ROW / 2.0 - 4.0;
            for (i, name) in DIL_PARAMS.iter().enumerate() {
                text(ui, th, pos2(xc(i) + inset, hm), Align2::LEFT_CENTER, name, Role::SansB, 13.5, dil_color(th, name));
            }
            let mut y = o.y + HEAD_ROW;
            ui.painter().line_segment([pos2(o.x, y - 0.5), pos2(o.x + iw, y - 0.5)], rule);

            // diluted entries
            let mid = y + ROW_H / 2.0;
            text(ui, th, pos2(o.x, mid), Align2::LEFT_CENTER, DIL_ROWS[0], Role::Sans, 13.0, th.muted);
            for i in 0..DIL_PARAMS.len() {
                let r = Rect::from_min_size(pos2(xc(i), mid - CELL_H / 2.0), vec2(colw, CELL_H));
                let invalid = parse_number(&self.dil[i]) == Num::Invalid;
                field(ui, th, r, Id::new(("dil", i)), &mut self.dil[i], "", invalid);
            }
            y += ROW_H;
            ui.painter().line_segment([pos2(o.x, y - 0.5), pos2(o.x + iw, y - 0.5)], rule);

            // calculated neat: results, so plain ink; only an unusable entry is red
            let mid = y + 38.0 / 2.0;
            text(ui, th, pos2(o.x, mid), Align2::LEFT_CENTER, DIL_ROWS[1], Role::Sans, 13.0, th.muted);
            for i in 0..DIL_PARAMS.len() {
                let p = pos2(xc(i) + inset, mid);
                match parse_number(&self.dil[i]) {
                    Num::Empty => {
                        text(ui, th, p, Align2::LEFT_CENTER, "—", Role::MonoB, 15.0, th.line_strong);
                    }
                    Num::Invalid => {
                        text(ui, th, p, Align2::LEFT_CENTER, "Check", Role::SansB, 12.5, th.bad);
                    }
                    Num::Val(v) => {
                        text(ui, th, p, Align2::LEFT_CENTER, &fmt_js(v * 2.0), Role::MonoB, 15.0, th.text);
                    }
                }
            }
            y += 38.0 - 6.0;
            ui.allocate_rect(Rect::from_min_max(o, pos2(o.x + iw, y)), Sense::hover());
        });

        let mut clear = false;
        header_buttons(ui, rect, |ui| {
            if button(ui, th, "Clear", Btn::MiniMuted, true).clicked() {
                clear = true;
            }
        });
        if clear {
            for s in self.dil.iter_mut() {
                s.clear();
            }
            ui.memory_mut(|m| m.request_focus(Id::new(("dil", 0usize))));
        }
        rect.bottom()
    }
}
