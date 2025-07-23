use axum::Json;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::Serialize;

#[derive(Serialize)]
pub struct RaceResult {
    pub name: String,
    pub gender: String,
    pub image_path: Option<String>,
}

pub const RACES: &[&str] = &[
    "Human",
    "Barbarian",
    "Erudite",
    "Wood Elf",
    "High Elf",
    "Dark Elf",
    "Half Elf",
    "Dwarf",
    "Troll",
    "Ogre",
    "Halfling",
    "Gnome",
    "Iksar",
    "Vah Shir",
    "Drakkin",
    "Froglok",
];

// Define which races have which gender images available
pub const RACE_GENDERS: &[(&str, &[&str])] = &[
    ("Human", &["male", "female"]),
    ("Barbarian", &["male", "female"]),
    ("Erudite", &["male", "female"]),
    ("Wood Elf", &["male", "female"]),
    ("High Elf", &["male", "female"]),
    ("Dark Elf", &["male", "female"]),
    ("Half Elf", &["male", "female"]),
    ("Dwarf", &["male", "female"]),
    ("Troll", &["male", "female"]),
    ("Ogre", &["male", "female"]),
    ("Halfling", &["male", "female"]),
    ("Gnome", &["male", "female"]),
    ("Iksar", &["male", "female"]),
    ("Vah Shir", &["male", "female"]),
    ("Drakkin", &["male", "female"]),
    ("Froglok", &["male", "female"]),
];

pub async fn random_race() -> Json<RaceResult> {
    let mut rng = thread_rng();

    let race_name = RACES.choose(&mut rng).unwrap();

    let available_genders = RACE_GENDERS
        .iter()
        .find(|(name, _)| name == race_name)
        .map(|(_, genders)| *genders)
        .unwrap_or(&["male"]);

    let selected_gender = available_genders.choose(&mut rng).unwrap();

    let image_filename = format!(
        "{}-{}.png",
        race_name.to_lowercase().replace(" ", "-"),
        selected_gender
    );

    let image_path = format!("/assets/images/races/{}", image_filename);

    Json(RaceResult {
        name: race_name.to_string(),
        gender: selected_gender.to_string(),
        image_path: Some(image_path),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    #[test]
    fn test_race_result_structure() {
        let race_result = RaceResult {
            name: "Human".to_string(),
            gender: "female".to_string(),
            image_path: Some("/assets/images/races/human-female.png".to_string()),
        };

        assert_eq!(race_result.name, "Human");
        assert_eq!(race_result.gender, "female");
        assert!(race_result.image_path.is_some());
    }

    #[test]
    fn test_race_genders_mapping() {
        // Test that all races in RACES array have corresponding gender mappings
        for race in RACES {
            let has_mapping = RACE_GENDERS.iter().any(|(name, _)| name == race);
            assert!(
                has_mapping,
                "Race '{}' is missing from RACE_GENDERS mapping",
                race
            );
        }
    }

    #[test]
    fn test_random_race_generation() {
        // Test that random race generation works without panicking
        let mut rng = thread_rng();

        for _ in 0..10 {
            let race_name = RACES.choose(&mut rng).unwrap();

            let available_genders = RACE_GENDERS
                .iter()
                .find(|(name, _)| name == race_name)
                .map(|(_, genders)| *genders)
                .unwrap_or(&["male"]);

            let selected_gender = available_genders.choose(&mut rng).unwrap();

            // Verify the image path format
            let image_filename = format!(
                "{}-{}.png",
                race_name.to_lowercase().replace(" ", "-"),
                selected_gender
            );

            let image_path = format!("/assets/images/races/{}", image_filename);

            // Basic validation
            assert!(!race_name.is_empty());
            assert!(!selected_gender.is_empty());
            assert!(image_path.starts_with("/assets/images/races/"));
            assert!(image_path.ends_with(".png"));
        }
    }

    #[test]
    fn test_image_path_generation() {
        let test_cases = vec![
            ("Human", "female", "/assets/images/races/human-female.png"),
            (
                "High Elf",
                "female",
                "/assets/images/races/high-elf-female.png",
            ),
            ("Half Elf", "male", "/assets/images/races/half-elf-male.png"),
            ("Vah Shir", "male", "/assets/images/races/vah-shir-male.png"),
            (
                "Barbarian",
                "male",
                "/assets/images/races/barbarian-male.png",
            ),
            (
                "Barbarian",
                "female",
                "/assets/images/races/barbarian-female.png",
            ),
            ("Iksar", "male", "/assets/images/races/iksar-male.png"),
            ("Iksar", "female", "/assets/images/races/iksar-female.png"),
        ];

        for (race, gender, expected_path) in test_cases {
            let image_filename =
                format!("{}-{}.png", race.to_lowercase().replace(" ", "-"), gender);
            let image_path = format!("/assets/images/races/{}", image_filename);

            assert_eq!(image_path, expected_path);
        }
    }

    #[test]
    fn test_all_races_have_valid_genders() {
        for (race_name, genders) in RACE_GENDERS {
            assert!(
                !genders.is_empty(),
                "Race '{}' has no genders defined",
                race_name
            );

            for gender in *genders {
                assert!(
                    *gender == "male" || *gender == "female",
                    "Invalid gender '{}' for race '{}'",
                    gender,
                    race_name
                );
            }
        }
    }
}
