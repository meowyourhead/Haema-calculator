//! Design tokens ported 1:1 from the HTML version's CSS custom properties.
use eframe::egui::{Color32, FontFamily, FontId};

pub const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// CSS `color-mix(in srgb, a pct%, b)`.
pub fn mix(pct: f32, a: Color32, b: Color32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 * pct + y as f32 * (1.0 - pct)).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

/// Colour with a CSS-style alpha.
pub fn rgba(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a * 255.0).round() as u8)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Palette {
    Default,
    Genshin,
    Pokemon,
}

impl Palette {
    pub fn key(self) -> &'static str {
        match self {
            Palette::Default => "default",
            Palette::Genshin => "genshin",
            Palette::Pokemon => "pokemon",
        }
    }
    pub fn from_key(s: &str) -> Palette {
        match s {
            "genshin" => Palette::Genshin,
            "pokemon" => Palette::Pokemon,
            _ => Palette::Default,
        }
    }
}

/// Which face a piece of text uses (CSS `--font-*` + weight).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Sans,
    SansB,
    Display,
    Serif,
    SerifI,
    Mono,
    MonoB,
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub palette: Palette,
    pub dark: bool,
    pub bg: Color32,
    pub surface: Color32,
    pub surface2: Color32,
    pub secondary: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub line: Color32,
    pub line_strong: Color32,
    pub accent: Color32,
    pub accent_dark: Color32,
    pub accent_soft: Color32,
    pub on_accent: Color32,
    pub ring: Color32,
    pub ok: Color32,
    pub ok_bg: Color32,
    pub bad: Color32,
    pub bad_bg: Color32,
    pub neutral: Color32,
    pub neutral_bg: Color32,
    pub warn: Color32,
    pub warn_bg: Color32,
    /// (fg, bg) for WBC, RBC, HGB, PLT
    pub param: [(Color32, Color32); 4],
    pub radius: f32,
    pub radius_lg: f32,
    pub gold: Color32,
    // Pokémon extras
    pub ink: Color32,
    pub frame: Color32,
    pub hard: Color32,
    pub check: Color32,
    pub field: Color32,
}

