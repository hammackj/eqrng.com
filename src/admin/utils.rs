#[cfg(feature = "admin")]
use axum::{extract::State, http::StatusCode, response::Html};
#[cfg(feature = "admin")]
use std::fs;
#[cfg(feature = "admin")]
use std::path::Path;

use crate::AppState;

#[cfg(feature = "admin")]
pub async fn dump_database_sql(State(_state): State<AppState>) -> Result<Html<String>, StatusCode> {
    // Get current timestamp for filename
    let now = chrono::Utc::now();
    let timestamp = now.format("%Y%m%d_%H%M%S");
    let filename = format!("data-{}.sql", timestamp);

    // Create data directory if it doesn't exist, then use it
    let data_dir = Path::new("data");
    if !data_dir.exists() {
        fs::create_dir_all(data_dir).map_err(|e| {
            eprintln!("Failed to create data directory: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }
    let file_path = format!("data/{}", filename);

    // Use SQLite's built-in .dump command for complete schema and data
    let output = std::process::Command::new("sqlite3")
        .arg("data/zones.db")
        .args(&[".mode insert", ".headers off", ".dump"])
        .output()
        .map_err(|e| {
            eprintln!("Failed to execute sqlite3 command: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        eprintln!("SQLite dump failed: {}", error);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Clone stdout before writing to file so we can use it for counting
    let stdout_clone = output.stdout.clone();

    // Write the complete dump to file
    std::fs::write(&file_path, &output.stdout).map_err(|e| {
        eprintln!("Failed to write dump file {}: {}", file_path, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Count records for display (count semicolons as a rough estimate)
    let total_records = stdout_clone.iter().filter(|&&b| b == b';').count();

    // Return success page
    let html = format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Database Export Complete</title>
            <meta charset="utf-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <script src="https://cdn.tailwindcss.com"></script>
        </head>
        <body class="bg-gray-100 min-h-screen">
            <div class="container mx-auto px-4 py-8">
                <div class="max-w-2xl mx-auto bg-white rounded-lg shadow-md p-6">
                    <div class="text-center">
                        <div class="text-green-500 text-6xl mb-4">✅</div>
                        <h1 class="text-2xl font-bold text-gray-800 mb-4">Database Export Complete</h1>
                        <p class="text-gray-600 mb-6">
                            Your database has been successfully exported to:<br>
                            <code class="bg-gray-100 px-2 py-1 rounded text-sm font-mono">{}</code>
                        </p>
                        <div class="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6">
                            <p class="text-blue-800">
                                <strong>Records exported:</strong> {} (estimated)<br>
                                <strong>File size:</strong> {:.1} KB
                            </p>
                        </div>
                        <div class="space-y-3">
                            <a href="/admin" class="inline-block bg-blue-500 hover:bg-blue-600 text-white font-semibold py-2 px-6 rounded-lg transition-colors">
                                ← Back to Admin Dashboard
                            </a>
                            <br>
                            <a href="/admin/zones" class="inline-block bg-gray-500 hover:bg-gray-600 text-white font-semibold py-2 px-6 rounded-lg transition-colors">
                                View Zones
                            </a>
                        </div>
                    </div>
                </div>
            </div>
        </body>
        </html>
        "#,
        file_path,
        total_records,
        output.stdout.len() as f64 / 1024.0
    );

    Ok(Html(html))
}
