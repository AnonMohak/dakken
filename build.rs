// Puts the dakken icon into dakken.exe on Windows. Other systems have no
// icon in the binary, so this does nothing there.
fn main() {
    println!("cargo:rerun-if-changed=assets/brand/dakken.ico");
    // The target triple names the release file a self-update downloads.
    let target = std::env::var("TARGET").expect("cargo sets TARGET");
    println!("cargo:rustc-env=DAKKEN_TARGET={target}");
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/brand/dakken.ico");
        res.compile().expect("could not embed the Windows icon");
    }
}
