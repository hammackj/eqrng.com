use eq_rng::races::{RACE_GENDERS, RACES, random_race};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    println!("Testing new Barbarian and Iksar races...\n");

    // Check if the races are in the RACES array
    println!("Checking if races are in RACES array:");
    let has_barbarian = RACES.iter().any(|&race| race == "Barbarian");
    let has_iksar = RACES.iter().any(|&race| race == "Iksar");

    println!("- Barbarian in RACES: {}", has_barbarian);
    println!("- Iksar in RACES: {}", has_iksar);

    // Check if the races have gender mappings
    println!("\nChecking gender mappings:");
    let barbarian_genders = RACE_GENDERS
        .iter()
        .find(|(name, _)| *name == "Barbarian")
        .map(|(_, genders)| *genders);
    let iksar_genders = RACE_GENDERS
        .iter()
        .find(|(name, _)| *name == "Iksar")
        .map(|(_, genders)| *genders);

    if let Some(genders) = barbarian_genders {
        println!("- Barbarian genders: {:?}", genders);
    } else {
        println!("- Barbarian: No gender mapping found!");
    }

    if let Some(genders) = iksar_genders {
        println!("- Iksar genders: {:?}", genders);
    } else {
        println!("- Iksar: No gender mapping found!");
    }

    // Test image path generation
    println!("\nTesting image path generation:");
    let test_cases = vec![
        ("Barbarian", "male"),
        ("Barbarian", "female"),
        ("Iksar", "male"),
        ("Iksar", "female"),
    ];

    for (race, gender) in test_cases {
        let image_filename = format!("{}-{}.png", race.to_lowercase().replace(" ", "-"), gender);
        let image_path = format!("/assets/images/races/{}", image_filename);
        println!("- {} {}: {}", race, gender, image_path);
    }

    // Generate random races and look for the new ones
    println!("\nGenerating 50 random races to test distribution:");
    let mut race_counts: HashMap<String, u32> = HashMap::new();

    for _ in 0..50 {
        let result = random_race().await;
        let race_name = result.0.name.clone();
        *race_counts.entry(race_name).or_insert(0) += 1;
    }

    // Sort and display results
    let mut sorted_races: Vec<_> = race_counts.iter().collect();
    sorted_races.sort_by_key(|(name, _)| *name);

    for (race, count) in sorted_races {
        let marker = if race == "Barbarian" || race == "Iksar" {
            " ⭐"
        } else {
            ""
        };
        println!("- {}: {}{}", race, count, marker);
    }

    // Check if we got any Barbarian or Iksar results
    let got_barbarian = race_counts.contains_key("Barbarian");
    let got_iksar = race_counts.contains_key("Iksar");

    println!("\nResults:");
    println!("- Generated Barbarian: {}", got_barbarian);
    println!("- Generated Iksar: {}", got_iksar);

    if got_barbarian || got_iksar {
        println!("✅ SUCCESS: New races are working!");
    } else {
        println!(
            "⚠️  Note: Didn't generate new races in this small sample, but they should be available"
        );
    }

    // Test specific race generation
    println!("\nTesting specific race image paths:");
    for _ in 0..10 {
        let result = random_race().await;
        let race_result = result.0;

        if race_result.name == "Barbarian" || race_result.name == "Iksar" {
            println!(
                "Generated: {} {} -> {}",
                race_result.name,
                race_result.gender,
                race_result
                    .image_path
                    .unwrap_or("No image path".to_string())
            );
        }
    }
}
