use config::{Config, Environment, File};
use serde::Deserialize;
use std::env;
use tracing::{info, warn};

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub host: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub path: String,
    pub backup_dir: String,
    pub migrate_on_startup: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecurityConfig {
    pub rating_ip_hash_key: String,
    pub min_ip_hash_key_length: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RatingsConfig {
    pub min_rating: u8,
    pub max_rating: u8,
    pub transaction_log_path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminConfig {
    pub enabled: bool,
    pub page_size: i32,
    pub min_page_size: i32,
    pub max_page_size: i32,
    pub default_sort_column: String,
    pub default_sort_order: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CorsConfig {
    pub development_origins: Vec<String>,
    pub production_origins: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub file_path: String,
    pub max_file_size: String,
    pub max_files: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub security: SecurityConfig,
    pub ratings: RatingsConfig,
    pub admin: AdminConfig,
    pub cors: CorsConfig,
    pub logging: LoggingConfig,
}

impl AppConfig {
    pub fn load() -> Result<Self, config::ConfigError> {
        let environment = env::var("EQ_RNG_ENV").unwrap_or_else(|_| "development".to_string());

        info!("Loading configuration for environment: {}", environment);

        let config = Config::builder()
            // Start with default config
            .add_source(File::with_name("config/default"))
            // Add environment-specific config
            .add_source(File::with_name(&format!("config/{}", environment)).required(false))
            // Add environment variables with prefix EQ_RNG_
            .add_source(Environment::with_prefix("EQ_RNG").separator("_"))
            .build()?;

        let app_config: AppConfig = config.try_deserialize()?;

        // Validate configuration
        app_config.validate()?;

        info!("Configuration loaded successfully");
        Ok(app_config)
    }

    fn validate(&self) -> Result<(), config::ConfigError> {
        // Validate security settings
        if self.security.rating_ip_hash_key.len() < self.security.min_ip_hash_key_length {
            return Err(config::ConfigError::Message(format!(
                "security.rating_ip_hash_key must be at least {} characters long",
                self.security.min_ip_hash_key_length
            )));
        }

        // Validate rating range
        if self.ratings.min_rating >= self.ratings.max_rating {
            warn!(
                "Invalid rating range: min_rating ({}) >= max_rating ({})",
                self.ratings.min_rating, self.ratings.max_rating
            );
        }

        // Validate admin settings
        if self.admin.min_page_size >= self.admin.max_page_size {
            warn!(
                "Invalid admin pagination: min_page_size ({}) >= max_page_size ({})",
                self.admin.min_page_size, self.admin.max_page_size
            );
        }

        if self.admin.page_size < self.admin.min_page_size
            || self.admin.page_size > self.admin.max_page_size
        {
            warn!(
                "Invalid admin page_size: {} is not between {} and {}",
                self.admin.page_size, self.admin.min_page_size, self.admin.max_page_size
            );
        }

        Ok(())
    }

    pub fn is_development(&self) -> bool {
        env::var("EQ_RNG_ENV").unwrap_or_else(|_| "development".to_string()) == "development"
    }

    pub fn is_production(&self) -> bool {
        env::var("EQ_RNG_ENV").unwrap_or_else(|_| "development".to_string()) == "production"
    }

    pub fn get_cors_origins(&self) -> Vec<String> {
        if self.is_development() {
            self.cors.development_origins.clone()
        } else {
            self.cors.production_origins.clone()
        }
    }
}
