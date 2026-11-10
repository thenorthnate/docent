/// The app version, as built. Shown in the UI so a bug report can name it.
#[tauri::command]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_version])
        .run(tauri::generate_context!())
        .expect("Tauri failed to start the app; nothing can run without it");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_version_matches_the_bundle_version() {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json"))
            .expect("tauri.conf.json is valid JSON");

        assert_eq!(Some(app_version()), conf["version"].as_str());
    }
}
