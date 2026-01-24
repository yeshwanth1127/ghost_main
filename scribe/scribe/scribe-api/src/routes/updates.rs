use axum::response::{IntoResponse, Json};
use chrono::Utc;
use serde::Serialize;

#[derive(Serialize)]
struct TauriUpdateManifest {
    /// URL to the installer/bundle for the latest version.
    url: String,
    /// Latest desktop app version (must match the Tauri app's Cargo.toml/tauri.conf.json).
    version: String,
    /// Optional release notes shown in the updater UI.
    notes: String,
    /// RFC3339 publication date required by the Tauri updater.
    #[serde(rename = "pub_date")]
    pub_date: String,
}

/// Tauri updater manifest endpoint.
///
/// This is consumed by the `@tauri-apps/plugin-updater` in the Ghost desktop app.
/// Values are driven by environment variables so you can update them on each release:
/// - `DESKTOP_LATEST_VERSION`  (e.g. 0.1.7)
/// - `DESKTOP_DOWNLOAD_URL`    (direct link to installer / bundle)
/// - `DESKTOP_CHANGELOG`       (short markdown or text release notes)
pub async fn tauri_manifest() -> impl IntoResponse {
    let version =
        std::env::var("DESKTOP_LATEST_VERSION").unwrap_or_else(|_| "0.1.7".to_string());
    let url = std::env::var("DESKTOP_DOWNLOAD_URL").unwrap_or_else(|_| {
        "https://ghost.exora.solutions/downloads/ghost-latest".to_string()
    });
    let notes = std::env::var("DESKTOP_CHANGELOG")
        .unwrap_or_else(|_| "Bug fixes and improvements.".to_string());

    let manifest = TauriUpdateManifest {
        url,
        version,
        notes,
        pub_date: Utc::now().to_rfc3339(),
    };

    Json(manifest)
}




