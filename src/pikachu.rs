//! Pokémon-theme mascot: a pixel-art Pikachu that roams the UI.
//!
//! Widgets register *perches* every frame — surfaces Pikachu can stand on
//! (buttons, cards, the theme rail, the window floor) or hang from (the
//! title-bar tabs). [`Pikachu::tick`] then advances a small behaviour state
//! machine and draws the sprite on a foreground layer that never takes input.
//!
//! The sprite is not a fixed sprite sheet. Every frame a parametric pose
//! (body, head, ears, tail, legs, face) is rasterised into a small pixel grid
//! and outlined, so walking, galloping, pouncing, hanging, rolling and so on
//! share one consistent pixel style while limbs, ears and tail move freely.
//! Positions are snapped to whole sprite pixels, which keeps it crisp.
//!
//! Behaviour is deliberately unpredictable: weighted random choices that avoid
//! repeating themselves, randomised durations, speeds and landing spots, an
//! energy level that makes it nap after a lot of running, and reactions to the
//! pointer (it watches a nearby cursor, startles at a fast one, and enjoys
//! being clicked).

use crate::theme::{hex, Role, Theme};
use eframe::egui::{
    self, epaint::StrokeKind, pos2, vec2, Align2, Color32, Id, LayerId, Order, Painter, Pos2, Rect,
    Shape, Stroke, Ui, Vec2,
};
use std::f32::consts::{PI, TAU};

/// Size of one sprite pixel, in points (snapped to whole physical pixels).
const S_PT: f32 = 2.0;

// ---------------------------------------------------------------------------
// Perches
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PerchKind {
    /// Stand on the top edge.
    Top,
    /// Hang from the bottom edge.
    Hang,
    /// The bottom of the window; always catches a fall.
    Floor,
}

#[derive(Clone, Copy, Debug)]
struct Perch {
    id: Id,
    x0: f32,
    x1: f32,
    y: f32,
    kind: PerchKind,
    pressed: bool,
}

fn key() -> Id {
    Id::new("pikachu-perches")
}

/// Clear last frame's perches. Call once at the start of every frame.
pub fn begin_frame(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp(key(), Vec::<Perch>::new()));
}

/// Offer `rect` as a perch for this frame. `id` must be stable across frames
/// so Pikachu stays attached when the surface moves (scrolling, resizing).
/// Surfaces clipped out of view are ignored. `pressed` lets a click on the
/// surface bounce whoever is sitting on it.
pub fn perch(ui: &Ui, id: Id, rect: Rect, kind: PerchKind, pressed: bool) {
    let clip = ui.clip_rect();
    let y = match kind {
        PerchKind::Hang => rect.bottom(),
        PerchKind::Floor => rect.bottom() - 2.0,
        PerchKind::Top => rect.top(),
    };
    let (min_y, max_y) = match kind {
        PerchKind::Top => (clip.top() + 16.0, clip.bottom() - 2.0),
        PerchKind::Hang => (clip.top(), clip.bottom() + 1.0),
        PerchKind::Floor => (clip.top(), clip.bottom() + 1.0),
    };
    if y < min_y || y > max_y {
        return;
    }
    let inset = if kind == PerchKind::Floor { 12.0 } else { 6.0 };
    let x0 = rect.left().max(clip.left()) + inset;
    let x1 = rect.right().min(clip.right()) - inset;
    if x1 - x0 < 16.0 {
        return;
    }
    ui.ctx().data_mut(|d| {
        d.get_temp_mut_or_default::<Vec<Perch>>(key()).push(Perch { id, x0, x1, y, kind, pressed })
    });
}

fn find(perches: &[Perch], id: Id) -> Option<Perch> {
    perches.iter().find(|p| p.id == id).copied()
}

/// The surface a right-click at `g` points at: a tab just above it, or the
/// nearest surface at (or a little above) the click, falling back to the floor.
fn goal_perch(perches: &[Perch], g: Pos2) -> Option<Perch> {
    let mut best: Option<(f32, Perch)> = None;
    for p in perches {
        if g.x < p.x0 - 12.0 || g.x > p.x1 + 12.0 {
            continue;
        }
        let d = match p.kind {
            PerchKind::Hang if g.y <= p.y + 4.0 && p.y - g.y < 48.0 => p.y - g.y,
            PerchKind::Hang => continue,
            _ if p.y >= g.y - 28.0 => (p.y - g.y).abs(),
            _ => continue,
        };
        if best.as_ref().map_or(true, |b| d < b.0) {
            best = Some((d, *p));
        }
    }
    best.map(|b| b.1)
}

