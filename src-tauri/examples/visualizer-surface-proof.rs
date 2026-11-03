//! Isolated native integration fixture: no AppState, profile, library or audio.
use std::sync::{Arc, Mutex};
use tauri::Manager;
const HTML: &str = r#"<!doctype html><meta charset='utf-8'><html style='background:transparent'><body style='background:transparent;color:white;font:20px system-ui;margin:60px'><h1>musicOS native GPU surface check</h1><p>Original boxes preset, Rust / Metal, behind transparent web content. Synthetic test features; no audio output.</p><button style='padding:16px' onclick='this.textContent="Overlay interaction passed"'>Check overlay interaction</button><details style='margin:24px 0'><summary>Menu layering</summary><p style='background:#263449;padding:24px'>Opaque control over the native GPU surface</p></details><p style='margin-top:140px'>Synchronized lyrics overlay — native surface proof only</p></body></html>"#;
struct ProofState(Arc<Mutex<Option<musicos_tauri::visualizer::surface::Session>>>);
#[tauri::command]
async fn visualizer(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    request: musicos_tauri::visualizer::Request,
) -> Result<musicos_tauri::visualizer::Status, String> {
    use musicos_tauri::visualizer::{surface::Session, Status};
    if request.action == "close" {
        drop(app.state::<ProofState>().0.lock().unwrap().take());
        return Ok(Status::default());
    }
    if request.action == "status" {
        return Ok(app
            .state::<ProofState>()
            .0
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| s.status())
            .unwrap_or_default());
    }
    let settings = request.settings.unwrap_or_default();
    let status = Status {
        phase: "running".into(),
        preset: settings.preferences.preset.clone(),
        request_id: settings.request_id,
        ..Default::default()
    };
    if let Some(session) = app.state::<ProofState>().0.lock().unwrap().as_ref() {
        session.update(settings);
        return Ok(status);
    }
    let (send, receive) = tokio::sync::oneshot::channel();
    window
        .with_webview(move |web| {
            let result = unsafe { Session::proof(web.inner()) }
                .map(|session| {
                    session.update(settings);
                    *app.state::<ProofState>().0.lock().unwrap() = Some(session);
                    status
                })
                .map_err(|e| e.to_string());
            let _ = send.send(result);
        })
        .map_err(|e| e.to_string())?;
    receive.await.map_err(|e| e.to_string())?
}
fn main() {
    let session = Arc::new(Mutex::new(None));
    let cleanup = session.clone();
    let mut context = tauri::generate_context!();
    context.config_mut().identifier = "com.musicos.visualizer-proof".into();
    let ui = std::env::var_os("MUSICOS_VISUALIZER_UI_PROOF").is_some();
    if ui {
        context.config_mut().build.dev_url = Some("http://127.0.0.1:1422".parse().unwrap());
    }
    tauri::Builder::default()
        .manage(ProofState(session.clone()))
        .invoke_handler(tauri::generate_handler![visualizer])
        .register_uri_scheme_protocol("visualizer-proof", |_, _| {
            tauri::http::Response::builder()
                .header("Content-Type", "text/html")
                .body(HTML.as_bytes().to_vec())
                .unwrap()
        })
        .setup(move |app| {
            if ui { app.add_capability(r#"{"identifier":"visualizer-proof-only","windows":["proof"],"remote":{"urls":["http://127.0.0.1:1422/*"]},"permissions":["core:default","core:window:allow-is-fullscreen","core:window:allow-set-fullscreen"]}"#)?; }
            let window = tauri::WebviewWindowBuilder::new(
                app,
                "proof",
                tauri::WebviewUrl::External(if ui{"http://127.0.0.1:1422/visualizer-proof.html?lyrics=timed"}else{"visualizer-proof://localhost/index.html"}.parse()?),
            )
            .title_bar_style(tauri::TitleBarStyle::Overlay)
            .hidden_title(true)
            .title("musicOS GPU surface check")
            .inner_size(900., 650.)
            .build()?;
            if ui { musicos_tauri::player_shortcuts::install(app.handle(),"proof");return Ok(()); }
            let target = session.clone();
            window.with_webview(move |w| {
                match unsafe { musicos_tauri::visualizer::surface::Session::proof(w.inner()) } {
                    Ok(s) => *target.lock().unwrap() = Some(s),
                    Err(e) => eprintln!("Native surface proof failed: {e:#}"),
                }
            })?;
            Ok(())
        })
        .build(context)
        .expect("native proof fixture")
        .run(move |app, event| {
            if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
                musicos_tauri::player_shortcuts::shutdown();
                if let (Some(s), Some(w)) = (
                    cleanup.lock().unwrap().take(),
                    app.get_webview_window("proof"),
                ) {
                    musicos_tauri::visualizer::surface::close(&w, s);
                }
            }
        });
}
