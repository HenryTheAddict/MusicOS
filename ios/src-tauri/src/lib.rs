mod decoder;
#[cfg_attr(mobile,tauri::mobile_entry_point)]
pub fn run(){
 tauri::Builder::default().plugin(tauri_plugin_companion::init())
  .run(tauri::generate_context!()).expect("musicOS companion could not start");
}