// ---------------------------------------------------------------------------
// Behaviour
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Idle {
    Sit,
    Look,
    Wag,
    Scratch,
    Sniff,
    Stretch,
    Wave,
    Spark,
    Yawn,
    Sleep,
    Dance,
    Curious,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Gait {
    Walk,
    Run,
    Roll,
    Shimmy,
}

#[derive(Clone, Copy, Debug)]
struct Target {
    id: Id,
    dx: f32,
    kind: PerchKind,
    /// A long horizontal leap, drawn as a four-legged pounce.
    pounce: bool,
}

#[derive(Clone, Copy, Debug)]
enum Act {
    Idle { kind: Idle, dur: f32 },
    Move { to: f32, gait: Gait },
    Crouch { dur: f32, target: Target },
    Jump { target: Target, from: Pos2, dur: f32, h: f32, from_hang: bool },
    Land { dur: f32 },
    Fall,
    Hang { dur: f32 },
    Hop { n: u8 },
    Startle { away: f32 },
    Trip,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PartKind {
    Dust,
    Spark,
    Z,
    Heart,
    Note,
    Star,
    /// Expanding ring where the user right-clicked.
    Ping,
}

#[derive(Clone, Copy)]
struct Particle {
    pos: Pos2,
    vel: Vec2,
    life: f32,
    max: f32,
    kind: PartKind,
}

const LINES: [&str; 6] = ["Pika!", "Pika pika!", "Pikachu!", "Chu~", "Pi?", "Pikaaa!"];

pub struct Pikachu {
    rng: u64,
    /// Whether Pikachu was on screen last frame (re-entering spawns it again).
    on: bool,
    /// Feet position, or the hand position while hanging.
    pos: Pos2,
    /// Current perch: id, x offset from the perch's left end, kind.
    ground: Option<(Id, f32, PerchKind)>,
    prev_perch: Option<Id>,
    facing: f32,
    act: Act,
    t: f32,
    clock: f32,
    energy: f32,
    vel: Vec2,
    speed: f32,
    speed_mul: f32,
    phase: f32,
    run_happy: bool,
    blink_in: f32,
    blink_t: f32,
    twitch_in: f32,
    twitch_t: f32,
    turn_t: f32,
    face_cd: f32,
    swing: f32,
    swing_v: f32,
    kick_t: f32,
    fx_t: f32,
    bump_latch: bool,
    greet: bool,
    last_choice: u8,
    bubble: Option<(&'static str, f32)>,
    parts: Vec<Particle>,
    /// Where the user last right-clicked, and how long Pikachu stays interested.
    goal: Option<(Pos2, f32)>,
    goal_new: bool,
}

impl Default for Pikachu {
    fn default() -> Self {
        Self::new()
    }
}

impl Pikachu {
    pub fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e37_79b9_7f4a_7c15)
            | 1;
        Pikachu {
            rng: seed,
            on: false,
            pos: Pos2::ZERO,
            ground: None,
            prev_perch: None,
            facing: 1.0,
            act: Act::Fall,
            t: 0.0,
            clock: 0.0,
            energy: 0.85,
            vel: Vec2::ZERO,
            speed: 0.0,
            speed_mul: 1.0,
            phase: 0.0,
            run_happy: false,
            blink_in: 2.0,
            blink_t: 0.0,
            twitch_in: 3.0,
            twitch_t: 0.0,
            turn_t: 0.0,
            face_cd: 0.0,
            swing: 0.0,
            swing_v: 0.0,
            kick_t: 0.0,
            fx_t: 0.0,
            bump_latch: false,
            greet: true,
            last_choice: 255,
            bubble: None,
            parts: Vec::new(),
            goal: None,
            goal_new: false,
        }
    }

    // --- randomness --------------------------------------------------------

    fn rnd(&mut self) -> f32 {
        // xorshift64*
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        (x.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.rnd()
    }

    fn chance(&mut self, p: f32) -> bool {
        self.rnd() < p
    }

    fn sign(&mut self) -> f32 {
        if self.chance(0.5) {
            1.0
        } else {
            -1.0
        }
    }

    fn weighted(&mut self, opts: &[(f32, u8)]) -> u8 {
        let total: f32 = opts.iter().map(|o| o.0.max(0.0)).sum();
        let mut r = self.rnd() * total;
        for o in opts {
            if o.0 <= 0.0 {
                continue;
            }
            if r < o.0 {
                return o.1;
            }
            r -= o.0;
        }
        0
    }

    fn say(&mut self) {
        let i = (self.rnd() * LINES.len() as f32) as usize % LINES.len();
        self.bubble = Some((LINES[i], 1.5));
    }

    // --- frame ---------------------------------------------------------------

    /// Advance and draw. `enabled` is false outside the Pokémon theme.
    pub fn tick(&mut self, ctx: &egui::Context, th: &Theme, enabled: bool) {
        if !enabled {
            self.on = false;
            return;
        }
        let perches: Vec<Perch> = ctx.data(|d| d.get_temp(key())).unwrap_or_default();
        if perches.is_empty() {
            return;
        }
        let screen = ctx.screen_rect();
        let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.05);
        self.clock += dt;
        if !self.on {
            self.spawn(&perches, screen);
            self.on = true;
        }

        // Follow the perch (it may have scrolled or resized), or fall off it.
        if let Some((id, dx, _)) = self.ground {
            match find(&perches, id) {
                Some(p) => {
                    let dx = dx.clamp(0.0, p.x1 - p.x0);
                    self.ground = Some((id, dx, p.kind));
                    self.pos = pos2(p.x0 + dx, p.y);
                    if p.pressed && !self.bump_latch {
                        self.bumped(p.kind);
                    }
                    self.bump_latch = p.pressed;
                }
                None => {
                    if matches!(self.ground, Some((_, _, PerchKind::Hang))) {
                        self.pos.y += HANG_Y * S_PT;
                    }
                    self.ground = None;
                    self.vel = Vec2::ZERO;
                    self.set(Act::Fall);
                }
            }
        }

        self.update_act(dt, &perches, screen);
        self.pointer(ctx);
        self.timers(dt);
        self.update_particles(dt);
        self.draw(ctx, th, &perches);
        ctx.request_repaint();
    }

    fn set(&mut self, a: Act) {
        self.act = a;
        self.t = 0.0;
    }

    fn face(&mut self, dir: f32) {
        if dir != 0.0 && dir.signum() != self.facing {
            self.facing = dir.signum();
            self.turn_t = 0.1;
        }
    }

    fn spawn(&mut self, perches: &[Perch], screen: Rect) {
        let tops: Vec<Perch> = perches.iter().filter(|p| p.kind != PerchKind::Hang).copied().collect();
        let Some(&first) = tops.first() else { return };
        let i = (self.rnd() * tops.len() as f32) as usize;
        let p = tops.get(i).copied().unwrap_or(first);
        let x = self.range(p.x0, p.x1);
        self.pos = pos2(x, screen.top() - 10.0);
        self.ground = None;
        self.vel = Vec2::ZERO;
        self.parts.clear();
        self.bubble = None;
        self.goal = None;
        self.goal_new = false;
        self.greet = true;
        self.facing = self.sign();
        self.set(Act::Fall);
    }

    fn attach(&mut self, p: &Perch) {
        let cur = self.ground.map(|g| g.0);
        if cur != Some(p.id) {
            self.prev_perch = cur.or(self.prev_perch);
        }
        self.ground = Some((p.id, (self.pos.x - p.x0).clamp(0.0, p.x1 - p.x0), p.kind));
        self.pos.y = p.y;
        self.bump_latch = p.pressed;
    }

    fn slide(&mut self, d: f32, perches: &[Perch]) {
        if let Some((id, dx, kind)) = self.ground {
            if let Some(p) = find(perches, id) {
                let nd = (dx + d).clamp(0.0, p.x1 - p.x0);
                self.ground = Some((id, nd, kind));
                self.pos.x = p.x0 + nd;
            }
        }
    }

    fn update_act(&mut self, dt: f32, perches: &[Perch], screen: Rect) {
        self.t += dt;
        // A fresh right-click interrupts whatever it can; mid-air moves finish
        // first and pick the goal up when they next decide what to do.
        if self.goal_new {
            self.goal_new = false;
            let busy = matches!(
                self.act,
                Act::Jump { .. } | Act::Fall | Act::Crouch { .. } | Act::Hop { .. } | Act::Startle { .. } | Act::Trip
            );
            if !busy && self.ground.is_some() {
                if matches!(self.act, Act::Idle { kind: Idle::Sleep, .. }) {
                    self.bubble = Some(("!", 0.8));
                    self.last_choice = 255;
                }
                self.speed = 0.0;
                if !self.pursue(perches) {
                    self.next(perches);
                }
                return;
            }
        }
        match self.act {
            Act::Idle { kind, dur } => {
                self.idle_tick(kind, dt);
                if self.t >= dur {
                    self.next(perches);
                }
            }
            Act::Move { to, gait } => self.move_tick(to, gait, dt, perches),
            Act::Crouch { dur, target } => {
                if self.t >= dur {
                    self.launch(target, perches);
                }
            }
            Act::Jump { target, from, dur, h, .. } => match find(perches, target.id) {
                None => {
                    self.vel = vec2(self.facing * 60.0, 0.0);
                    self.ground = None;
                    self.set(Act::Fall);
                }
                Some(p) => {
                    let to = pos2(p.x0 + target.dx.clamp(0.0, p.x1 - p.x0), p.y);
                    let u = (self.t / dur).min(1.0);
                    self.pos = from.lerp(to, u) + vec2(0.0, -h * 4.0 * u * (1.0 - u));
                    if u >= 1.0 {
                        self.attach(&p);
                        if p.kind == PerchKind::Hang {
                            self.swing = self.facing * 3.5;
                            self.swing_v = 0.0;
                            let dur = self.range(1.8, 5.0);
                            self.set(Act::Hang { dur });
                        } else {
                            self.land(500.0);
                        }
                    }
                }
            },
            Act::Land { dur } => {
                if self.t >= dur {
                    self.next(perches);
                }
            }
            Act::Fall => self.fall_tick(dt, perches, screen),
            Act::Hang { dur } => {
                self.swing_tick(dt, 0.0);
                if self.chance(0.6 * dt) {
                    self.kick_t = 0.6;
                    self.swing_v += self.range(-5.0, 5.0);
                }
                if self.t >= dur {
                    self.next(perches);
                }
            }
            Act::Hop { n } => {
                if self.t >= 0.34 * n as f32 {
                    let (kind, dur) = if self.chance(0.5) { (Idle::Wave, self.range(0.8, 1.4)) } else { (Idle::Sit, self.range(0.8, 2.0)) };
                    self.set(Act::Idle { kind, dur });
                }
            }
            Act::Startle { away } => {
                if self.t >= 0.45 {
                    match self.ground {
                        Some((_, dx, kind)) if kind != PerchKind::Hang => {
                            let away = if away == 0.0 { self.sign() } else { away };
                            let to = dx + away * self.range(90.0, 220.0);
                            self.start_move(to, Gait::Run);
                        }
                        _ => self.next(perches),
                    }
                }
            }
            Act::Trip => {
                if self.t < 0.35 {
                    self.phase += dt * 16.0;
                    let d = self.facing * 120.0 * (1.0 - self.t / 0.35) * dt;
                    self.slide(d, perches);
                } else if self.chance(5.0 * dt) {
                    let hp = self.head_pos();
                    let vx = self.range(-25.0, 25.0);
                    self.emit(PartKind::Star, hp + vec2(0.0, -6.0), vec2(vx, -30.0), 0.7);
                }
                if self.t > 1.9 {
                    self.bubble = Some(("?", 1.2));
                    let dur = self.range(1.0, 2.0);
                    self.set(Act::Idle { kind: Idle::Sit, dur });
                }
            }
        }
    }

    fn idle_tick(&mut self, kind: Idle, dt: f32) {
        self.energy += if kind == Idle::Sleep { 0.12 } else { 0.03 } * dt;
        self.fx_t -= dt;
        match kind {
            Idle::Sit => {
                if self.face_cd <= 0.0 && self.chance(0.15 * dt) {
                    let f = -self.facing;
                    self.face(f);
                }
            }
            Idle::Look => {
                if self.face_cd <= 0.0 {
                    let f = -self.facing;
                    self.face(f);
                    self.face_cd = self.range(0.45, 1.5);
                }
            }
            Idle::Sleep => {
                if self.fx_t <= 0.0 {
                    self.fx_t = self.range(0.9, 1.5);
                    let hp = self.head_pos() + vec2(self.facing * -4.0, -10.0);
                    self.emit(PartKind::Z, hp, vec2(self.facing * -6.0, -16.0), 1.8);
                }
            }
            Idle::Spark => {
                if self.fx_t <= 0.0 {
                    self.fx_t = self.range(0.05, 0.12);
                    let c = self.cheek_pos();
                    let a = self.range(-PI, 0.0);
                    let sp = self.range(40.0, 90.0);
                    let life = self.range(0.2, 0.4);
                    self.emit(PartKind::Spark, c, vec2(a.cos() * sp, a.sin() * sp), life);
                }
            }
            Idle::Dance => {
                if self.fx_t <= 0.0 {
                    self.fx_t = self.range(0.4, 0.7);
                    let hp = self.head_pos() + vec2(self.range(-10.0, 10.0), -14.0);
                    let vx = self.range(-12.0, 12.0);
                    self.emit(PartKind::Note, hp, vec2(vx, -22.0), 1.2);
                }
                if self.face_cd <= 0.0 {
                    let f = -self.facing;
                    self.face(f);
                    self.face_cd = 0.77;
                }
            }
            _ => {}
        }
    }

    fn start_move(&mut self, to: f32, gait: Gait) {
        let Some((_, dx, _)) = self.ground else { return };
        self.face((to - dx).signum());
        self.speed = 0.0;
        self.speed_mul = self.range(0.8, 1.25);
        self.run_happy = self.chance(0.45);
        self.set(Act::Move { to, gait });
    }

    fn move_tick(&mut self, to: f32, gait: Gait, dt: f32, perches: &[Perch]) {
        let Some((id, dx, kind)) = self.ground else {
            self.set(Act::Fall);
            return;
        };
        let Some(p) = find(perches, id) else { return };
        let w = p.x1 - p.x0;
        let to = to.clamp(0.0, w);
        let dist = to - dx;
        let max = match gait {
            Gait::Walk => 34.0,
            Gait::Run => 150.0,
            Gait::Roll => 110.0,
            Gait::Shimmy => 46.0,
        } * self.speed_mul;
        let acc = max * if gait == Gait::Run { 3.0 } else { 4.5 };
        let desired = dist.signum() * max.min((1.6 * acc * dist.abs()).sqrt());
        self.speed += (desired - self.speed).clamp(-acc * dt, acc * dt);
        let nd = (dx + self.speed * dt).clamp(0.0, w);
        self.ground = Some((id, nd, kind));
        self.pos.x = p.x0 + nd;
        let moved = self.speed.abs() * dt;
        match gait {
            Gait::Walk => self.phase = (self.phase + moved / 11.0).fract(),
            Gait::Run => self.phase = (self.phase + moved / 34.0).fract(),
            Gait::Shimmy => self.phase = (self.phase + moved / 14.0).fract(),
            Gait::Roll => self.phase += moved / (7.0 * S_PT),
        }
        if gait == Gait::Shimmy {
            let target = -self.speed * 0.05;
            self.swing += (target - self.swing) * (dt * 5.0).min(1.0);
        }
        if matches!(gait, Gait::Run | Gait::Roll) {
            self.energy -= 0.05 * dt;
            self.fx_t -= dt;
            if self.fx_t <= 0.0 && self.speed.abs() > 60.0 {
                self.fx_t = self.range(0.07, 0.14);
                let at = self.pos + vec2(-self.facing * 8.0, -1.0);
                let vx = -self.facing * self.range(10.0, 40.0);
                let vy = -self.range(8.0, 22.0);
                self.emit(PartKind::Dust, at, vec2(vx, vy), 0.4);
            }
            if gait == Gait::Run && self.speed.abs() > 100.0 && self.chance(0.035 * dt) {
                self.set(Act::Trip);
                return;
            }
        }
        let arrived = (to - nd).abs() < 0.8 && self.speed.abs() < max * 0.35;
        let blocked = (nd <= 0.0 && dist < 0.0) || (nd >= w && dist > 0.0);
        if arrived || blocked {
            self.speed = 0.0;
            if kind == PerchKind::Hang {
                let dur = self.range(1.2, 3.5);
                self.set(Act::Hang { dur });
                return;
            }
            if gait == Gait::Run && self.chance(0.55) {
                let f = self.facing;
                if let Some(tg) = self.pick_jump(perches, Some(f)) {
                    self.launch(tg, perches);
                    return;
                }
            }
            if gait == Gait::Run && self.chance(0.4) {
                // skid to a stop
                for _ in 0..3 {
                    let vx = self.facing * self.range(20.0, 50.0);
                    let at = self.pos + vec2(self.facing * 4.0, -1.0);
                    self.emit(PartKind::Dust, at, vec2(vx, -10.0), 0.35);
                }
                self.set(Act::Land { dur: 0.22 });
                return;
            }
            self.next(perches);
        }
    }

    fn swing_tick(&mut self, dt: f32, drive: f32) {
        self.swing_v += (-20.0 * (self.swing - drive) - 1.0 * self.swing_v) * dt;
        self.swing = (self.swing + self.swing_v * dt).clamp(-6.0, 6.0);
    }

    fn fall_tick(&mut self, dt: f32, perches: &[Perch], screen: Rect) {
        let prev = self.pos.y;
        self.vel.y = (self.vel.y + 1700.0 * dt).min(1100.0);
        self.vel.x *= (1.0 - 1.5 * dt).max(0.0);
        self.pos += self.vel * dt;
        self.pos.x = self.pos.x.clamp(screen.left() + 12.0, screen.right() - 12.0);
        if self.vel.y > 0.0 {
            let mut hit: Option<Perch> = None;
            for p in perches {
                if p.kind == PerchKind::Hang || self.pos.x < p.x0 || self.pos.x > p.x1 {
                    continue;
                }
                if prev <= p.y + 0.5 && self.pos.y >= p.y && hit.map_or(true, |h| p.y < h.y) {
                    hit = Some(*p);
                }
            }
            if let Some(p) = hit {
                let impact = self.vel.y;
                self.attach(&p);
                self.land(impact);
                return;
            }
        }
        if self.pos.y > screen.bottom() + 80.0 {
            self.on = false;
        }
    }

    fn land(&mut self, impact: f32) {
        let n = 2 + (impact / 250.0) as usize;
        for _ in 0..n.min(6) {
            let vx = self.range(-50.0, 50.0);
            let vy = -self.range(5.0, 25.0);
            self.emit(PartKind::Dust, self.pos + vec2(vx * 0.12, -1.0), vec2(vx, vy), 0.4);
        }
        self.energy -= 0.02;
        let dur = 0.1 + (impact / 1800.0).min(0.2);
        self.set(Act::Land { dur });
        if self.greet {
            self.greet = false;
            self.bubble = Some(("Pika pika!", 1.8));
        } else if self.chance(0.07) {
            self.say();
        }
    }

    fn launch(&mut self, tg: Target, perches: &[Perch]) {
        let Some(p) = find(perches, tg.id) else {
            self.next(perches);
            return;
        };
        let from_hang = matches!(self.ground, Some((_, _, PerchKind::Hang)));
        let to = pos2(p.x0 + tg.dx.clamp(0.0, p.x1 - p.x0), p.y);
        let from = self.pos;
        let dx = to.x - from.x;
        let up = from.y - to.y;
        let dist = (dx * dx + up * up).sqrt();
        let (dur, h) = if tg.kind == PerchKind::Hang {
            ((0.26 + dist / 1300.0).clamp(0.26, 0.6), 6.0 + 0.08 * dx.abs())
        } else {
            let dur = (0.3 + dist / 1150.0 + up.max(0.0) / 1000.0).clamp(0.28, 0.85);
            let h = if up > 0.0 { up * 0.55 + 14.0 + 0.12 * dx.abs() } else { 10.0 + 0.16 * dx.abs() };
            (dur, h * self.range(0.85, 1.2))
        };
        if dx.abs() > 3.0 {
            self.face(dx);
        }
        if !from_hang {
            for _ in 0..2 {
                let vx = -self.facing * self.range(10.0, 40.0);
                self.emit(PartKind::Dust, self.pos + vec2(0.0, -1.0), vec2(vx, -12.0), 0.35);
            }
        }
        self.ground = None;
        self.energy -= 0.03;
        self.set(Act::Jump { target: tg, from, dur, h, from_hang });
    }

    /// Can Pikachu jump from where it is to `x` on perch `p`?
    fn reachable(&self, p: &Perch, x: f32) -> bool {
        let cur = self.ground.map(|g| g.0);
        let cur_kind = self.ground.map(|g| g.2);
        if Some(p.id) == cur {
            return false;
        }
        match (cur_kind, p.kind) {
            (Some(PerchKind::Floor), PerchKind::Floor) => return false,
            (Some(PerchKind::Hang), k) if k != PerchKind::Hang => return false,
            _ => {}
        }
        let dx = x - self.pos.x;
        let up = self.pos.y - p.y;
        let max_up = if cur_kind == Some(PerchKind::Floor) { 430.0 } else { 240.0 };
        if dx.abs() > 460.0 || up > max_up || up < -520.0 {
            return false;
        }
        if p.kind == PerchKind::Hang && cur_kind != Some(PerchKind::Hang) && (up < 20.0 || dx.abs() > 260.0) {
            return false;
        }
        if cur_kind == Some(PerchKind::Hang) && dx.abs() > 240.0 {
            return false;
        }
        !(dx.abs() < 14.0 && up.abs() < 14.0)
    }

    /// Crouch and leap to `tg` (straight away when swinging from a tab).
    fn jump_via(&mut self, tg: Target, perches: &[Perch]) {
        if matches!(self.ground, Some((_, _, PerchKind::Hang))) {
            self.launch(tg, perches);
            return;
        }
        if let Some(q) = find(perches, tg.id) {
            let dir = q.x0 + tg.dx - self.pos.x;
            if dir.abs() > 3.0 {
                self.face(dir);
            }
        }
        let dur = self.range(0.12, 0.26);
        self.set(Act::Crouch { dur, target: tg });
    }

    fn pick_jump(&mut self, perches: &[Perch], dir: Option<f32>) -> Option<Target> {
        let from = self.pos;
        let mut best: Option<(f32, Target)> = None;
        for p in perches {
            let spread = self.range(-80.0, 80.0);
            let x = (from.x + spread).clamp(p.x0 + 4.0, (p.x1 - 4.0).max(p.x0 + 4.0));
            if !self.reachable(p, x) {
                continue;
            }
            let dx = x - from.x;
            let up = from.y - p.y;
            if let Some(d) = dir {
                if dx * d < 10.0 {
                    continue;
                }
            }
            let dist = (dx * dx + up * up).sqrt();
            let mut score = self.range(0.15, 1.0) / (1.0 + dist / 240.0);
            if Some(p.id) == self.prev_perch {
                score *= 0.4;
            }
            match p.kind {
                PerchKind::Hang => score *= 1.2,
                PerchKind::Floor => score *= 0.35,
                PerchKind::Top => {}
            }
            if best.as_ref().map_or(true, |b| score > b.0) {
                let pounce = dx.abs() > 34.0 && p.kind != PerchKind::Hang;
                best = Some((score, Target { id: p.id, dx: x - p.x0, kind: p.kind, pounce }));
            }
        }
        best.map(|b| b.1)
    }

    fn next(&mut self, perches: &[Perch]) {
        // A right-click goal wins most decisions, but not every one: now and
        // then Pikachu still gets distracted on the way.
        if self.goal.is_some() && self.chance(0.88) && self.pursue(perches) {
            return;
        }
        match self.ground.map(|g| g.2) {
            None => self.set(Act::Fall),
            Some(PerchKind::Hang) => self.next_hang(perches),
            Some(_) => self.next_ground(perches),
        }
    }

    fn next_ground(&mut self, perches: &[Perch]) {
        let Some((id, dx, kind)) = self.ground else { return };
        let Some(p) = find(perches, id) else { return };
        let w = p.x1 - p.x0;
        self.energy = self.energy.clamp(0.0, 1.0);
        let e = self.energy;

        // Waking up is always followed by a stretch or a yawn.
        if self.last_choice == 9 {
            let (kind, c) = if self.chance(0.5) { (Idle::Stretch, 5) } else { (Idle::Yawn, 8) };
            self.last_choice = c;
            self.set(Act::Idle { kind, dur: 1.4 });
            return;
        }

        let jump = self.pick_jump(perches, None);
        let floor = kind == PerchKind::Floor;
        let mut opts: Vec<(f32, u8)> = vec![
            (3.0, 0),
            (1.3, 1),
            (1.0, 2),
            (0.6, 3),
            (0.7, 4),
            (0.4, 5),
            (0.5, 6),
            (0.55, 7),
            (0.3 + 0.8 * (1.0 - e), 8),
            (if e < 0.25 { 5.0 } else { 0.08 }, 9),
            (0.35 * e, 10),
            (if w > 40.0 { 2.4 } else { 0.0 }, 20),
            (if w > 120.0 { 1.6 * e + 0.2 } else { 0.0 }, 21),
            (if w > 90.0 { 0.55 * e } else { 0.0 }, 22),
            (if jump.is_some() { if floor { 6.0 } else { 2.8 } } else { 0.0 }, 30),
        ];
        for o in &mut opts {
            if o.1 == self.last_choice {
                o.0 *= 0.25;
            }
        }
        let c = self.weighted(&opts);
        self.last_choice = c;
        let idle = |k: Idle, dur: f32| Act::Idle { kind: k, dur };
        match c {
            0 => {
                let d = self.range(1.2, 4.5);
                self.set(idle(Idle::Sit, d))
            }
            1 => {
                let d = self.range(1.5, 3.5);
                self.set(idle(Idle::Look, d))
            }
            2 => {
                let d = self.range(1.2, 2.6);
                self.set(idle(Idle::Wag, d))
            }
            3 => {
                let d = self.range(0.9, 1.6);
                self.set(idle(Idle::Scratch, d))
            }
            4 => {
                let d = self.range(1.0, 2.2);
                self.set(idle(Idle::Sniff, d))
            }
            5 => self.set(idle(Idle::Stretch, 1.4)),
            6 => {
                let d = self.range(1.0, 1.8);
                if self.chance(0.5) {
                    self.say();
                }
                self.set(idle(Idle::Wave, d))
            }
            7 => {
                let d = self.range(0.9, 1.4);
                if self.chance(0.5) {
                    self.bubble = Some(("Pika!", 1.2));
                }
                self.set(idle(Idle::Spark, d))
            }
            8 => self.set(idle(Idle::Yawn, 1.4)),
            9 => {
                let d = self.range(6.0, 14.0);
                self.fx_t = 0.6;
                self.set(idle(Idle::Sleep, d))
            }
            10 => {
                let d = self.range(2.0, 3.5);
                self.set(idle(Idle::Dance, d))
            }
            20 => {
                let mut off = self.range(-130.0, 130.0);
                if off.abs() < 12.0 {
                    off = 30.0 * off.signum();
                }
                let mut to = dx + off;
                if to < 0.0 || to > w {
                    to = dx - off;
                }
                self.start_move(to, Gait::Walk)
            }
            21 => {
                let to = self.range(0.0, w);
                self.start_move(to, Gait::Run)
            }
            22 => {
                let to = self.range(0.0, w);
                self.start_move(to, Gait::Roll)
            }
            30 => {
                if let Some(target) = jump {
                    self.jump_via(target, perches);
                } else {
                    self.set(idle(Idle::Sit, 1.0));
                }
            }
            _ => self.set(idle(Idle::Sit, 1.0)),
        }
    }

    /// Take one step toward the right-click goal: walk or run along this
    /// surface, leap straight to the goal's surface, or hop to whichever
    /// reachable surface gets closest. Returns false when there is nothing
    /// useful to do (the goal is then dropped).
    fn pursue(&mut self, perches: &[Perch]) -> bool {
        let Some((g, _)) = self.goal else { return false };
        let Some((cur, dx, kind)) = self.ground else { return false };
        let Some(here) = find(perches, cur) else { return false };
        let Some(dest) = goal_perch(perches, g) else {
            self.goal = None;
            return false;
        };
        let gdx = (g.x - dest.x0).clamp(0.0, dest.x1 - dest.x0);
        let spot = pos2(dest.x0 + gdx, dest.y);

        if dest.id == cur {
            let dist = spot.x - self.pos.x;
            if dist.abs() < 10.0 {
                // Made it.
                self.goal = None;
                self.say();
                if kind == PerchKind::Hang {
                    self.swing_v += self.facing * 5.0;
                    let dur = self.range(1.5, 3.0);
                    self.set(Act::Hang { dur });
                } else {
                    let hp = self.head_pos() + vec2(0.0, -12.0);
                    self.emit(PartKind::Heart, hp, vec2(0.0, -35.0), 1.0);
                    self.set(Act::Hop { n: 1 });
                }
                return true;
            }
            let gait = if kind == PerchKind::Hang {
                Gait::Shimmy
            } else if dist.abs() > 90.0 {
                Gait::Run
            } else if self.chance(0.12) {
                Gait::Roll
            } else {
                Gait::Walk
            };
            self.start_move(gdx, gait);
            return true;
        }

        // From a tab, get above the goal first, then let go.
        if kind == PerchKind::Hang && dest.kind != PerchKind::Hang {
            let to = (spot.x - here.x0).clamp(0.0, here.x1 - here.x0);
            // only shimmy for a short adjustment; otherwise let go and run
            if (to - dx).abs() > 16.0 && (to - dx).abs() < 70.0 && (spot.x - (here.x0 + to)).abs() < 30.0 {
                self.start_move(to, Gait::Shimmy);
            } else {
                self.drop_from_hang();
            }
            return true;
        }

        let pounce = |x: f32, from: f32, k: PerchKind| (x - from).abs() > 34.0 && k != PerchKind::Hang;
        if self.reachable(&dest, spot.x) {
            let tg = Target { id: dest.id, dx: gdx, kind: dest.kind, pounce: pounce(spot.x, self.pos.x, dest.kind) };
            self.jump_via(tg, perches);
            return true;
        }

        let d_now = (spot - self.pos).length();
        let mut best: Option<(f32, Target)> = None;
        for p in perches {
            let x = spot.x.clamp(p.x0 + 4.0, (p.x1 - 4.0).max(p.x0 + 4.0));
            if !self.reachable(p, x) {
                continue;
            }
            let d = (spot - pos2(x, p.y)).length();
            if d < d_now - 20.0 && best.as_ref().map_or(true, |b| d < b.0) {
                best = Some((d, Target { id: p.id, dx: x - p.x0, kind: p.kind, pounce: pounce(x, self.pos.x, p.kind) }));
            }
        }
        if let Some((_, tg)) = best {
            self.jump_via(tg, perches);
            return true;
        }

        // Nothing closer in reach: head for the goal's side of this surface.
        let to = (spot.x - here.x0).clamp(0.0, here.x1 - here.x0);
        if (to - dx).abs() > 8.0 && kind != PerchKind::Hang {
            self.start_move(to, Gait::Run);
            return true;
        }
        self.goal = None;
        self.bubble = Some(("Pi...?", 1.2));
        false
    }

    fn set_goal(&mut self, at: Pos2) {
        self.goal = Some((at, 15.0));
        self.goal_new = true;
        self.emit(PartKind::Ping, at, Vec2::ZERO, 0.5);
        if self.chance(0.4) {
            self.bubble = Some(if self.chance(0.5) { ("Pika!", 1.0) } else { ("Pi?", 1.0) });
        }
    }

    fn next_hang(&mut self, perches: &[Perch]) {
        let Some((id, _, _)) = self.ground else { return };
        let Some(p) = find(perches, id) else { return };
        let r = self.rnd();
        if r < 0.38 {
            self.drop_from_hang();
            if self.chance(0.25) {
                self.bubble = Some(("Wheee!", 1.0));
            }
        } else if r < 0.68 {
            let to = self.range(0.0, p.x1 - p.x0);
            self.start_move(to, Gait::Shimmy);
        } else if let Some(tg) = self.pick_jump(perches, None) {
            self.launch(tg, perches);
        } else {
            let to = self.range(0.0, p.x1 - p.x0);
            self.start_move(to, Gait::Shimmy);
        }
    }

    fn drop_from_hang(&mut self) {
        self.ground = None;
        self.pos.y += HANG_Y * S_PT;
        self.vel = vec2(self.swing * 10.0, -30.0);
        self.set(Act::Fall);
    }

    /// Someone pressed the surface Pikachu is on.
    fn bumped(&mut self, kind: PerchKind) {
        match kind {
            PerchKind::Hang => {
                self.bubble = Some(("!", 0.9));
                self.drop_from_hang();
            }
            PerchKind::Top => {
                if !matches!(self.act, Act::Startle { .. } | Act::Hop { .. } | Act::Jump { .. }) {
                    self.bubble = Some(("!", 0.9));
                    let away = self.sign();
                    self.set(Act::Startle { away });
                }
            }
            PerchKind::Floor => {}
        }
    }

    fn pointer(&mut self, ctx: &egui::Context) {
        let (hover, vel, pressed) =
            ctx.input(|i| (i.pointer.hover_pos(), i.pointer.velocity(), i.pointer.primary_pressed()));
        let Some(hp) = hover else { return };
        if ctx.input(|i| i.pointer.secondary_pressed()) {
            self.set_goal(hp);
            return;
        }
        let body = self.body_rect();
        let d = (hp - body.center()).length();
        if pressed && body.expand(4.0).contains(hp) {
            self.petted();
            return;
        }
        let fast = vel.length() > 1400.0;
        match self.act {
            Act::Hang { .. } => {
                if d < 70.0 && fast {
                    self.swing_v += vel.x.signum() * 8.0;
                }
            }
            Act::Idle { kind, .. } if self.ground.is_some() => {
                if kind == Idle::Sleep {
                    if d < 50.0 && fast {
                        self.bubble = Some(("!", 0.9));
                        let away = (self.pos.x - hp.x).signum();
                        self.set(Act::Startle { away });
                    }
                    return;
                }
                if d < 60.0 && fast {
                    self.bubble = Some(("!", 0.9));
                    let away = (self.pos.x - hp.x).signum();
                    self.set(Act::Startle { away });
                    return;
                }
                let watching = matches!(kind, Idle::Sit | Idle::Look | Idle::Wag | Idle::Curious | Idle::Wave);
                if watching && d < 170.0 && self.face_cd <= 0.0 && (hp.x - self.pos.x).abs() > 12.0 {
                    self.face(hp.x - self.pos.x);
                    self.face_cd = 0.7;
                }
                if matches!(kind, Idle::Sit | Idle::Look) && d < 130.0 && self.t > 0.5 && self.chance(0.6 * ctx.input(|i| i.stable_dt)) {
                    if self.chance(0.4) {
                        self.bubble = Some(("?", 1.0));
                    }
                    let dur = self.range(1.2, 2.0);
                    self.set(Act::Idle { kind: Idle::Curious, dur });
                }
            }
            Act::Move { gait: Gait::Walk, .. } => {
                if d < 60.0 && fast {
                    self.bubble = Some(("!", 0.9));
                    let away = (self.pos.x - hp.x).signum();
                    self.set(Act::Startle { away });
                }
            }
            _ => {}
        }
    }

    fn petted(&mut self) {
        match self.act {
            Act::Idle { kind: Idle::Sleep, .. } => {
                self.bubble = Some(("!", 0.9));
                self.set(Act::Startle { away: 0.0 });
            }
            Act::Hang { .. } | Act::Move { gait: Gait::Shimmy, .. } => {
                self.swing_v += self.facing * 7.0;
                self.bubble = Some(("Pika?", 1.0));
            }
            Act::Jump { .. } | Act::Fall | Act::Hop { .. } | Act::Trip => {}
            _ if self.ground.is_some() => {
                self.say();
                self.energy = (self.energy + 0.1).min(1.0);
                for i in 0..3 {
                    let hp = self.head_pos() + vec2((i as f32 - 1.0) * 8.0, -12.0);
                    let vx = self.range(-15.0, 15.0);
                    let vy = -self.range(25.0, 45.0);
                    self.emit(PartKind::Heart, hp, vec2(vx, vy), 1.1);
                }
                let n = if self.chance(0.5) { 1 } else { 2 };
                self.set(Act::Hop { n });
            }
            _ => {}
        }
    }

    fn timers(&mut self, dt: f32) {
        self.blink_in -= dt;
        if self.blink_in <= 0.0 {
            self.blink_t = 0.11;
            self.blink_in = if self.chance(0.2) { 0.25 } else { self.range(1.8, 5.5) };
        }
        self.blink_t = (self.blink_t - dt).max(0.0);
        self.twitch_in -= dt;
        if self.twitch_in <= 0.0 {
            self.twitch_t = 0.2;
            self.twitch_in = self.range(2.5, 9.0);
        }
        self.twitch_t = (self.twitch_t - dt).max(0.0);
        self.turn_t = (self.turn_t - dt).max(0.0);
        self.face_cd = (self.face_cd - dt).max(0.0);
        self.kick_t = (self.kick_t - dt).max(0.0);
        self.energy = self.energy.clamp(0.0, 1.0);
        if let Some((_, life)) = &mut self.bubble {
            *life -= dt;
            if *life <= 0.0 {
                self.bubble = None;
            }
        }
        if let Some((_, ttl)) = &mut self.goal {
            *ttl -= dt;
            if *ttl <= 0.0 {
                self.goal = None;
            }
        }
    }

    fn emit(&mut self, kind: PartKind, pos: Pos2, vel: Vec2, life: f32) {
        if self.parts.len() < 64 {
            self.parts.push(Particle { pos, vel, life, max: life, kind });
        }
    }

    fn update_particles(&mut self, dt: f32) {
        for p in &mut self.parts {
            p.pos += p.vel * dt;
            match p.kind {
                PartKind::Dust => p.vel *= (1.0 - 4.0 * dt).max(0.0),
                PartKind::Spark => p.vel *= (1.0 - 6.0 * dt).max(0.0),
                PartKind::Z | PartKind::Note => p.pos.x += (p.life * 5.0).sin() * 10.0 * dt,
                PartKind::Heart | PartKind::Star => p.vel.y += 10.0 * dt,
                PartKind::Ping => {}
            }
            p.life -= dt;
        }
        self.parts.retain(|p| p.life > 0.0);
    }

    // --- geometry ------------------------------------------------------------

    fn hanging_pose(&self) -> bool {
        matches!(self.act, Act::Hang { .. } | Act::Move { gait: Gait::Shimmy, .. } | Act::Jump { from_hang: true, .. })
    }

    fn body_rect(&self) -> Rect {
        let s = S_PT;
        if self.hanging_pose() {
            Rect::from_min_max(pos2(self.pos.x - 11.0 * s, self.pos.y), pos2(self.pos.x + 11.0 * s, self.pos.y + HANG_Y * s))
        } else {
            Rect::from_min_max(pos2(self.pos.x - 11.0 * s, self.pos.y - 22.0 * s), pos2(self.pos.x + 11.0 * s, self.pos.y))
        }
    }

    fn head_pos(&self) -> Pos2 {
        if self.hanging_pose() {
            self.pos + vec2(0.0, (HANG_Y - 13.0) * S_PT)
        } else {
            self.pos + vec2(0.0, -13.0 * S_PT)
        }
    }

    fn cheek_pos(&self) -> Pos2 {
        self.head_pos() + vec2(self.facing * 7.0 * S_PT, 2.0 * S_PT)
    }

    // --- pose --------------------------------------------------------------

    /// The pose for the current state and how far (points) it is lifted off
    /// the anchor (negative = drawn lower).
    fn pose(&self) -> (Pose, f32) {
        let mut p = Pose::default();
        let t = self.t;
        let mut lift = 0.0f32;
        let beat = |rate: f32| (t * rate) as i32;
        // Idle life: a slow breath (the body grows a pixel) and a lazy tail.
        if (self.clock * 2.1).sin() > 0.7 {
            p.body = 1;
        }
        p.tail = ((self.clock * 1.7).sin() > 0.4) as i32;
        match self.act {
            Act::Idle { kind, dur } => {
                let u = (t / dur).clamp(0.0, 1.0);
                match kind {
                    Idle::Sit => {}
                    Idle::Look => p.look = 1,
                    Idle::Wag => {
                        p.tail = [0, 1, 2, 1][(beat(12.0) & 3) as usize];
                        p.mouth = Mouth::Smile;
                        if beat(1.5) % 2 == 1 {
                            p.eyes = Eyes::Happy;
                        }
                    }
                    Idle::Scratch => {
                        // rubs an ear with a paw
                        p.arms.1 = Arm::Up;
                        p.ears.1 = if beat(14.0) % 2 == 0 { Ear::Flick } else { Ear::Up };
                        p.hdx = 1;
                        p.eyes = Eyes::Happy;
                    }
                    Idle::Sniff => {
                        p.hdy = 1;
                        p.body = -1;
                        p.eyes = Eyes::Half;
                        p.look = 1;
                        p.nose = beat(14.0) % 2;
                    }
                    Idle::Stretch => {
                        let k = smooth((u * 3.0).min(1.0)) * smooth(((1.0 - u) * 3.0).min(1.0));
                        if k > 0.3 {
                            p.arms = (Arm::Up, Arm::Up);
                            p.body = if k > 0.7 { 2 } else { 1 };
                            p.eyes = Eyes::Closed;
                        }
                        if k > 0.6 {
                            p.mouth = Mouth::Yawn;
                        }
                    }
                    Idle::Wave => {
                        p.arms.0 = if beat(7.0) % 2 == 0 { Arm::Up } else { Arm::Out };
                        p.eyes = Eyes::Happy;
                        p.mouth = Mouth::Open;
                    }
                    Idle::Spark => {
                        p.eyes = Eyes::Squint;
                        p.mouth = Mouth::Smile;
                        p.body = -1;
                        p.hdx = beat(30.0) % 2;
                        p.cheek = if (self.clock * 16.0) as i32 % 2 == 0 { H } else { R };
                        p.tail = beat(20.0) % 2;
                    }
                    Idle::Yawn => {
                        let k = (u * PI).sin();
                        p.eyes = Eyes::Closed;
                        if k > 0.35 {
                            p.mouth = Mouth::Yawn;
                            p.arms = (Arm::Out, Arm::Out);
                        }
                        if k > 0.7 {
                            p.body = 1;
                            p.ears = (Ear::Flick, Ear::Flick);
                        }
                    }
                    Idle::Sleep => {
                        p.eyes = Eyes::Closed;
                        p.ears = (Ear::Down, Ear::Down);
                        p.body = -1;
                        // a slow doze-and-nod
                        p.hdy = 1 + ((self.clock * 0.9).sin() > 0.6) as i32;
                        p.tail = 0;
                    }
                    Idle::Dance => {
                        let b = t * 2.6;
                        let ph = b.fract();
                        lift = 7.0 * 4.0 * ph * (1.0 - ph);
                        let even = b as i32 % 2 == 0;
                        p.arms = if even { (Arm::Up, Arm::Out) } else { (Arm::Out, Arm::Up) };
                        p.hdx = if even { -1 } else { 1 };
                        p.eyes = Eyes::Happy;
                        p.mouth = Mouth::Open;
                        p.body = if !(0.12..=0.9).contains(&ph) { -1 } else { 1 };
                        p.tail = if even { 0 } else { 2 };
                    }
                    Idle::Curious => {
                        p.hdx = 1;
                        p.ears.0 = Ear::Flick;
                        p.eyes = Eyes::Wide;
                        p.look = 1;
                    }
                }
            }
            Act::Move { gait, .. } => {
                let step = (self.phase * 4.0) as i32 & 3;
                match gait {
                    Gait::Walk => {
                        // chibi toddle: alternate feet, a pixel of bounce
                        p.look = 1;
                        match step {
                            0 => {
                                p.feet.0 = -2;
                                p.bob = -1;
                            }
                            2 => {
                                p.feet.1 = -2;
                                p.bob = -1;
                            }
                            _ => {}
                        }
                        p.tail = step & 1;
                    }
                    Gait::Run => {
                        // leaning dash, ears pinned back, arms out
                        p.look = 1;
                        p.lean = 2;
                        p.ears = (Ear::Down, Ear::Down);
                        p.arms = (Arm::Out, Arm::Out);
                        match step {
                            0 => {
                                p.feet.0 = -3;
                                p.bob = -1;
                            }
                            1 => {
                                p.feet = (-1, -1);
                                p.bob = -2;
                            }
                            2 => {
                                p.feet.1 = -3;
                                p.bob = -1;
                            }
                            _ => {}
                        }
                        p.tail = 2 - (step & 1);
                        if self.run_happy {
                            p.mouth = Mouth::Open;
                        }
                    }
                    Gait::Roll => {
                        p.form = Form::Ball;
                        p.roll = (self.phase / (PI / 2.0)) as i32 & 3;
                    }
                    Gait::Shimmy => {
                        p.form = Form::Hang;
                        p.hands = (self.phase * TAU).sin();
                        p.swing = -self.facing * self.swing;
                        p.feet = (step & 1, 1 - (step & 1));
                        p.mouth = Mouth::Smile;
                    }
                }
            }
            Act::Crouch { target, .. } => {
                p.body = -2;
                p.look = 1;
                p.ears = (Ear::Flick, Ear::Flick);
                if target.kind == PerchKind::Hang {
                    p.eyes = Eyes::Wide;
                }
            }
            Act::Jump { target, dur, from_hang, .. } => {
                let u = (t / dur).min(1.0);
                let rising = u < 0.5;
                if from_hang {
                    p.form = Form::Hang;
                    // swings forward into the leap (sprite space faces left)
                    p.swing = 3.0 * (1.0 - 2.0 * u);
                    p.hands = 1.0;
                    p.mouth = Mouth::Open;
                    p.feet = (1, 1);
                } else if target.kind == PerchKind::Hang {
                    p.arms = (Arm::Reach, Arm::Reach);
                    p.body = 1;
                    p.eyes = Eyes::Wide;
                    p.mouth = Mouth::Open;
                    lift = -HANG_Y * S_PT * smooth(u);
                } else if target.pounce {
                    p.lean = 2;
                    p.body = 1;
                    p.look = 1;
                    p.arms = (Arm::Out, Arm::Out);
                    p.ears = (Ear::Down, Ear::Down);
                    p.mouth = Mouth::Open;
                    p.feet = if rising { (1, 1) } else { (-1, -1) };
                } else {
                    p.body = if (1.0 - 2.0 * u).abs() < 0.6 { 2 } else { 1 };
                    p.arms = (Arm::Up, Arm::Up);
                    p.feet = (1, 1);
                    if rising {
                        p.mouth = Mouth::Open;
                    }
                }
            }
            Act::Land { dur } => {
                let u = (t / dur).min(1.0);
                p.body = if u < 0.5 { -2 } else { -1 };
                if u < 0.6 {
                    p.ears = (Ear::Flick, Ear::Flick);
                }
            }
            Act::Fall => {
                let k = beat(14.0) % 2;
                p.arms = (Arm::Up, Arm::Up);
                p.eyes = Eyes::Wide;
                p.mouth = Mouth::Open;
                p.feet = (k, 1 - k);
                p.body = 1;
                p.ears = if k == 0 { (Ear::Up, Ear::Flick) } else { (Ear::Flick, Ear::Up) };
            }
            Act::Hang { .. } => {
                p.form = Form::Hang;
                p.swing = -self.facing * self.swing;
                p.mouth = Mouth::Smile;
                let k = if self.kick_t > 0.0 { beat(12.0) % 2 } else { ((self.clock * 1.2).sin() > 0.0) as i32 };
                p.feet = (k, 1 - k);
            }
            Act::Hop { .. } => {
                let k = (t / 0.34).fract();
                lift = 15.0 * 4.0 * k * (1.0 - k);
                p.body = if !(0.1..=0.92).contains(&k) { -1 } else { 1 };
                p.arms = (Arm::Up, Arm::Up);
                p.eyes = Eyes::Happy;
                p.mouth = Mouth::Open;
                if lift > 3.0 {
                    p.feet = (1, 1);
                }
            }
            Act::Startle { .. } => {
                let k = (t / 0.4).min(1.0);
                lift = 12.0 * 4.0 * k * (1.0 - k);
                p.eyes = Eyes::Wide;
                p.mouth = Mouth::Open;
                p.arms = (Arm::Out, Arm::Out);
                p.body = if k >= 1.0 { -1 } else { 1 };
                p.tail = 2;
            }
            Act::Trip => {
                if t < 0.35 {
                    p.form = Form::Ball;
                    p.roll = (self.phase / (PI / 2.0)) as i32 & 3;
                } else {
                    p.eyes = Eyes::Dizzy;
                    p.ears = (Ear::Down, Ear::Down);
                    p.body = -1;
                    p.mouth = Mouth::Open;
                    p.hdx = ((t * 6.0).sin() * 1.4).round() as i32;
                }
            }
        }
        if self.blink_t > 0.0 && p.eyes == Eyes::Open {
            p.eyes = Eyes::Closed;
        }
        if self.twitch_t > 0.0 && p.ears.1 == Ear::Up {
            p.ears.1 = Ear::Flick;
        }
        if self.turn_t > 0.0 {
            p.body = (p.body - 1).max(-2);
        }
        if p.form != Form::Hang {
            lift -= p.bob as f32 * S_PT;
        }
        (p, lift)
    }

    // --- drawing -----------------------------------------------------------

    fn draw(&self, ctx: &egui::Context, th: &Theme, perches: &[Perch]) {
        let ppp = ctx.pixels_per_point();
        let s = (S_PT * ppp).round().max(1.0) / ppp;
        let snap = |v: f32| (v * ppp).round() / ppp;
        let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("pikachu-layer")));
        let (pose, lift) = self.pose();
        let grid = rasterize(&pose);
        let hang = pose.form == Form::Hang;
        let origin = if hang { pos2(self.pos.x, self.pos.y + HANG_Y * s) } else { pos2(self.pos.x, self.pos.y - lift) };
        let origin = pos2(snap(origin.x), snap(origin.y));

        // Shadow on whatever is below.
        if !hang {
            let below = match self.ground {
                Some(_) => Some(self.pos.y),
                None => perches
                    .iter()
                    .filter(|p| p.kind != PerchKind::Hang && self.pos.x >= p.x0 && self.pos.x <= p.x1 && p.y >= self.pos.y - 1.0)
                    .map(|p| p.y)
                    .fold(None, |a: Option<f32>, y| Some(a.map_or(y, |a| a.min(y)))),
            };
            if let Some(y) = below {
                let h = (y - (self.pos.y - lift)).max(0.0);
                let k = 1.0 / (1.0 + h / 70.0);
                let w = (11.0 * k).round().max(3.0);
                let a = 0.2 * k;
                let c = Color32::from_black_alpha((a * 255.0) as u8);
                let y = snap(y);
                let x = snap(self.pos.x);
                painter.rect_filled(Rect::from_min_size(pos2(x - w * s / 2.0, y - s), vec2(w * s, s)), 0.0, c);
                if w > 5.0 {
                    let w2 = w - 4.0;
                    painter.rect_filled(Rect::from_min_size(pos2(x - w2 * s / 2.0, y), vec2(w2 * s, s * 0.5)), 0.0, c);
                }
            }
        }

        paint_grid(&painter, &grid, origin, s, self.facing);

        for part in &self.parts {
            let a = (part.life / part.max).clamp(0.0, 1.0);
            let a = if part.kind == PartKind::Dust { a * 0.8 } else { (a * 3.0).min(1.0) };
            let at = pos2(snap(part.pos.x), snap(part.pos.y));
            let (bmp, outline): (&[&str], bool) = match part.kind {
                PartKind::Dust => (if part.life > part.max * 0.5 { &["DD", "DD"] } else { &["D"] }, false),
                PartKind::Spark => (&["..Y", ".Y.", "YYY", ".Y.", "Y.."], true),
                PartKind::Z => (&["WWW", "..W", ".W.", "W..", "WWW"], true),
                PartKind::Heart => (&[".R.R.", "RRRRR", ".RRR.", "..R.."], true),
                PartKind::Note => (&["..KK", "..K.", "..K.", "KKK.", "KK.."], false),
                PartKind::Star => (&[".Y.", "YYY", ".Y."], true),
                PartKind::Ping => (&["..RRR..", ".R...R.", "R.....R", "R.....R", "R.....R", ".R...R.", "..RRR.."], false),
            };
            let ps = match part.kind {
                PartKind::Dust => s,
                PartKind::Ping => s * (0.5 + 1.2 * (1.0 - part.life / part.max)),
                _ => s * 0.75,
            };
            let a = if part.kind == PartKind::Ping { (part.life / part.max).clamp(0.0, 1.0) } else { a };
            paint_bitmap(&painter, bmp, at, ps, a, outline, th.dark);
        }

        // A little pokéball marks the right-click goal while Pikachu heads there.
        if let Some((g, ttl)) = self.goal {
            let a = (ttl / 0.6).clamp(0.0, 1.0);
            let bob = (self.clock * 4.0).sin() * 1.5;
            let ball = [".KKK.", "KRRRK", "KKWKK", "KWWWK", ".KKK."];
            paint_bitmap(&painter, &ball, pos2(snap(g.x), snap(g.y - 6.0 + bob)), s, a, false, false);
        }

        if let Some((txt, life)) = self.bubble {
            let ink = hex(0x3a2414);
            let galley = painter.layout_no_wrap(txt.to_string(), th.font(Role::SansB, 11.5), ink);
            let size = galley.size() + vec2(14.0, 7.0);
            let rise = ((1.5 - life).max(0.0) * 3.0).min(4.0);
            let anchor = if hang {
                self.pos + vec2(self.facing * 14.0 * s, 4.0 * s)
            } else {
                pos2(self.pos.x + self.facing * 6.0 * s, self.pos.y - lift - 27.0 * s)
            };
            let screen = ctx.screen_rect();
            let mut r = Rect::from_center_size(pos2(anchor.x + self.facing * size.x * 0.3, anchor.y - size.y / 2.0 - 6.0 - rise), size);
            let shift_x = (screen.left() + 4.0 - r.left()).max(0.0) + (screen.right() - 4.0 - r.right()).min(0.0);
            let shift_y = (screen.top() + 4.0 - r.top()).max(0.0);
            r = r.translate(vec2(shift_x, shift_y));
            let r = Rect::from_min_max(pos2(snap(r.left()), snap(r.top())), pos2(snap(r.right()), snap(r.bottom())));
            let fade = (life / 0.3).clamp(0.0, 1.0);
            let white = Color32::from_white_alpha((255.0 * fade) as u8);
            let line = crate::theme::rgba(ink, fade);
            let tx = anchor.x.clamp(r.left() + 8.0, r.right() - 8.0);
            let tail = vec![pos2(tx - 4.0, r.bottom() - 1.0), pos2(tx + 4.0, r.bottom() - 1.0), pos2(anchor.x, r.bottom() + 6.0)];
            painter.add(Shape::convex_polygon(tail, white, Stroke::new(1.5_f32, line)));
            painter.rect_filled(r, 7.0, white);
            painter.rect_stroke(r, 7.0, Stroke::new(1.5_f32, line), StrokeKind::Inside);
            // re-cover the seam between the tail and the bubble
            painter.add(Shape::convex_polygon(
                vec![pos2(tx - 2.8, r.bottom() - 2.0), pos2(tx + 2.8, r.bottom() - 2.0), pos2(anchor.x + (tx - anchor.x) * 0.5, r.bottom() + 2.5)],
                white,
                Stroke::NONE,
            ));
            painter.text(r.center(), Align2::CENTER_CENTER, txt, th.font(Role::SansB, 11.5), line);
        }
    }
}

