fn main() {
    println!("cargo:rerun-if-changed=assets/jusage.ico");
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target != "windows" || !cfg!(windows) {
        return;
    }
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/jusage.ico");
    if let Err(err) = res.compile() {
        println!("cargo:warning=icon resource skipped: {err}");
    }
}
