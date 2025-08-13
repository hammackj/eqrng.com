use axum::Json;
use serde::Serialize;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const UPDATE_VERSION: &str = "Update 7 - August 13, 2025";

#[derive(Serialize)]
pub struct VersionResponse {
    version: String,
    features: Vec<&'static str>,
}

pub async fn version() -> Json<VersionResponse> {
    let version = format!("v{} - {}", VERSION, UPDATE_VERSION);

    let mut features = Vec::new();

    #[cfg(feature = "admin")]
    features.push("admin");

    if features.is_empty() {
        features.push("production");
    }

    Json(VersionResponse { version, features })
}