// ---------------------------------------------------------------------------
// Pixel art
//
// Pikachu is drawn front-on, the way the classic pixel-art pieces show it: a
// big wide head, long V-shaped ears with black tips, round red cheeks, glinting
// eyes, a stubby body and a zigzag tail behind. Every frame is composed from
// hand-drawn layers (tail, body, feet, head, ears, face, arms) so the same
// art serves every animation: the body comes in five heights for squash and
// stretch, the ears in three states, the arms in four, and the whole upper
// body can lean into a run or swing while hanging.
//
// Sprite space: 23 columns wide with column 11 as the centre line, rows 0 (ear
// tips) to 23 (soles). The art faces the viewer with the tail on the right,
// so it reads as heading left; it is mirrored when heading right.
// ---------------------------------------------------------------------------

/// Sprite rows from the hands to the soles while hanging.
const HANG_Y: f32 = 27.0;

// palette indices
const K: u8 = 1; // outline / ear tips
const Y: u8 = 2; // yellow
const S: u8 = 3; // yellow shade
const R: u8 = 4; // cheek
const B: u8 = 5; // brown tail base
const W: u8 = 7; // white
const M: u8 = 8; // mouth
const H: u8 = 10; // highlight

fn color(c: u8) -> Color32 {
    match c {
        K => hex(0x2a1c12),
        Y => hex(0xf8d030),
        S => hex(0xd9a520),
        R => hex(0xe8402c),
        B => hex(0x8f5418),
        W => Color32::WHITE,
        M => hex(0xa8282c),
        H => hex(0xfff3a6),
        _ => Color32::TRANSPARENT,
    }
}

