//! EverQuest RNG Tests Library
//!
//! This library contains testing utilities and database validation tools
//! for the EQ RNG application.

pub mod test_db;

pub use test_db::{run_all_tests, test_app_database, test_database_permissions};

/// Re-export commonly used types from the main eq_rng crate
pub use eq_rng::{database_health_check, get_zones_count, setup_database};

/// Test utilities and helpers
pub mod utils {
    use std::path::Path;

    /// Check if a file exists and is readable
    pub fn file_exists_and_readable(path: &str) -> bool {
        Path::new(path).exists() && Path::new(path).is_file()
    }

    /// Get the current working directory as a string
    pub fn current_dir_string() -> Result<String, std::io::Error> {
        std::env::current_dir().map(|path| path.to_string_lossy().to_string())
    }
}
