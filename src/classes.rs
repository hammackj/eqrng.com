use axum::extract::FromRef;
use axum::{
    Json,
    extract::{Query, State},
};
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::Deserialize;
use std::collections::HashMap;
use std::{fs, sync::Arc};

#[derive(Clone)]
pub struct ClassRaceState {
    pub class_race_map: Arc<RaceClassMap>,
}

impl FromRef<crate::AppState> for ClassRaceState {
    fn from_ref(app_state: &crate::AppState) -> ClassRaceState {
        app_state.class_race_state.clone()
    }
}

pub type RaceClassMap = HashMap<String, Vec<String>>;

pub const CLASSES: &[&str] = &[
    "Warrior",
    "Cleric",
    "Paladin",
    "Ranger",
    "Shadow Knight",
    "Druid",
    "Monk",
    "Bard",
    "Rogue",
    "Shaman",
    "Necromancer",
    "Wizard",
    "Magician",
    "Enchanter",
    "Beastlord",
    "Berserker",
];

#[derive(Deserialize)]
pub struct ClassQuery {
    race: Option<String>,
}

pub fn load_classes() -> Arc<RaceClassMap> {
    let file_contents = fs::read_to_string("data/class_race.json").unwrap_or_else(|e| {
        tracing::error!(error = %e, "Failed to read classes.json");
        // Return empty map as fallback
        "{}".to_string()
    });

    let class_race_json = serde_json::from_str(&file_contents).unwrap_or_else(|e| {
        tracing::error!(error = %e, "Failed to parse classes.json");
        // Return empty map as fallback
        HashMap::new()
    });

    Arc::new(class_race_json)
}

pub async fn random_class(
    Query(query): Query<ClassQuery>,
    State(state): State<crate::AppState>,
) -> Json<Option<String>> {
    let mut rng = thread_rng();

    let map = state.class_race_state.class_race_map.clone();

    if let Some(race) = query.race {
        if let Some(classes) = map.get(&race) {
            return Json(classes.choose(&mut rng).cloned());
        } else {
            return Json(None);
        }
    }

    // Safe to unwrap here since CLASSES is a constant array that's never empty
    let class_name = CLASSES
        .choose(&mut rng)
        .expect("CLASSES array should never be empty")
        .to_string();

    Json(Some(class_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::{Query, State};
    use sqlx::sqlite::SqlitePoolOptions;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    use crate::{
        config::{
            AdminConfig, AppConfig, CorsConfig, DatabaseConfig, LoggingConfig, RatingsConfig,
            SecurityConfig, ServerConfig,
        },
        instances, zones, AppState,
    };

    static DIR_LOCK: Mutex<()> = Mutex::new(());

    fn test_app_config() -> AppConfig {
        AppConfig {
            server: ServerConfig {
                port: 0,
                host: "localhost".into(),
            },
            database: DatabaseConfig {
                path: String::new(),
                backup_dir: String::new(),
                migrate_on_startup: false,
            },
            security: SecurityConfig {
                rating_ip_hash_key: "test".into(),
                min_ip_hash_key_length: 0,
            },
            ratings: RatingsConfig {
                min_rating: 0,
                max_rating: 10,
                transaction_log_path: String::new(),
            },
            admin: AdminConfig {
                enabled: false,
                page_size: 0,
                min_page_size: 0,
                max_page_size: 1,
                default_sort_column: String::new(),
                default_sort_order: String::new(),
            },
            cors: CorsConfig {
                development_origins: vec![],
                production_origins: vec![],
            },
            logging: LoggingConfig {
                level: String::new(),
                format: String::new(),
                file_path: String::new(),
                max_file_size: String::new(),
                max_files: 0,
            },
        }
    }

    fn test_app_state(class_race_state: ClassRaceState) -> AppState {
        let pool = SqlitePoolOptions::new()
            .connect_lazy("sqlite::memory:")
            .expect("create pool");
        let pool = Arc::new(pool);

        AppState {
            config: Arc::new(test_app_config()),
            zone_state: zones::ZoneState { pool: pool.clone() },
            instance_state: instances::InstanceState { pool: pool.clone() },
            class_race_state,
        }
    }

    #[test]
    fn load_classes_missing_file() {
        let _guard = DIR_LOCK.lock().unwrap();
        let orig_dir = std::env::current_dir().unwrap();
        let tmp = tempdir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();

        let classes = load_classes();
        assert!(classes.is_empty());

        std::env::set_current_dir(orig_dir).unwrap();
    }

    #[test]
    fn load_classes_invalid_json() {
        let _guard = DIR_LOCK.lock().unwrap();
        let orig_dir = std::env::current_dir().unwrap();
        let tmp = tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("data")).unwrap();
        std::fs::write(tmp.path().join("data/class_race.json"), "{invalid").unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();

        let classes = load_classes();
        assert!(classes.is_empty());

        std::env::set_current_dir(orig_dir).unwrap();
    }

    #[tokio::test]
    async fn random_class_with_race() {
        let map = HashMap::from([(
            "Human".to_string(),
            vec!["Warrior".to_string()],
        )]);
        let state = test_app_state(ClassRaceState {
            class_race_map: Arc::new(map),
        });

        let result =
            random_class(Query(ClassQuery { race: Some("Human".into()) }), State(state)).await;
        assert_eq!(result.0, Some("Warrior".to_string()));
    }

    #[tokio::test]
    async fn random_class_without_race() {
        let state = test_app_state(ClassRaceState {
            class_race_map: Arc::new(HashMap::new()),
        });

        for _ in 0..10 {
            let result = random_class(Query(ClassQuery { race: None }), State(state.clone())).await;
            let class = result.0.expect("expected a class");
            assert!(CLASSES.contains(&class.as_str()));
        }
    }
}
