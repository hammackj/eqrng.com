//! EverQuest RNG Migrations Library
//!
//! This library contains data migration utilities for the EQ RNG application.

pub mod migrate_zones;

pub use migrate_zones::*;

/// Re-export commonly used types from the main eq_rng crate
pub use eq_rng::{get_zones_count, setup_database};
