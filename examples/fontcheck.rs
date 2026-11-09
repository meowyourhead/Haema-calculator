fn main() {
    for entry in std::fs::read_dir("assets/fonts").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ttf") {
            continue;
        }
        let data = std::fs::read(&path).unwrap();
        match ab_glyph::FontVec::try_from_vec(data) {
            Ok(font) => {
                use ab_glyph::Font;
                println!("OK   {:?} glyphs={} upem={}", path.file_name().unwrap(), font.glyph_count(), font.units_per_em().unwrap_or(0.0));
            }
            Err(e) => println!("FAIL {:?}: {e:?}", path.file_name().unwrap()),
        }
    }
}
