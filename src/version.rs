use axum::Json;
use serde::Serialize;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const UPDATE_VERSION: &str = "Update 3 - July 20, 2025";

#[derive(Serialize)]
pub struct VersionResponse {
    version: &'static str,
}

pub async fn version() -> Json<VersionResponse> {
    let version = format!(
        "v{} - {}",
        String::from(VERSION),
        String::from(UPDATE_VERSION)
    );
    let static_version: &'static str = Box::leak(version.into_boxed_str());

    Json(VersionResponse {
        version: static_version,
    })
}
