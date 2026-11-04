//! Resource paths are selected by the compiled platform, never a media/model file.
pub fn onnx_resource() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "runtime/libonnxruntime.1.31.0.dylib"
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
#[cfg(test)]
mod tests {
    #[test]
    fn runtime_path_matches_build_target() {
        let path = super::onnx_resource();
        assert!(path.starts_with("runtime/"));
        if cfg!(target_os = "linux") {
            assert!(path.ends_with(".so.1.31.0"));
        }
        if cfg!(target_os = "macos") {
            assert!(path.ends_with(".dylib"));
        }
    }
}
