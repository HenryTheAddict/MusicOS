fn main() {
    println!("cargo:rerun-if-changed=src/local/apple_media.m");
    println!("cargo:rerun-if-changed=src/local/audio_route.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new().file("src/local/apple_media.m").file("src/local/audio_route.m").flag("-fobjc-arc").flag("-Wno-deprecated-declarations").compile("musicos_apple_media");
        for framework in ["CoreAudio", "Foundation", "AVFoundation", "CoreMedia", "AudioToolbox", "CoreML", "CoreVideo", "CoreGraphics", "ImageIO", "WebKit"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
    }
}
