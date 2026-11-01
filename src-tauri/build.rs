fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new().file("src/trackpad_phase.m").file("src/artwork_sdr.m").file("src/haptics.m").flag("-fobjc-arc").compile("musicos_display_sdr");
        for framework in ["AppKit", "Foundation", "CoreGraphics", "ImageIO"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
        println!("cargo:rerun-if-changed=src/artwork_sdr.m");
        println!("cargo:rerun-if-changed=src/haptics.m");
        println!("cargo:rerun-if-changed=src/trackpad_phase.m");
    }
    tauri_build::build()
}
