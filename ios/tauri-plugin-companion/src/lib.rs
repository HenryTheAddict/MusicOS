use tauri::{
    plugin::{Builder, TauriPlugin},
    Runtime,
};
#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_companion);
#[tauri::command]
async fn request<R: Runtime>(
    app: tauri::AppHandle<R>,
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    use tauri::Manager;
    #[cfg(target_os = "ios")]
    {
        let plugin = app.state::<tauri::plugin::PluginHandle<R>>();
        return plugin
            .run_mobile_plugin("request", payload)
            .map_err(|e| e.to_string());
    }
    #[cfg(not(target_os = "ios"))]
    {
        let _ = (app, payload);
        Err("The companion requires iOS".into())
    }
}
#[tauri::command]
async fn register_listener<R: Runtime>(
    app: tauri::AppHandle<R>,
    event: String,
    handler: tauri::ipc::Channel<serde_json::Value>,
) -> Result<(), String> {
    #[cfg(target_os = "ios")]
    {
        use tauri::Manager;
        let plugin = app.state::<tauri::plugin::PluginHandle<R>>();
        plugin
            .run_mobile_plugin(
                "registerListener",
                serde_json::json!({"event":event,"handler":handler}),
            )
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "ios"))]
    {
        let _ = (app, event, handler);
        Ok(())
    }
}
#[tauri::command]
async fn remove_listener<R: Runtime>(
    app: tauri::AppHandle<R>,
    event: String,
    channel_id: u32,
) -> Result<(), String> {
    #[cfg(target_os = "ios")]
    {
        use tauri::Manager;
        let plugin = app.state::<tauri::plugin::PluginHandle<R>>();
        plugin
            .run_mobile_plugin(
                "removeListener",
                serde_json::json!({"event":event,"channelId":channel_id}),
            )
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "ios"))]
    {
        let _ = (app, event, channel_id);
        Ok(())
    }
}
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("companion")
        .invoke_handler(tauri::generate_handler![
            request,
            register_listener,
            remove_listener
        ])
        .setup(|app, api| {
            #[cfg(target_os = "ios")]
            {
                use tauri::Manager;
                let handle = api.register_ios_plugin(init_plugin_companion)?;
                app.manage(handle);
            }
            #[cfg(not(target_os = "ios"))]
            let _ = (app, api);
            Ok(())
        })
        .build()
}