impl Theme {
    pub fn new(palette: Palette, dark: bool) -> Theme {
        let mut t = if dark { Self::default_dark() } else { Self::default_light() };
        t.palette = palette;
        t.dark = dark;
        match (palette, dark) {
            (Palette::Default, _) => {}
            (Palette::Genshin, false) => {
                t.bg = hex(0xdcd8cb);
                t.surface = hex(0xefe9de);
                t.surface2 = hex(0xe4ddd0);
                t.secondary = hex(0xd3cbbb);
                t.text = hex(0x3b475c);
                t.muted = hex(0x566075);
                t.line = hex(0xcfc7b6);
                t.line_strong = hex(0xa59d89);
                t.accent = hex(0x454f68);
                t.accent_dark = hex(0x3b475c);
                t.accent_soft = mix(0.42, hex(0xcbba8c), hex(0xefe9de));
                t.on_accent = hex(0xf1e7dd);
                t.ring = hex(0x5a82cc);
                t.gold = hex(0xb8975a);
                t.ok = hex(0x256b4a);
                t.ok_bg = hex(0xd9e6d8);
                t.bad = hex(0xa63530);
                t.bad_bg = hex(0xefd8cf);
                t.warn = hex(0x7a5a14);
                t.warn_bg = hex(0xeadcb6);
                t.neutral = hex(0x566075);
                t.neutral_bg = hex(0xe4ddd0);
                t.param = [
                    (hex(0xa8481e), hex(0xf1ddcb)),
                    (hex(0x7a45b0), hex(0xe6dcec)),
                    (hex(0x7a560a), hex(0xeee0b8)),
                    (hex(0x185d83), hex(0xd5e3e6)),
                ];
                t.radius = 4.0;
                t.radius_lg = 6.0;
            }
            (Palette::Genshin, true) => {
                t.bg = hex(0x252f3f);
                t.surface = hex(0x3b475c);
                t.surface2 = hex(0x333f52);
                t.secondary = hex(0x47566a);
                t.text = hex(0xe4e6dc);
                t.muted = hex(0xc4c3ba);
                t.line = hex(0x4f5d73);
                t.line_strong = hex(0x6a798d);
                t.accent = hex(0xf1e7dd);
                t.accent_dark = hex(0xcbba8c);
                t.accent_soft = mix(0.18, hex(0xcbba8c), hex(0x3b475c));
                t.on_accent = hex(0x454f68);
                t.ring = hex(0x8fb0ee);
                t.gold = hex(0xcbba8c);
                t.ok = hex(0x8fd3a8);
                t.ok_bg = hex(0x2f4d4a);
                t.bad = hex(0xffa398);
                t.bad_bg = hex(0x5a3b44);
                t.warn = hex(0xe8c77a);
                t.warn_bg = hex(0x4b4637);
                t.neutral = hex(0xc4c3ba);
                t.neutral_bg = hex(0x434e63);
                t.param = [
                    (hex(0xf5a47a), hex(0x4b4450)),
                    (hex(0xc9a6f2), hex(0x444568)),
                    (hex(0xebc66b), hex(0x4c4e55)),
                    (hex(0x7fcbef), hex(0x35506a)),
                ];
                t.radius = 4.0;
                t.radius_lg = 6.0;
            }
            (Palette::Pokemon, false) => {
                // Let's Go, Pikachu/Eevee!: sunny meadow, cream paper panels,
                // thick milky rims, teal selections and a Pokémon-red CTA.
                t.bg = hex(0xe8f2d6);
                t.surface = hex(0xfff9e9);
                t.surface2 = hex(0xf8edcb);
                t.secondary = hex(0xf3e4bc);
                t.text = hex(0x2c3b4a);
                t.muted = hex(0x5a6b78);
                t.line = hex(0xe8dab6);
                t.line_strong = hex(0xc9b98e);
                t.accent = hex(0xd63c33);
                t.accent_dark = hex(0xb92e27);
                t.accent_soft = hex(0xfce2da);
                t.on_accent = hex(0xffffff);
                t.ring = hex(0x1b8580);
                t.ok = hex(0x1f7a4c);
                t.ok_bg = hex(0xddf2d6);
                t.bad = hex(0xc93c32);
                t.bad_bg = hex(0xfbe2db);
                t.warn = hex(0x8f6500);
                t.warn_bg = hex(0xfbf0c2);
                t.neutral = hex(0x667784);
                t.neutral_bg = hex(0xf3e4bc);
                t.param = [
                    (hex(0xc4552e), hex(0xfce3d4)),
                    (hex(0x7a4fc2), hex(0xebe2f8)),
                    (hex(0x8a6a00), hex(0xfbefc2)),
                    (hex(0x1d7d9c), hex(0xd9eef4)),
                ];
                t.gold = hex(0xf2c94c);
                t.ink = hex(0x2c3b4a);
                t.frame = hex(0x2fa7a2);
                t.hard = rgba(hex(0x2c3b4a), 0.16);
                t.check = Color32::TRANSPARENT;
                t.field = hex(0xfffdf4);
                t.radius = 16.0;
                t.radius_lg = 22.0;
            }
            (Palette::Pokemon, true) => {
                // A moonlit route: the same rounded paper language in deep teal.
                t.bg = hex(0x162a31);
                t.surface = hex(0x203a42);
                t.surface2 = hex(0x1a3038);
                t.secondary = hex(0x2b4650);
                t.text = hex(0xf2f5e6);
                t.muted = hex(0xb9c9c4);
                t.line = hex(0x35525a);
                t.line_strong = hex(0x5a7a80);
                t.accent = hex(0xff6b5e);
                t.accent_dark = hex(0xff8c81);
                t.accent_soft = hex(0x4a2e30);
                t.on_accent = hex(0x2a1110);
                t.ring = hex(0x66d6cf);
                t.ok = hex(0x7fd9a8);
                t.ok_bg = hex(0x1f4034);
                t.bad = hex(0xff9a8c);
                t.bad_bg = hex(0x4a2a2c);
                t.warn = hex(0xf2ce6a);
                t.warn_bg = hex(0x3f3825);
                t.neutral = hex(0xb9c9c4);
                t.neutral_bg = hex(0x2b4650);
                t.param = [
                    (hex(0xff9e72), hex(0x43342f)),
                    (hex(0xc4aaf7), hex(0x36345a)),
                    (hex(0xecd27a), hex(0x403d2b)),
                    (hex(0x86d0f2), hex(0x274558)),
                ];
                t.gold = hex(0xf2c94c);
                t.ink = hex(0xeaf0e2);
                t.frame = hex(0x66d6cf);
                t.hard = rgba(Color32::BLACK, 0.35);
                t.check = Color32::TRANSPARENT;
                t.field = hex(0x13262c);
                t.radius = 16.0;
                t.radius_lg = 22.0;
            }
        }
        t
    }

