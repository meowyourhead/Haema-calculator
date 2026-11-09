fn main() {
    println!("cargo:rerun-if-changed=assets/icon/fox.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icon/fox.ico");
    res.set("ProductName", "Haematology Calculator");
    res.set("FileDescription", "Haematology Calculator");
    res.set("OriginalFilename", "haematology_calculator.exe");
    if let Err(err) = res.compile() {
        // Keep the build usable on machines without the Windows SDK resource compiler.
        println!("cargo:warning=could not embed the application icon: {err}");
    }
}