const HEAD: [&str; 10] = [
    "....KKKKKKKKKKKKKKK....",
    "...KHHYYYYYYYYYYYYYK...",
    "..KHYYYYYYYYYYYYYYYYK..",
    ".KYYYYYYYYYYYYYYYYYYYK.",
    ".KYYYYYYYYYYYYYYYYYYYK.",
    ".KYYYYYYYYYYYYYYYYYYYK.",
    ".KYYYYYYYYYYYYYYYYYYYK.",
    ".KYYYYYYYYYYYYYYYYYYYK.",
    "..KYYYYYYYYYYYYYYYYYK..",
    "...KKYYYYYYYYYYYYYKK...",
];

/// Body rows, chin to seat. Row 2 is repeated or dropped for stretch/squash.
const BODY: [&str; 6] = [
    "....KYYYYYYYYYYYYYK....",
    "...KYYYYYYYYYYYYYYYK...",
    "...KYYYYYYYYYYYYYYYK...",
    "...KYYYYYYYYYYYYYYYK...",
    "...KSYYYYYYYYYYYYYSK...",
    "....KSSSSSSSSSSSSSK....",
];

// Left ear (the right one is mirrored), placed at the sprite's top-left.
/// Long ears: a black tip a third of the way down. Placed two rows above the
/// sprite top so their bases land on the crown (row 6).
const EAR_UP: [&str; 9] = [
    "KK......",
    "KKK.....",
    ".KKK....",
    ".KKKK...",
    "..KKKK..",
    "..KYYYK.",
    "..KYYYK.",
    "...KYYYK",
    "...KYYYK",
];
/// One ear flicked: the tip bends outward and the ear dips a pixel.
const EAR_FLICK: [&str; 9] = [
    "........",
    "KKK.....",
    "KKKK....",
    ".KKKK...",
    "..KKKK..",
    "..KYYYK.",
    "..KYYYK.",
    "...KYYYK",
    "...KYYYK",
];
/// Ears laid back and out (sleepy, sad, running); placed three columns left, three rows down.
const EAR_DOWN: [&str; 4] = ["KKK........", "KKKKKK.....", ".KKKKYYYKK.", "....KYYYYYK"];

