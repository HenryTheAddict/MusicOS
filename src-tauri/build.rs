fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new().file("src/artwork_sdr.m").flag("-fobjc-arc").compile("musicos_display_sdr");
        for framework in ["Foundation", "CoreGraphics", "ImageIO"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
        println!("cargo:rerun-if-changed=src/artwork_sdr.m");
    }
    tauri_build::build()
}
