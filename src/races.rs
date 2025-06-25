use axum::Json;
use rand::seq::SliceRandom;
use rand::thread_rng;

pub const RACES: &[&str] = &[
    "Human", "Erudite", "Wood Elf", "High Elf", "Dark Elf", "Half Elf", "Dwarf", "Troll", "Ogre",
    "Halfling", "Gnome", "Iksar", "Vah Shir",
];

pub async fn random_race() -> Json<&'static str> {
    let mut rng = thread_rng();

    Json(RACES.choose(&mut rng).unwrap())
}
