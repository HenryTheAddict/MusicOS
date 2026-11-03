//! One focused-window owner. Editing commands remain owned by AppKit/WebKit.
#[cfg(target_os = "macos")]
mod mac {
    use std::cell::RefCell;
    use tauri::{Emitter, Manager};
    struct Context {
        app: tauri::AppHandle,
        label: String,
    }
    unsafe extern "C" {
        fn musicos_shortcuts_attach(
            web: *mut std::ffi::c_void,
            context: *mut std::ffi::c_void,
            dispatch: extern "C" fn(*mut std::ffi::c_void, u8),
            release: extern "C" fn(*mut std::ffi::c_void),
        ) -> *mut std::ffi::c_void;
        fn musicos_shortcuts_detach(owner: *mut std::ffi::c_void);
    }
    extern "C" fn dispatch(raw: *mut std::ffi::c_void, action: u8) {
        let context = unsafe { &*raw.cast::<Context>() };
        let action = match action {
            1 => "previous",
            2 => "next",
            3 => "visualizer",
            4 => "panel",
            _ => return,
        };
        let _ = context
            .app
            .emit_to(&context.label, "musicos-player-shortcut", action);
    }
    extern "C" fn release(raw: *mut std::ffi::c_void) {
        unsafe {
            drop(Box::from_raw(raw.cast::<Context>()));
        }
    }
    struct Guard(usize);
    impl Drop for Guard {
        fn drop(&mut self) {
            unsafe { musicos_shortcuts_detach(self.0 as *mut _) }
        }
    }
    thread_local! {static OWNER:RefCell<Option<Guard>>=const{RefCell::new(None)};}
    pub fn install(app: &tauri::AppHandle, label: &str) {
        let Some(window) = app.get_webview_window(label) else {
            return;
        };
        let context = Context {
            app: app.clone(),
            label: label.into(),
        };
        let _ = window.with_webview(move |web| {
            OWNER.with(|slot| {
                if slot.borrow().is_none() {
                    let raw = Box::into_raw(Box::new(context)).cast();
                    let owner =
                        unsafe { musicos_shortcuts_attach(web.inner(), raw, dispatch, release) };
                    if owner.is_null() {
                        release(raw);
                    } else {
                        *slot.borrow_mut() = Some(Guard(owner as usize));
                    }
                }
            })
        });
    }
    pub fn shutdown() {
        OWNER.with(|slot| drop(slot.borrow_mut().take()));
    }
}
#[cfg(target_os = "macos")]
pub use mac::{install, shutdown};
#[cfg(not(target_os = "macos"))]
pub fn install(_: &tauri::AppHandle, _: &str) {}
#[cfg(not(target_os = "macos"))]
pub fn shutdown() {}
