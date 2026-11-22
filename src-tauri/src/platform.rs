//! Resource paths are selected by the compiled platform, never a media/model file.
pub fn onnx_resource() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        macos_onnx_resource(std::env::consts::ARCH, !cfg!(debug_assertions))
    }
    #[cfg(target_os = "linux")]
    {
        "runtime/libonnxruntime.so.1.31.0"
    }
    #[cfg(target_os = "windows")]
    {
        "runtime/onnxruntime.dll"
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        "runtime/unavailable"
    }
}
#[cfg(any(target_os = "macos", test))]
fn macos_onnx_resource(arch: &str, release: bool) -> &'static str {
    if arch == "x86_64" && release {
        "../Frameworks/libonnxruntime.1.31.0.dylib"
    } else {
        "runtime/libonnxruntime.1.31.0.dylib"
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn runtime_path_matches_build_target() {
        let path = super::onnx_resource();
        assert!(path.starts_with("runtime/") || path.starts_with("../Frameworks/"));
        if cfg!(target_os = "linux") {
            assert!(path.ends_with(".so.1.31.0"));
        }
        if cfg!(target_os = "macos") {
            assert!(path.ends_with(".dylib"));
        }
    }

    #[test]
    fn intel_release_loads_the_signed_framework_copy() {
        assert_eq!(
            super::macos_onnx_resource("x86_64", true),
            "../Frameworks/libonnxruntime.1.31.0.dylib"
        );
        for (arch, release) in [("x86_64", false), ("aarch64", true), ("aarch64", false)] {
            assert_eq!(
                super::macos_onnx_resource(arch, release),
                "runtime/libonnxruntime.1.31.0.dylib"
            );
        }
    }
}
