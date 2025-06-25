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
    let file_contents =
        fs::read_to_string("data/class_race.json").expect("Failed to read classes.json");
    let class_race_json =
        serde_json::from_str(&file_contents).expect("Failed to parse classes.json");

    let shared_class_race = Arc::new(class_race_json);

    return shared_class_race;
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

    let class_name = CLASSES.choose(&mut rng).unwrap().to_string();

    Json(Some(class_name))
}