// Left arm (mirrored for the right).
const ARM_REST: [&str; 3] = ["KKK.", "YYYK", "KKK."];
const ARM_OUT: [&str; 3] = ["KKKK.", "KYYYY", ".KKKK"];
const ARM_UP: [&str; 7] = ["KK....", "KYK...", "KYYK..", ".KYYK.", "..KYYK", "...KYY", "....KY"];

const FOOT: [&str; 3] = ["KYYYK", "KYYYK", ".KKK."];

/// Zigzag tail, brown at the base, tucked behind the body's right side.
const TAIL: [&str; 13] = [
    "......KKKK",
    ".....KYYYK",
    "....KYYYYK",
    "...KYYYYK.",
    "...KKKYYK.",
    ".....KYK..",
    "....KYYK..",
    "...KYYK...",
    "...KKYK...",
    "....KBK...",
    "...KBBK...",
    "..KBBK....",
    ".KBK......",
];

/// Curled up for rolling: ear tips at the top-left, a cheek peeking out.
const BALL: [&str; 14] = [
    "..KK.KKKK.....",
    "..KKKYYYYKK...",
    "...KYYYYYYYK..",
    "..KYYYYBBYYYK.",
    ".KYYYYYYYBBYK.",
    "KYYYYYYYYYYYYK",
    "KYYYYYYYYYYYYK",
    "KYRRYYYYYYYYYK",
    "KYRRYYYYYYYYYK",
    "KYYYYYYYYYYYSK",
    ".KYYYYYYYYYSK.",
    ".KSYYYYYYYSSK.",
    "..KKSSSSSSKK..",
    "....KKKKKK....",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Form {
    Front,
    Ball,
    Hang,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Eyes {
    Open,
    Closed,
    Happy,
    Wide,
    Half,
    Dizzy,
    Squint,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mouth {
    None,
    Smile,
    Open,
    Yawn,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Ear {
    Up,
    Flick,
    Down,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Arm {
    Rest,
    Out,
    Up,
    /// Straight up over the head (hanging, or reaching for a tab).
    Reach,
}

#[derive(Clone, Copy, Debug)]
struct Pose {
    form: Form,
    /// Body height change in pixels: -2 squash .. +2 stretch.
    body: i32,
    hdx: i32,
    hdy: i32,
    /// Whole-sprite vertical step, pixels (negative = up).
    bob: i32,
    /// Upper body lean toward the heading, pixels at the ear tips.
    lean: i32,
    /// Face turned toward the heading by this many pixels.
    look: i32,
    nose: i32,
    tail: i32,
    ears: (Ear, Ear),
    arms: (Arm, Arm),
    /// Foot offsets, pixels (negative = lifted).
    feet: (i32, i32),
    eyes: Eyes,
    mouth: Mouth,
    cheek: u8,
    roll: i32,
    /// Sideways swing while hanging, pixels at the soles (sprite space).
    swing: f32,
    hands: f32,
}

impl Default for Pose {
    fn default() -> Self {
        Pose {
            form: Form::Front,
            body: 0,
            hdx: 0,
            hdy: 0,
            bob: 0,
            lean: 0,
            look: 0,
            nose: 0,
            tail: 0,
            ears: (Ear::Up, Ear::Up),
            arms: (Arm::Rest, Arm::Rest),
            feet: (0, 0),
            eyes: Eyes::Open,
            mouth: Mouth::None,
            cheek: R,
            roll: 0,
            swing: 0.0,
            hands: 0.0,
        }
    }
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

const GW: i32 = 48;
const GH: i32 = 36;
/// Grid index of sprite column 0 / row 0.
const CO: i32 = 12;
const RO: i32 = 8;

struct Grid(Vec<u8>);

impl Grid {
    fn new() -> Self {
        Grid(vec![0; (GW * GH) as usize])
    }

    fn idx(c: i32, r: i32) -> Option<usize> {
        let (i, j) = (c + CO, r + RO);
        (i >= 0 && j >= 0 && i < GW && j < GH).then(|| (j * GW + i) as usize)
    }

    fn get(&self, c: i32, r: i32) -> u8 {
        Self::idx(c, r).map_or(0, |k| self.0[k])
    }

    fn set(&mut self, c: i32, r: i32, v: u8) {
        if let Some(k) = Self::idx(c, r) {
            self.0[k] = v;
        }
    }

    /// Paint only on fur (face details).
    fn paint(&mut self, c: i32, r: i32, v: u8) {
        if matches!(self.get(c, r), Y | S | H) {
            self.set(c, r, v);
        }
    }

    /// Stamp a bitmap with its top-left at (`ox`, `oy`); `mirror` reflects it
    /// across the centre line (for right-hand ears, arms and feet).
    fn st(&mut self, rows: &[&str], ox: i32, oy: i32, mirror: bool) {
        self.st_with(rows, ox, oy, mirror, |_| 0);
    }

    fn st_with(&mut self, rows: &[&str], ox: i32, oy: i32, mirror: bool, row_dx: impl Fn(i32) -> i32) {
        for (j, row) in rows.iter().enumerate() {
            let dx = row_dx(j as i32);
            for (i, ch) in row.bytes().enumerate() {
                let v = match ch {
                    b'K' => K,
                    b'Y' => Y,
                    b'S' => S,
                    b'H' => H,
                    b'R' => R,
                    b'B' => B,
                    b'W' => W,
                    b'M' => M,
                    _ => continue,
                };
                let c = ox + i as i32 + dx;
                self.set(if mirror { 22 - c } else { c }, oy + j as i32, v);
            }
        }
    }

    /// A two-pixel-wide outlined limb through `pts` (hanging / reaching arms).
    fn limb(&mut self, pts: &[(f32, f32)]) {
        let mut core: Vec<(i32, i32)> = Vec::new();
        for w in pts.windows(2) {
            let (a, b) = (w[0], w[1]);
            let n = (b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil().max(1.0) as i32;
            for k in 0..=n {
                let t = k as f32 / n as f32;
                let c = (a.0 + (b.0 - a.0) * t).round() as i32;
                let r = (a.1 + (b.1 - a.1) * t).round() as i32;
                for cell in [(c, r), (c + 1, r)] {
                    if !core.contains(&cell) {
                        core.push(cell);
                    }
                }
            }
        }
        for &(c, r) in &core {
            for (dc, dr) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                if !core.contains(&(c + dc, r + dr)) {
                    self.set(c + dc, r + dr, K);
                }
            }
        }
        for &(c, r) in &core {
            self.set(c, r, Y);
        }
    }

    /// Shift each row sideways by `f(row)` pixels (lean, swing, roll offsets).
    fn shift_rows(&mut self, f: impl Fn(i32) -> i32) {
        for j in 0..GH {
            let s = f(j - RO);
            if s == 0 {
                continue;
            }
            let start = (j * GW) as usize;
            let old: Vec<u8> = self.0[start..start + GW as usize].to_vec();
            for i in 0..GW {
                let src = i - s;
                self.0[start + i as usize] = if (0..GW).contains(&src) { old[src as usize] } else { 0 };
            }
        }
    }

    /// Close any gap where fur meets empty space (layers are already outlined).
    fn outline(&mut self) {
        let src = self.0.clone();
        let at = |i: i32, j: i32| -> u8 {
            if i < 0 || j < 0 || i >= GW || j >= GH {
                0
            } else {
                src[(j * GW + i) as usize]
            }
        };
        for j in 0..GH {
            for i in 0..GW {
                if at(i, j) != 0 {
                    continue;
                }
                let fur = [(-1, 0), (1, 0), (0, -1), (0, 1)].iter().any(|&(di, dj)| {
                    let v = at(i + di, j + dj);
                    v != 0 && v != K
                });
                if fur {
                    self.0[(j * GW + i) as usize] = K;
                }
            }
        }
    }
}

fn rot90(rows: &[&str], turns: i32) -> Vec<String> {
    let mut cur: Vec<Vec<u8>> = rows.iter().map(|r| r.bytes().collect()).collect();
    for _ in 0..(turns & 3) {
        let h = cur.len();
        let w = cur[0].len();
        let mut next = vec![vec![b'.'; h]; w];
        for (r, row) in cur.iter().enumerate() {
            for (c, &ch) in row.iter().enumerate() {
                next[c][h - 1 - r] = ch;
            }
        }
        cur = next;
    }
    cur.into_iter().map(|r| String::from_utf8(r).unwrap_or_default()).collect()
}

fn ear_art(e: Ear) -> (&'static [&'static str], i32, i32) {
    match e {
        Ear::Up => (&EAR_UP, 0, -2),
        Ear::Flick => (&EAR_FLICK, 0, -2),
        Ear::Down => (&EAR_DOWN, -3, 3),
    }
}

fn face(g: &mut Grid, p: &Pose, hx: i32, hy: i32) {
    let fx = hx - p.look;
    let eyes = [(6 + fx, false), (15 + fx, true)];
    for (ex, right) in eyes {
        let ey = 9 + hy;
        // `o` is the outer column of a 3-wide eye (toward the cheek)
        let o = if right { ex + 1 } else { ex - 1 };
        match p.eyes {
            Eyes::Open => {
                for r in 0..3 {
                    g.paint(ex, ey + r, K);
                    g.paint(ex + 1, ey + r, K);
                }
                g.set(ex, ey, W);
            }
            Eyes::Closed => {
                g.paint(ex, ey + 2, K);
                g.paint(ex + 1, ey + 2, K);
                // a gentle curve: the outer corner lifts a pixel
                g.paint(if right { o + 1 } else { o }, ey + 1, K);
            }
            Eyes::Happy => {
                g.paint(ex - 1, ey + 2, K);
                g.paint(ex, ey + 1, K);
                g.paint(ex + 1, ey + 1, K);
                g.paint(ex + 2, ey + 2, K);
            }
            Eyes::Wide => {
                for r in 0..3 {
                    for c in -1..=1 {
                        g.paint(ex + c + right as i32, ey + r, K);
                    }
                }
                g.set(ex - 1 + right as i32, ey, W);
            }
            Eyes::Half => {
                for r in 1..3 {
                    g.paint(ex, ey + r, K);
                    g.paint(ex + 1, ey + r, K);
                }
            }
            Eyes::Dizzy => {
                let c0 = ex - 1 + right as i32;
                for (dc, dr) in [(0, 0), (2, 0), (1, 1), (0, 2), (2, 2)] {
                    g.paint(c0 + dc, ey + dr, K);
                }
            }
            Eyes::Squint => {
                // > <
                let (a, b) = if right { (ex + 1, ex) } else { (ex, ex + 1) };
                g.paint(a, ey, K);
                g.paint(b, ey + 1, K);
                g.paint(a, ey + 2, K);
            }
        }
    }
    for cx in [3 + fx, 17 + fx] {
        for r in 0..2 {
            for c in 0..3 {
                g.paint(cx + c, 12 + hy + r, p.cheek);
            }
        }
    }
    let nx = 11 + fx + p.nose;
    g.paint(nx, 11 + hy, K);
    let (mx, my) = (11 + fx, 12 + hy);
    match p.mouth {
        Mouth::None => {}
        Mouth::Smile => {
            // the little ω under the nose
            for (dc, dr) in [(-2, 0), (-1, 1), (0, 0), (1, 1), (2, 0)] {
                g.paint(mx + dc, my + dr, K);
            }
        }
        Mouth::Open => {
            g.paint(mx - 1, my, K);
            g.paint(mx, my, M);
            g.paint(mx + 1, my, K);
            for c in -1..=1 {
                g.paint(mx + c, my + 1, M);
            }
            g.paint(mx, my + 2, K);
        }
        Mouth::Yawn => {
            for r in 0..3 {
                for c in -1..=1 {
                    g.paint(mx + c, my + r, M);
                }
            }
            g.paint(mx - 1, my, K);
            g.paint(mx + 1, my, K);
        }
    }
}

fn front(g: &mut Grid, p: &Pose, hang: bool) {
    let mut body: Vec<&str> = BODY.to_vec();
    match p.body {
        d if d <= -2 => {
            body.remove(2);
            body.remove(2);
        }
        -1 => {
            body.remove(2);
        }
        1 => body.insert(2, BODY[2]),
        d if d >= 2 => {
            body.insert(2, BODY[2]);
            body.insert(2, BODY[2]);
        }
        _ => {}
    }
    let top = 22 - body.len() as i32;
    let dy = top - 16;
    let (hx, hy) = (p.hdx, dy + p.hdy);

    // tail first: everything else sits in front of it
    let wag = p.tail;
    g.st_with(&TAIL, 17, 8 + dy, false, |r| if r < 9 { wag } else { 0 });

    for (k, row) in body.iter().enumerate() {
        g.st(&[row], 0, top + k as i32, false);
    }
    g.st(&FOOT, 3, 21 + p.feet.0, false);
    g.st(&FOOT, 3, 21 + p.feet.1, true);

    g.st(&HEAD, hx, 6 + hy, false);
    let (art, ox, oy) = ear_art(p.ears.0);
    g.st(art, ox + hx, oy + hy, false);
    let (art, ox, oy) = ear_art(p.ears.1);
    g.st(art, ox - hx, oy + hy, true);
    face(g, p, hx, hy);

    if hang {
        // arms straight up past the head; hands at the very top
        let lift_l = p.hands.max(0.0);
        let lift_r = (-p.hands).max(0.0);
        g.limb(&[(3.0, top as f32 + 1.0), (-1.0, 11.0), (-0.5 - p.hands, -4.0 + lift_l)]);
        g.limb(&[(18.0, top as f32 + 1.0), (22.0, 11.0), (21.5 - p.hands, -4.0 + lift_r)]);
    } else {
        for (arm, mirror) in [(p.arms.0, false), (p.arms.1, true)] {
            match arm {
                Arm::Rest => g.st(&ARM_REST, 5, top + 1, mirror),
                Arm::Out => g.st(&ARM_OUT, 0, top + 1, mirror),
                Arm::Up => g.st(&ARM_UP, -2, top - 5, mirror),
                Arm::Reach => {
                    let pts = [(3.0, top as f32 + 1.0), (-1.0, 11.0), (-0.5, -4.0)];
                    let pts: Vec<(f32, f32)> = if mirror { pts.iter().map(|&(c, r)| (21.0 - c, r)).collect() } else { pts.to_vec() };
                    g.limb(&pts);
                }
            }
        }
    }

    // Lean and swing move whole bands (ears + head together, then the body,
    // then the feet) so the face never shears apart.
    let head_end = 16 + hy;
    if p.lean != 0 {
        // lean toward the heading (left, in sprite space)
        let lean = p.lean;
        g.shift_rows(|r| if r < head_end { -lean } else if r < top + 3 { -(lean / 2) } else { 0 });
    }
    if hang && p.swing.abs() > 0.2 {
        let sw = p.swing;
        let band = |k: f32| (sw * k).round() as i32;
        g.shift_rows(|r| if r < 4 { 0 } else if r < head_end { band(0.4) } else if r < 21 { band(0.7) } else { band(1.0) });
    }
}

fn rasterize(p: &Pose) -> Grid {
    let mut g = Grid::new();
    match p.form {
        Form::Front => front(&mut g, p, false),
        Form::Hang => front(&mut g, p, true),
        Form::Ball => {
            let art = rot90(&BALL, p.roll);
            let rows: Vec<&str> = art.iter().map(|s| s.as_str()).collect();
            g.st(&rows, 4, 10, false);
        }
    }
    g.outline();
    g
}

/// Paint the grid with sprite column 11 centred on `origin.x` and the soles
/// on `origin.y`; mirrored (so it heads right) when `facing` is positive.
/// Horizontal runs of one colour become a single rectangle.
fn paint_grid(p: &Painter, g: &Grid, origin: Pos2, s: f32, facing: f32) {
    let mirror = facing > 0.0;
    let x0 = origin.x - 11.5 * s;
    let mut row: Vec<(i32, u8)> = Vec::with_capacity(GW as usize);
    for j in 0..GH {
        row.clear();
        for i in 0..GW {
            let v = g.0[(j * GW + i) as usize];
            if v != 0 {
                let c = i - CO;
                row.push((if mirror { 22 - c } else { c }, v));
            }
        }
        row.sort_by_key(|e| e.0);
        let top = origin.y + (j - RO - 24) as f32 * s;
        let mut k = 0;
        while k < row.len() {
            let (c0, v) = row[k];
            let mut c1 = c0;
            while k + 1 < row.len() && row[k + 1].1 == v && row[k + 1].0 == c1 + 1 {
                k += 1;
                c1 += 1;
            }
            p.rect_filled(
                Rect::from_min_max(pos2(x0 + c0 as f32 * s, top), pos2(x0 + (c1 + 1) as f32 * s, top + s)),
                0.0,
                color(v),
            );
            k += 1;
        }
    }
}

/// Tiny particle bitmaps: Y yellow, W white, R red, K ink, D dust.
fn paint_bitmap(p: &Painter, rows: &[&str], center: Pos2, s: f32, alpha: f32, outline: bool, dark: bool) {
    let h = rows.len() as f32;
    let w = rows.iter().map(|r| r.len()).max().unwrap_or(0) as f32;
    let o = center - vec2(w * s / 2.0, h * s / 2.0);
    let col = |ch: u8| -> Option<Color32> {
        let c = match ch {
            b'Y' => color(Y),
            b'W' => Color32::WHITE,
            b'R' => color(R),
            b'K' => if dark { hex(0xf4ead0) } else { color(K) },
            b'D' => if dark { Color32::from_white_alpha(150) } else { hex(0xd8c7a0) },
            _ => return None,
        };
        Some(crate::theme::rgba(c, alpha * (c.a() as f32 / 255.0)))
    };
    let cells = || {
        rows.iter().enumerate().flat_map(|(j, r)| r.bytes().enumerate().map(move |(i, ch)| (i as f32, j as f32, ch)))
    };
    if outline {
        let k = crate::theme::rgba(color(K), alpha);
        for (i, j, ch) in cells() {
            if col(ch).is_some() {
                for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                    p.rect_filled(Rect::from_min_size(o + vec2((i + dx) * s, (j + dy) * s), vec2(s, s)), 0.0, k);
                }
            }
        }
    }
    for (i, j, ch) in cells() {
        if let Some(c) = col(ch) {
            p.rect_filled(Rect::from_min_size(o + vec2(i * s, j * s), vec2(s, s)), 0.0, c);
        }
    }
}

// ---------------------------------------------------------------------------
// Developer sprite sheet (cargo feature "screenshot", HAEM_SCENE=pika=sheet)
// ---------------------------------------------------------------------------

#[cfg(feature = "screenshot")]
impl Pikachu {
    /// Simulate a right-click at `at` (HAEM_SCENE=goal=X:Y).
    pub fn debug_goal(&mut self, at: Pos2) {
        self.set_goal(at);
    }
}

#[cfg(feature = "screenshot")]
pub fn debug_sheet(ctx: &egui::Context) {
    let mut k = Pikachu::new();
    let tg = |kind: PerchKind, pounce: bool| Target { id: Id::new("sheet"), dx: 0.0, kind, pounce };
    let idle = |kind: Idle, dur: f32| Act::Idle { kind, dur };
    let cases: Vec<(&str, Act, f32, f32)> = vec![
        ("sit", idle(Idle::Sit, 3.0), 0.0, 0.0),
        ("wag", idle(Idle::Wag, 3.0), 0.12, 0.0),
        ("scratch", idle(Idle::Scratch, 3.0), 0.3, 0.0),
        ("wave", idle(Idle::Wave, 3.0), 0.2, 0.0),
        ("spark", idle(Idle::Spark, 3.0), 0.2, 0.0),
        ("yawn", idle(Idle::Yawn, 1.4), 0.7, 0.0),
        ("curious", idle(Idle::Curious, 2.0), 0.3, 0.0),
        ("dance", idle(Idle::Dance, 3.0), 0.1, 0.0),
        ("sniff", idle(Idle::Sniff, 2.0), 0.1, 0.0),
        ("stretch", idle(Idle::Stretch, 1.4), 0.7, 0.0),
        ("sleep", idle(Idle::Sleep, 9.0), 1.0, 0.0),
        ("walk 0", Act::Move { to: 0.0, gait: Gait::Walk }, 0.0, 0.0),
        ("walk .25", Act::Move { to: 0.0, gait: Gait::Walk }, 0.0, 0.25),
        ("walk .5", Act::Move { to: 0.0, gait: Gait::Walk }, 0.0, 0.5),
        ("run 0", Act::Move { to: 0.0, gait: Gait::Run }, 0.0, 0.0),
        ("run .25", Act::Move { to: 0.0, gait: Gait::Run }, 0.0, 0.25),
        ("run .5", Act::Move { to: 0.0, gait: Gait::Run }, 0.0, 0.5),
        ("run .75", Act::Move { to: 0.0, gait: Gait::Run }, 0.0, 0.75),
        ("roll", Act::Move { to: 0.0, gait: Gait::Roll }, 0.0, 1.3),
        ("crouch", Act::Crouch { dur: 0.2, target: tg(PerchKind::Top, true) }, 0.1, 0.0),
        ("pounce", Act::Jump { target: tg(PerchKind::Top, true), from: Pos2::ZERO, dur: 0.6, h: 30.0, from_hang: false }, 0.15, 0.0),
        ("hop", Act::Jump { target: tg(PerchKind::Top, false), from: Pos2::ZERO, dur: 0.6, h: 30.0, from_hang: false }, 0.15, 0.0),
        ("land", Act::Land { dur: 0.2 }, 0.0, 0.0),
        ("fall", Act::Fall, 0.1, 0.0),
        ("hang", Act::Hang { dur: 3.0 }, 0.5, 0.0),
        ("shimmy", Act::Move { to: 0.0, gait: Gait::Shimmy }, 0.0, 0.25),
        ("happy", Act::Hop { n: 2 }, 0.17, 0.0),
        ("startle", Act::Startle { away: 1.0 }, 0.2, 0.0),
        ("dizzy", Act::Trip, 1.0, 0.0),
    ];
    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new("pika-sheet")));
    painter.rect_filled(screen, 0.0, hex(0xfaf3dc));
    let s = 3.0;
    let cols = 8usize;
    for (i, (name, act, t, ph)) in cases.iter().enumerate() {
        k.act = *act;
        k.t = *t;
        k.phase = *ph;
        k.blink_t = 0.0;
        k.twitch_t = 0.0;
        k.swing = if name == &"hang" { 2.5 } else { 0.0 };
        let (pose, lift) = k.pose();
        let g = rasterize(&pose);
        let cell = pos2(screen.left() + 80.0 + (i % cols) as f32 * 140.0, screen.top() + 150.0 + (i / cols) as f32 * 190.0);
        let origin = if pose.form == Form::Hang { pos2(cell.x, cell.y - 120.0 + HANG_Y * s) } else { pos2(cell.x, cell.y - lift * 1.5) };
        painter.line_segment([pos2(cell.x - 50.0, cell.y), pos2(cell.x + 50.0, cell.y)], Stroke::new(1.0_f32, Color32::from_black_alpha(40)));
        paint_grid(&painter, &g, origin, s, 1.0);
        painter.text(pos2(cell.x, cell.y + 14.0), Align2::CENTER_CENTER, *name, egui::FontId::proportional(12.0), Color32::from_black_alpha(160));
    }
}