    fn default_light() -> Theme {
        Theme {
            palette: Palette::Default,
            dark: false,
            bg: hex(0xfaf8f1),
            surface: hex(0xfcfcfc),
            surface2: hex(0xede8d9),
            secondary: hex(0xe7e4dd),
            text: hex(0x3d3826),
            muted: hex(0x85837d),
            line: hex(0xd9d8d0),
            line_strong: hex(0xb4b1a3),
            accent: hex(0xcb6441),
            accent_dark: hex(0xb2572f),
            accent_soft: mix(0.24, hex(0xf4997b), hex(0xfaf8f1)),
            on_accent: hex(0xffffff),
            ring: hex(0x1b7ede),
            ok: hex(0x1f7a54),
            ok_bg: hex(0xe6f2e9),
            bad: hex(0xc23b2e),
            bad_bg: hex(0xfbe9e5),
            neutral: hex(0x85837d),
            neutral_bg: hex(0xede8d9),
            warn: hex(0x8a5a00),
            warn_bg: hex(0xf8eed7),
            param: [
                (hex(0xb2572f), hex(0xf6e4dc)),
                (hex(0x6f58d6), hex(0xe9e5f7)),
                (hex(0x8a7a3c), hex(0xefe9d3)),
                (hex(0x4c5c82), hex(0xe3e6ee)),
            ],
            radius: 8.0,
            radius_lg: 10.0,
            gold: hex(0xb8975a),
            ink: hex(0x2f4750),
            frame: hex(0x86b69b),
            hard: Color32::TRANSPARENT,
            check: Color32::TRANSPARENT,
            field: hex(0xfffdf2),
        }
    }

    fn default_dark() -> Theme {
        let mut t = Self::default_light();
        t.bg = hex(0x262626);
        t.surface = hex(0x2e2e2c);
        t.surface2 = hex(0x1b1b1b);
        t.secondary = hex(0x3a3a36);
        t.text = hex(0xc3c1ba);
        t.muted = hex(0xb7b5a6);
        t.line = hex(0x3e3e38);
        t.line_strong = hex(0x56554d);
        t.accent = hex(0xd87757);
        t.accent_dark = hex(0xe8906f);
        t.accent_soft = mix(0.18, hex(0xd87757), hex(0x262626));
        t.on_accent = hex(0x1a1813);
        t.ok = hex(0x6fcf9a);
        t.ok_bg = hex(0x1f3329);
        t.bad = hex(0xf14444);
        t.bad_bg = hex(0x3a2220);
        t.neutral = hex(0xb7b5a6);
        t.neutral_bg = hex(0x333330);
        t.warn = hex(0xe5b45c);
        t.warn_bg = hex(0x3a3022);
        t.param = [
            (hex(0xe8906f), hex(0x3a2a24)),
            (hex(0xb3a3f8), hex(0x302b43)),
            (hex(0xd4c27a), hex(0x383423)),
            (hex(0x94a6d4), hex(0x272d3b)),
        ];
        t.gold = hex(0xcbba8c);
        t
    }

    // ---- fonts -------------------------------------------------------------

    /// Family name registered in `fonts.rs`, honouring the palette's font swaps
    /// (Genshin: one display font everywhere including body notes; Pokémon: the
    /// rounded Let's Go sans in medium and bold).
    pub fn family(&self, role: Role) -> &'static str {
        match self.palette {
            Palette::Genshin => match role {
                Role::SerifI => "serif-i",
                _ => "genshin",
            },
            Palette::Pokemon => match role {
                Role::SansB | Role::Display | Role::MonoB => "pokemon-b",
                _ => "pokemon",
            },
            Palette::Default => match role {
                Role::Sans => "sans",
                Role::SansB => "sans-b",
                Role::Display => "display",
                Role::Serif => "serif",
                Role::SerifI => "serif-i",
                Role::Mono => "mono",
                Role::MonoB => "mono-b",
            },
        }
    }

    /// Browsers synthesise bold for single-weight fonts (Genshin); Pokémon disables it.
    pub fn faux_bold(&self, role: Role) -> bool {
        self.palette == Palette::Genshin && matches!(role, Role::SansB | Role::Display | Role::MonoB)
    }

    pub fn font(&self, role: Role, size: f32) -> FontId {
        FontId::new(size, FontFamily::Name(self.family(role).into()))
    }
}
