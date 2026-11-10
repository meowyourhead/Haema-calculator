//! Font registration. The HTML embeds Poppins, Lora, Carlito (the "Lato" slot),
//! Jersey 15 (Pokémon) and HYWenHei (Genshin); the same faces are embedded here.
use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontTweak};
use std::sync::Arc;

pub fn install(ctx: &egui::Context) {
    let mut fd = FontDefinitions::default();

    let mut add = |fd: &mut FontDefinitions, key: &str, data: FontData| {
        fd.font_data.insert(key.to_owned(), Arc::new(data));
    };

    add(&mut fd, "carlito", FontData::from_static(include_bytes!("../assets/fonts/Carlito-Regular.ttf")));
    add(&mut fd, "carlito-b", FontData::from_static(include_bytes!("../assets/fonts/Carlito-Bold.ttf")));
    add(&mut fd, "poppins", FontData::from_static(include_bytes!("../assets/fonts/Poppins-SemiBold.ttf")));
    add(&mut fd, "poppins-m", FontData::from_static(include_bytes!("../assets/fonts/Poppins-500.ttf")));
    add(&mut fd, "lora", FontData::from_static(include_bytes!("../assets/fonts/Lora-Regular.ttf")));
    add(&mut fd, "lora-i", FontData::from_static(include_bytes!("../assets/fonts/Lora-Italic.ttf")));
    // Cyberpunk: Rajdhani (condensed techno sans) and Share Tech Mono (data).
    add(&mut fd, "rajdhani-m", FontData::from_static(include_bytes!("../assets/fonts/Rajdhani-Medium.ttf")));
    add(&mut fd, "rajdhani-sb", FontData::from_static(include_bytes!("../assets/fonts/Rajdhani-SemiBold.ttf")));
    add(&mut fd, "rajdhani-b", FontData::from_static(include_bytes!("../assets/fonts/Rajdhani-Bold.ttf")));
    add(&mut fd, "sharetech-mono", FontData::from_static(include_bytes!("../assets/fonts/ShareTechMono-Regular.ttf")));

    // Jersey 15 is shown at 115 % in the HTML (size-adjust), Genshin's font at 86 %.
    let mut pixel = FontData::from_static(include_bytes!("../assets/fonts/Jersey15-Regular.ttf"));
    pixel.tweak = FontTweak { scale: 1.15, y_offset_factor: 0.0, y_offset: 0.0, baseline_offset_factor: 0.0 };
    add(&mut fd, "pixel", pixel);
    let mut genshin = FontData::from_static(include_bytes!("../assets/fonts/Genshin-Subset.ttf"));
    genshin.tweak = FontTweak { scale: 0.86, y_offset_factor: 0.0, y_offset: 0.0, baseline_offset_factor: 0.0 };
    add(&mut fd, "genshin", genshin);

    // Monospace: the HTML falls back to Consolas on Windows; use it when present.
    let (mono_key, mono_b_key) = load_consolas(&mut fd);

    // Glyph fallbacks (superscripts, arrows, …) come after the primary face.
    let tail: Vec<String> = ["carlito", "Ubuntu-Light", "NotoEmoji-Regular", "emoji-icon-font"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let fam = |primary: &str| {
        let mut v = vec![primary.to_string()];
        v.extend(tail.iter().filter(|t| t.as_str() != primary).cloned());
        v
    };

    let named = |n: &str| FontFamily::Name(n.into());
    fd.families.insert(named("sans"), fam("carlito"));
    fd.families.insert(named("sans-b"), fam("carlito-b"));
    fd.families.insert(named("display"), fam("poppins"));
    fd.families.insert(named("serif"), fam("lora"));
    fd.families.insert(named("serif-i"), fam("lora-i"));
    fd.families.insert(named("pixel"), fam("pixel"));
    fd.families.insert(named("genshin"), fam("genshin"));
    fd.families.insert(named("pokemon"), fam("poppins-m"));
    fd.families.insert(named("pokemon-b"), fam("poppins"));
    fd.families.insert(named("cyber"), fam("rajdhani-m"));
    fd.families.insert(named("cyber-sb"), fam("rajdhani-sb"));
    fd.families.insert(named("cyber-b"), fam("rajdhani-b"));
    fd.families.insert(named("cyber-mono"), fam("sharetech-mono"));
    let mut mono = vec![mono_key];
    mono.push("Hack".into());
    mono.extend(tail.iter().cloned());
    fd.families.insert(named("mono"), mono);
    let mut mono_b = vec![mono_b_key];
    mono_b.push("Hack".into());
    mono_b.extend(tail.iter().cloned());
    fd.families.insert(named("mono-b"), mono_b);

    // Anything using the plain egui families (e.g. TextEdit defaults) gets Carlito too.
    fd.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "carlito".into());

    ctx.set_fonts(fd);
}

fn load_consolas(fd: &mut FontDefinitions) -> (String, String) {
    let dirs = ["C:\\Windows\\Fonts", "/usr/share/fonts/truetype/consolas"];
    let read = |name: &str| -> Option<Vec<u8>> {
        dirs.iter().find_map(|d| std::fs::read(std::path::Path::new(d).join(name)).ok())
    };
    match read("consola.ttf") {
        Some(reg) => {
            fd.font_data.insert("consolas".into(), Arc::new(FontData::from_owned(reg)));
            let bold = match read("consolab.ttf") {
                Some(b) => {
                    fd.font_data.insert("consolas-b".into(), Arc::new(FontData::from_owned(b)));
                    "consolas-b"
                }
                None => "consolas",
            };
            ("consolas".into(), bold.into())
        }
        None => ("Hack".into(), "Hack".into()),
    }
}
