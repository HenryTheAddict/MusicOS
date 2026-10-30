use tauri::Emitter;
static APP: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();
unsafe extern "C" {
    fn musicos_trackpad_install(
        view: *mut std::ffi::c_void,
        callback: extern "C" fn(i32),
        remove: i32,
    );
}
extern "C" fn phase(value: i32) {
    if let Some(app) = APP.get() {
        let _ = app.emit("trackpad-phase", value);
    }
}
pub fn install(app: &tauri::AppHandle, view: *mut std::ffi::c_void) {
    let _ = APP.set(app.clone());
    unsafe {
        musicos_trackpad_install(view, phase, 0);
    }
}
pub fn shutdown() {
    unsafe {
        musicos_trackpad_install(std::ptr::null_mut(), phase, 1);
    }
}
